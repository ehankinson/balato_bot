import random
from dataclasses import dataclass
from multiprocessing import get_context
from multiprocessing.connection import Connection
from time import perf_counter

import torch

from calculation.poker_discards import generate_discard_table
from calculation.poker_eval import find_best_hand_type
from calculation.score import get_best_scoring_hand
from core.enums import HandAction
from core.models import Card, Deck, GameState
from simulation.blind_env import ActionMasks
from simulation.blind_trainer import BlindModel
from simulation.decoder import build_mask, model_decoder_batch
from simulation.encoder import encode_game_state
from simulation.reward import calculate_score_progress_reward, calculate_terminal_reward


@dataclass(slots=True)
class GameSlot:
    deck: Deck
    game_state: GameState
    hand: list[Card]
    steps: int = 0
    active: bool = True


@dataclass(slots=True)
class SharedRolloutBuffers:
    """Fixed shared-memory storage indexed by [round, episode]."""

    observations: torch.Tensor
    mode_masks: torch.Tensor
    count_masks: torch.Tensor
    card_masks: torch.Tensor
    valid: torch.Tensor
    modes: torch.Tensor
    counts: torch.Tensor
    cards: torch.Tensor
    card_valid: torch.Tensor
    played_hand_types: torch.Tensor
    old_log_probs: torch.Tensor
    old_values: torch.Tensor
    rewards: torch.Tensor
    won: torch.Tensor
    final_scores: torch.Tensor
    final_targets: torch.Tensor
    hands_used: torch.Tensor
    score_seconds: torch.Tensor
    discard_seconds: torch.Tensor
    encode_seconds: torch.Tensor
    env_seconds: torch.Tensor

    @classmethod
    def create(
        cls,
        rollout_rounds: int,
        episode_count: int,
        input_size: int,
    ) -> "SharedRolloutBuffers":
        def shared(*shape: int, dtype: torch.dtype) -> torch.Tensor:
            return torch.zeros(shape, dtype=dtype).share_memory_()

        prefix = (rollout_rounds, episode_count)
        return cls(
            observations=shared(*prefix, input_size, dtype=torch.float32),
            mode_masks=shared(*prefix, 2, dtype=torch.float32),
            count_masks=shared(*prefix, 5, dtype=torch.float32),
            card_masks=shared(*prefix, 8, dtype=torch.float32),
            valid=shared(*prefix, dtype=torch.bool),
            modes=shared(*prefix, dtype=torch.long),
            counts=shared(*prefix, dtype=torch.long),
            cards=shared(*prefix, 5, dtype=torch.long),
            card_valid=shared(*prefix, 5, dtype=torch.float32),
            played_hand_types=shared(*prefix, dtype=torch.long),
            old_log_probs=shared(*prefix, dtype=torch.float32),
            old_values=shared(*prefix, dtype=torch.float32),
            rewards=shared(*prefix, dtype=torch.float32),
            won=shared(*prefix, dtype=torch.bool),
            final_scores=shared(*prefix, dtype=torch.float64),
            final_targets=shared(*prefix, dtype=torch.float64),
            hands_used=shared(*prefix, dtype=torch.long),
            score_seconds=shared(*prefix, dtype=torch.float64),
            discard_seconds=shared(*prefix, dtype=torch.float64),
            encode_seconds=shared(*prefix, dtype=torch.float64),
            env_seconds=shared(*prefix, dtype=torch.float64),
        )

    def clear(self) -> None:
        self.valid.zero_()
        self.rewards.zero_()
        self.won.zero_()
        self.played_hand_types.fill_(-1)
        self.final_scores.zero_()
        self.final_targets.zero_()
        self.hands_used.zero_()
        self.score_seconds.zero_()
        self.discard_seconds.zero_()
        self.encode_seconds.zero_()
        self.env_seconds.zero_()


@dataclass(frozen=True, slots=True)
class RolloutTimings:
    rollout_seconds: float
    inference_seconds: float
    inference_batches: int


@dataclass(slots=True)
class RolloutBatch:
    observations: torch.Tensor
    modes: torch.Tensor
    counts: torch.Tensor
    cards: torch.Tensor
    card_valid: torch.Tensor
    played_hand_types: torch.Tensor
    old_log_probs: torch.Tensor
    old_values: torch.Tensor
    rewards: torch.Tensor
    mode_masks: torch.Tensor
    count_masks: torch.Tensor
    card_masks: torch.Tensor
    episode_ids: torch.Tensor
    terminal_scores: torch.Tensor
    terminal_targets: torch.Tensor
    terminal_wins: torch.Tensor
    terminal_hands_used: torch.Tensor
    score_seconds: float
    discard_seconds: float
    encoding_seconds: float
    environment_seconds: float

    @classmethod
    def from_buffers(
        cls,
        buffers: SharedRolloutBuffers,
        episode_count: int,
        max_steps: int,
    ) -> "RolloutBatch":
        valid = buffers.valid
        episode_grid = torch.arange(episode_count).expand(max_steps, -1)
        terminal_mask = buffers.final_targets > 0
        return cls(
            observations=buffers.observations[valid],
            modes=buffers.modes[valid],
            counts=buffers.counts[valid],
            cards=buffers.cards[valid],
            card_valid=buffers.card_valid[valid],
            played_hand_types=buffers.played_hand_types[valid],
            old_log_probs=buffers.old_log_probs[valid],
            old_values=buffers.old_values[valid],
            rewards=buffers.rewards[valid],
            mode_masks=buffers.mode_masks[valid],
            count_masks=buffers.count_masks[valid],
            card_masks=buffers.card_masks[valid],
            episode_ids=episode_grid[valid],
            terminal_scores=buffers.final_scores[terminal_mask],
            terminal_targets=buffers.final_targets[terminal_mask],
            terminal_wins=buffers.won[terminal_mask],
            terminal_hands_used=buffers.hands_used[terminal_mask],
            score_seconds=float(buffers.score_seconds[valid].sum()),
            discard_seconds=float(buffers.discard_seconds[valid].sum()),
            encoding_seconds=float(buffers.encode_seconds[valid].sum()),
            environment_seconds=float(buffers.env_seconds[valid].sum()),
        )

    @property
    def total_steps(self) -> int:
        return len(self.rewards)

    @property
    def completed_episodes(self) -> int:
        return len(self.terminal_scores)


def worker_distribution(
    episode_count: int,
    worker_count: int,
) -> tuple[list[int], list[int]]:
    base, remainder = divmod(episode_count, worker_count)
    counts = [base + int(worker < remainder) for worker in range(worker_count)]
    offsets: list[int] = []
    offset = 0
    for count in counts:
        offsets.append(offset)
        offset += count
    return counts, offsets


def _write_observations(
    slots: list[GameSlot],
    buffers: SharedRolloutBuffers,
    round_index: int,
    episode_offset: int,
    profile: bool,
) -> int:
    active_count = 0
    with torch.inference_mode():
        for local_episode, slot in enumerate(slots):
            if not slot.active:
                continue

            episode = episode_offset + local_episode
            started = perf_counter()
            best_hand = get_best_scoring_hand(slot.hand, [], slot.game_state)
            scored = perf_counter()
            discard_table = generate_discard_table(slot.deck, slot.hand)
            discarded = perf_counter()
            observation = encode_game_state(
                slot.hand,
                slot.game_state,
                best_hand,
                discard_table,
            )
            masks = build_mask(slot.game_state, slot.hand, torch.device("cpu"))
            encoded = perf_counter()

            if profile:
                buffers.score_seconds[round_index, episode] = scored - started
                buffers.discard_seconds[round_index, episode] = discarded - scored
                buffers.encode_seconds[round_index, episode] = encoded - discarded

            buffers.observations[round_index, episode].copy_(observation)
            buffers.mode_masks[round_index, episode].copy_(masks.mode)
            buffers.count_masks[round_index, episode].copy_(masks.count)
            buffers.card_masks[round_index, episode].copy_(masks.card)
            buffers.valid[round_index, episode] = True
            active_count += 1

    return active_count


def _create_game_slots(
    episode_count: int,
    episode_offset: int,
    batch_seed: int,
    score_to_beat: int,
) -> list[GameSlot]:
    slots: list[GameSlot] = []
    for local_episode in range(episode_count):
        random.seed(batch_seed + episode_offset + local_episode)
        deck = Deck()
        game_state = GameState(score_to_beat=score_to_beat)
        hand = deck.draw(game_state.hand_size)
        slots.append(GameSlot(deck=deck, game_state=game_state, hand=hand))
    return slots


def _apply_shared_actions(
    slots: list[GameSlot],
    buffers: SharedRolloutBuffers,
    round_index: int,
    episode_offset: int,
    max_steps: int,
    profile: bool,
) -> bool:
    for local_episode, slot in enumerate(slots):
        if not slot.active:
            continue

        episode = episode_offset + local_episode
        started = perf_counter()
        mode = HandAction(int(buffers.modes[round_index, episode]))
        count = int(buffers.counts[round_index, episode])
        card_indices = [
            int(index) for index in buffers.cards[round_index, episode, :count]
        ]
        selected_cards = [slot.hand[index] for index in card_indices]
        for selected_card in selected_cards:
            slot.hand.remove(selected_card)

        score_before = slot.game_state.current_score
        has_won = slot.game_state.execute_hand_action(
            mode,
            selected_cards,
            slot.hand,
            slot.deck,
        )
        slot.steps += 1

        if mode == HandAction.PLAY_HAND:
            hand_stats, _scored_cards = find_best_hand_type(selected_cards)
            buffers.played_hand_types[round_index, episode] = int(hand_stats.name)
            buffers.rewards[round_index, episode] += calculate_score_progress_reward(
                score_before,
                slot.game_state,
            )

        is_terminal = has_won or slot.game_state.hands == 0 or slot.steps >= max_steps
        if is_terminal:
            buffers.rewards[round_index, episode] += calculate_terminal_reward(
                slot.game_state
            )
            slot.active = False
            buffers.won[round_index, episode] = has_won
            buffers.final_scores[round_index, episode] = (
                slot.game_state.current_score
            )
            buffers.final_targets[round_index, episode] = (
                slot.game_state.score_to_beat
            )
            buffers.hands_used[round_index, episode] = (
                slot.game_state.hands_played
            )

        if profile:
            buffers.env_seconds[round_index, episode] = perf_counter() - started

    return any(slot.active for slot in slots)


def rollout_actor(
    connection: Connection,
    buffers: SharedRolloutBuffers,
    episode_count: int,
    episode_offset: int,
    score_to_beat: int,
    max_steps: int,
    profile: bool,
) -> None:
    """Own a group of games while the main process performs GPU inference."""
    torch.set_num_threads(1)
    try:
        while True:
            command, payload = connection.recv()
            if command == "close":
                return
            if command != "start":
                raise RuntimeError(f"unexpected actor command: {command}")

            slots = _create_game_slots(
                episode_count,
                episode_offset,
                int(payload),
                score_to_beat,
            )
            for round_index in range(max_steps):
                active_count = _write_observations(
                    slots,
                    buffers,
                    round_index,
                    episode_offset,
                    profile,
                )
                connection.send(("observations_ready", active_count))

                command, _action_payload = connection.recv()
                if command == "close":
                    return
                if command != "actions_ready":
                    raise RuntimeError(f"unexpected actor command: {command}")

                still_active = _apply_shared_actions(
                    slots,
                    buffers,
                    round_index,
                    episode_offset,
                    max_steps,
                    profile,
                )
                connection.send(("results_ready", still_active))
                if not still_active:
                    break
    finally:
        connection.close()


class RolloutPool:
    def __init__(
        self,
        episode_count: int,
        worker_count: int,
        worker_targets: list[int],
        max_steps: int,
        input_size: int,
        profile: bool,
    ) -> None:
        self.episode_count = episode_count
        self.worker_count = worker_count
        self.max_steps = max_steps
        self.profile = profile
        self.buffers = SharedRolloutBuffers.create(
            max_steps,
            episode_count,
            input_size,
        )
        worker_counts, worker_offsets = worker_distribution(
            episode_count,
            worker_count,
        )
        context = get_context("spawn")
        self.connections: list[Connection] = []
        self.processes = []
        for worker in range(worker_count):
            parent_connection, child_connection = context.Pipe()
            process = context.Process(
                target=rollout_actor,
                args=(
                    child_connection,
                    self.buffers,
                    worker_counts[worker],
                    worker_offsets[worker],
                    worker_targets[worker],
                    max_steps,
                    profile,
                ),
            )
            process.start()
            child_connection.close()
            self.connections.append(parent_connection)
            self.processes.append(process)

    def collect(
        self,
        model: BlindModel,
        device: torch.device,
        batch_seed: int,
    ) -> tuple[RolloutBatch, RolloutTimings]:
        started = perf_counter()
        self.buffers.clear()
        for connection in self.connections:
            connection.send(("start", batch_seed))

        active_workers = list(range(self.worker_count))
        inference_batches = 0
        inference_seconds = 0.0
        round_index = 0
        while active_workers:
            for worker in active_workers:
                command, _active_count = self.connections[worker].recv()
                if command != "observations_ready":
                    raise RuntimeError(f"unexpected actor response: {command}")

            inference_started = perf_counter()
            active_mask = self.buffers.valid[round_index]
            observations = self.buffers.observations[round_index, active_mask].to(
                device
            )
            masks = ActionMasks(
                mode=self.buffers.mode_masks[round_index, active_mask].to(device),
                count=self.buffers.count_masks[round_index, active_mask].to(device),
                card=self.buffers.card_masks[round_index, active_mask].to(device),
            )
            model.eval()
            with torch.inference_mode():
                outputs = model(observations)
                (
                    modes,
                    counts,
                    cards,
                    card_valid,
                    log_probs,
                    _entropies,
                ) = model_decoder_batch(outputs, masks, stochastic=True)
                values = outputs["value"]

            self.buffers.modes[round_index, active_mask] = modes.cpu()
            self.buffers.counts[round_index, active_mask] = counts.cpu()
            self.buffers.cards[round_index, active_mask] = cards.cpu()
            self.buffers.card_valid[round_index, active_mask] = card_valid.cpu()
            self.buffers.old_log_probs[round_index, active_mask] = log_probs.cpu()
            self.buffers.old_values[round_index, active_mask] = values.cpu()
            if self.profile:
                inference_seconds += perf_counter() - inference_started

            for worker in active_workers:
                self.connections[worker].send(("actions_ready", None))

            next_active_workers = []
            for worker in active_workers:
                command, still_active = self.connections[worker].recv()
                if command != "results_ready":
                    raise RuntimeError(f"unexpected actor response: {command}")
                if still_active:
                    next_active_workers.append(worker)

            active_workers = next_active_workers
            inference_batches += 1
            round_index += 1

        batch = RolloutBatch.from_buffers(
            self.buffers,
            self.episode_count,
            self.max_steps,
        )
        timings = RolloutTimings(
            rollout_seconds=perf_counter() - started,
            inference_seconds=inference_seconds,
            inference_batches=inference_batches,
        )
        return batch, timings

    def close(self) -> None:
        for connection in self.connections:
            try:
                connection.send(("close", None))
            except (BrokenPipeError, EOFError):
                pass
            connection.close()
        for process in self.processes:
            process.join(timeout=5)
            if process.is_alive():
                process.terminate()
                process.join()

    def __enter__(self) -> "RolloutPool":
        return self

    def __exit__(self, *_args) -> None:
        self.close()
