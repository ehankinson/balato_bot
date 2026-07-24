import os
import random
import secrets
from dataclasses import dataclass
from multiprocessing import get_context
from multiprocessing.connection import Connection
from time import perf_counter

import torch
from torch.optim import Adam

from calculation.poker_discards import generate_discard_table
from calculation.score import get_best_scoring_hand
from core.enums import HandAction
from core.models import Card, Deck, GameState
from simulation.blind_env import ActionMasks
from simulation.blind_trainer import BlindModel
from simulation.decoder import (
    build_mask,
    evaluate_actions,
    model_decoder_batch,
)
from simulation.encoder import encode_game_state, observation_dim
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
    old_log_probs: torch.Tensor
    old_values: torch.Tensor
    rewards: torch.Tensor
    won: torch.Tensor
    score_seconds: torch.Tensor
    discard_seconds: torch.Tensor
    encode_seconds: torch.Tensor
    env_seconds: torch.Tensor

    @classmethod
    def create(
        cls, rollout_rounds: int, episode_count: int, input_size: int
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
            old_log_probs=shared(*prefix, dtype=torch.float32),
            old_values=shared(*prefix, dtype=torch.float32),
            rewards=shared(*prefix, dtype=torch.float32),
            won=shared(*prefix, dtype=torch.bool),
            score_seconds=shared(*prefix, dtype=torch.float64),
            discard_seconds=shared(*prefix, dtype=torch.float64),
            encode_seconds=shared(*prefix, dtype=torch.float64),
            env_seconds=shared(*prefix, dtype=torch.float64),
        )

    def clear(self) -> None:
        self.valid.zero_()
        self.rewards.zero_()
        self.won.zero_()
        self.score_seconds.zero_()
        self.discard_seconds.zero_()
        self.encode_seconds.zero_()
        self.env_seconds.zero_()


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
                slot.hand, slot.game_state, best_hand, discard_table
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
            buffers.rewards[round_index, episode] += calculate_score_progress_reward(
                score_before, slot.game_state
            )

        is_terminal = has_won or slot.game_state.hands == 0 or slot.steps >= max_steps
        if is_terminal:
            buffers.rewards[round_index, episode] += calculate_terminal_reward(
                slot.game_state
            )
            slot.active = False
            buffers.won[round_index, episode] = has_won

        if profile:
            buffers.env_seconds[round_index, episode] = perf_counter() - started

    return any(slot.active for slot in slots)


def _rollout_actor(
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
                    slots, buffers, round_index, episode_offset, profile
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


def _compute_gae(
    rewards: torch.Tensor,
    old_values: torch.Tensor,
    episode_ids: torch.Tensor,
    episode_count: int,
    gamma: float,
    lam: float,
) -> tuple[torch.Tensor, torch.Tensor]:
    advantages = torch.zeros_like(rewards)
    episode_steps: list[list[int]] = [[] for _ in range(episode_count)]
    for step, episode_id in enumerate(episode_ids.tolist()):
        episode_steps[episode_id].append(step)

    for steps in episode_steps:
        gae = 0.0
        for position in range(len(steps) - 1, -1, -1):
            step = steps[position]
            terminal = position == len(steps) - 1
            next_value = 0.0 if terminal else float(old_values[steps[position + 1]])
            delta = float(rewards[step]) + gamma * next_value - float(old_values[step])
            gae = delta + gamma * lam * (0.0 if terminal else gae)
            advantages[step] = gae

    returns = advantages + old_values
    normalized_advantages = (advantages - advantages.mean()) / (
        advantages.std(unbiased=False) + 1e-8
    )
    return returns, normalized_advantages


def _cpu_state_dict(model: BlindModel) -> dict[str, torch.Tensor]:
    return {
        name: parameter.detach().cpu() for name, parameter in model.state_dict().items()
    }


def _worker_distribution(
    episode_count: int, worker_count: int
) -> tuple[list[int], list[int]]:
    base, remainder = divmod(episode_count, worker_count)
    counts = [base + int(worker < remainder) for worker in range(worker_count)]
    offsets: list[int] = []
    offset = 0
    for count in counts:
        offsets.append(offset)
        offset += count
    return counts, offsets


def _synchronize(device: torch.device) -> None:
    if device.type == "cuda":
        torch.cuda.synchronize(device)


def main() -> None:
    iterations = int(os.getenv("BALATRO_ITERATIONS", "600"))
    episodes_per_update = int(os.getenv("BALATRO_EPISODES_PER_UPDATE", "1024"))
    ppo_epochs = int(os.getenv("BALATRO_PPO_EPOCHS", "4"))
    minibatch_size = int(os.getenv("BALATRO_MINIBATCH_SIZE", "512"))
    max_steps_per_episode = 20
    clip_ratio = 0.2
    lr = 3e-4
    entropy_coef = 0.001
    gamma = 0.99
    lam = 0.95
    hidden_size = 256
    checkpoint_path = os.getenv("BALATRO_CHECKPOINT", "ppo_blind.pt")
    profile = os.getenv("BALATRO_PROFILE", "0").lower() not in {
        "0",
        "false",
        "no",
    }
    configured_seed = os.getenv("BALATRO_SEED")
    run_seed = (
        int(configured_seed) if configured_seed is not None else secrets.randbits(63)
    )

    default_workers = min(8, max((os.cpu_count() or 2) - 1, 1))
    requested_workers = int(os.getenv("BALATRO_ROLLOUT_WORKERS", str(default_workers)))
    rollout_workers = min(requested_workers, episodes_per_update)
    requested_device = os.getenv(
        "BALATRO_DEVICE",
        "cuda"
        if torch.cuda.is_available()
        else "mps"
        if torch.backends.mps.is_available()
        else "cpu",
    )

    if requested_device.startswith("cuda") and not torch.cuda.is_available():
        raise RuntimeError(
            "BALATRO_DEVICE requests CUDA, but torch.cuda.is_available() is false"
        )
    update_device = torch.device(requested_device)

    random.seed(run_seed)
    torch.manual_seed(run_seed)
    if update_device.type == "cuda":
        torch.cuda.manual_seed_all(run_seed)

    model = BlindModel(observation_dim(8), hidden_size=hidden_size).to(update_device)
    optimizer = Adam(model.parameters(), lr=lr)
    recent_wins: list[float] = []

    worker_counts, worker_offsets = _worker_distribution(
        episodes_per_update, rollout_workers
    )
    buffers = SharedRolloutBuffers.create(
        max_steps_per_episode,
        episodes_per_update,
        observation_dim(8),
    )
    worker_context = get_context("spawn")
    connections: list[Connection] = []
    processes = []
    for worker in range(rollout_workers):
        parent_connection, child_connection = worker_context.Pipe()
        process = worker_context.Process(
            target=_rollout_actor,
            args=(
                child_connection,
                buffers,
                worker_counts[worker],
                worker_offsets[worker],
                random.choice([300, 450, 600]),
                max_steps_per_episode,
                profile,
            ),
        )
        process.start()
        child_connection.close()
        connections.append(parent_connection)
        processes.append(process)

    print(
        f"device={update_device} workers={rollout_workers} "
        f"episodes/update={episodes_per_update} minibatch={minibatch_size} "
        f"seed={run_seed} batched_inference=true shared_memory=true"
        f" profile={str(profile).lower()}"
    )

    try:
        for outer in range(iterations):
            iteration_started = perf_counter()
            buffers.clear()
            batch_seed = run_seed + outer * episodes_per_update
            for connection in connections:
                connection.send(("start", batch_seed))

            active_workers = list(range(rollout_workers))
            inference_batches = 0
            inference_seconds = 0.0
            round_index = 0
            while active_workers:
                for worker in active_workers:
                    command, _active_count = connections[worker].recv()
                    if command != "observations_ready":
                        raise RuntimeError(f"unexpected actor response: {command}")

                inference_started = perf_counter()
                active_mask = buffers.valid[round_index]
                observation_gpu = buffers.observations[round_index, active_mask].to(
                    update_device
                )
                masks_gpu = ActionMasks(
                    mode=buffers.mode_masks[round_index, active_mask].to(update_device),
                    count=buffers.count_masks[round_index, active_mask].to(
                        update_device
                    ),
                    card=buffers.card_masks[round_index, active_mask].to(update_device),
                )
                model.eval()
                with torch.inference_mode():
                    outputs = model(observation_gpu)
                    (
                        modes_gpu,
                        counts_gpu,
                        cards_gpu,
                        card_valid_gpu,
                        log_probs_gpu,
                        _entropies_gpu,
                    ) = model_decoder_batch(outputs, masks_gpu, stochastic=True)
                    values_gpu = outputs["value"]

                buffers.modes[round_index, active_mask] = modes_gpu.cpu()
                buffers.counts[round_index, active_mask] = counts_gpu.cpu()
                buffers.cards[round_index, active_mask] = cards_gpu.cpu()
                buffers.card_valid[round_index, active_mask] = card_valid_gpu.cpu()
                buffers.old_log_probs[round_index, active_mask] = log_probs_gpu.cpu()
                buffers.old_values[round_index, active_mask] = values_gpu.cpu()
                if profile:
                    inference_seconds += perf_counter() - inference_started

                for worker in active_workers:
                    connections[worker].send(("actions_ready", None))

                next_active_workers = []
                for worker in active_workers:
                    command, still_active = connections[worker].recv()
                    if command != "results_ready":
                        raise RuntimeError(f"unexpected actor response: {command}")
                    if still_active:
                        next_active_workers.append(worker)

                active_workers = next_active_workers
                inference_batches += 1
                round_index += 1

            valid = buffers.valid
            batch_obs = buffers.observations[valid]
            batch_modes = buffers.modes[valid]
            batch_counts = buffers.counts[valid]
            batch_cards = buffers.cards[valid]
            batch_card_valid = buffers.card_valid[valid]
            batch_old_log_probs = buffers.old_log_probs[valid]
            batch_old_values = buffers.old_values[valid]
            batch_rewards = buffers.rewards[valid]
            batch_mode_masks = buffers.mode_masks[valid]
            batch_count_masks = buffers.count_masks[valid]
            batch_card_masks = buffers.card_masks[valid]
            episode_grid = torch.arange(episodes_per_update).expand(
                max_steps_per_episode, -1
            )
            batch_episode_ids = episode_grid[valid]
            total_steps = len(batch_rewards)

            wins_in_batch = int(buffers.won.sum())
            discard_mask = batch_modes == HandAction.DISCARD
            play_mask = batch_modes == HandAction.PLAY_HAND
            discards_in_batch = int(discard_mask.sum())
            plays_in_batch = int(play_mask.sum())
            discarded_cards = int(batch_counts[discard_mask].sum())
            played_cards = int(batch_counts[play_mask].sum())
            rollout_seconds = perf_counter() - iteration_started

            gae_started = perf_counter()
            batch_returns, advantages = _compute_gae(
                batch_rewards,
                batch_old_values,
                batch_episode_ids,
                episodes_per_update,
                gamma,
                lam,
            )
            gae_seconds = perf_counter() - gae_started

            if profile:
                _synchronize(update_device)
            transfer_started = perf_counter()
            batch_obs = batch_obs.to(update_device)
            batch_modes = batch_modes.to(update_device)
            batch_counts = batch_counts.to(update_device)
            batch_cards = batch_cards.to(update_device)
            batch_card_valid = batch_card_valid.to(update_device)
            batch_old_log_probs = batch_old_log_probs.to(update_device)
            batch_returns = batch_returns.to(update_device)
            advantages = advantages.to(update_device)
            batch_mode_masks = batch_mode_masks.to(update_device)
            batch_count_masks = batch_count_masks.to(update_device)
            batch_card_masks = batch_card_masks.to(update_device)
            if profile:
                _synchronize(update_device)
            transfer_seconds = perf_counter() - transfer_started

            policy_loss_total = 0.0
            value_loss_total = 0.0
            entropy_total = 0.0
            update_count = 0
            if profile:
                _synchronize(update_device)
            update_started = perf_counter()
            model.train()
            for _epoch in range(ppo_epochs):
                permutation = torch.randperm(total_steps, device=update_device)
                for start in range(0, total_steps, minibatch_size):
                    indices = permutation[start : start + minibatch_size]
                    outputs = model(batch_obs[indices])
                    new_log_probs, new_entropies = evaluate_actions(
                        outputs,
                        batch_mode_masks[indices],
                        batch_count_masks[indices],
                        batch_card_masks[indices],
                        batch_modes[indices],
                        batch_counts[indices],
                        batch_cards[indices],
                        batch_card_valid[indices],
                    )
                    new_values = outputs["value"]

                    ratio = torch.exp(new_log_probs - batch_old_log_probs[indices])
                    minibatch_advantages = advantages[indices]
                    surrogate_1 = ratio * minibatch_advantages
                    surrogate_2 = (
                        torch.clamp(ratio, 1 - clip_ratio, 1 + clip_ratio)
                        * minibatch_advantages
                    )
                    policy_loss = -torch.min(surrogate_1, surrogate_2).mean()
                    value_loss = ((new_values - batch_returns[indices]) ** 2).mean()
                    entropy_loss = -new_entropies.mean()
                    loss = policy_loss + 0.5 * value_loss + entropy_coef * entropy_loss

                    optimizer.zero_grad(set_to_none=True)
                    loss.backward()
                    torch.nn.utils.clip_grad_norm_(model.parameters(), max_norm=0.5)
                    optimizer.step()

                    policy_loss_total += policy_loss.item()
                    value_loss_total += value_loss.item()
                    entropy_total += entropy_loss.item()
                    update_count += 1

            if profile:
                _synchronize(update_device)
            update_seconds = perf_counter() - update_started
            iteration_seconds = perf_counter() - iteration_started

            recent_wins.append(wins_in_batch / episodes_per_update)
            if outer % 5 == 0:
                win_average = sum(recent_wins[-10:]) / len(recent_wins[-10:])
                print(
                    f"iter {outer:4d}: win={win_average:.2f} "
                    f"policy={policy_loss_total / update_count:.3f} "
                    f"value={value_loss_total / update_count:.3f} "
                    f"entropy={entropy_total / update_count:.3f} "
                    f"discards/ep={discards_in_batch / episodes_per_update:.2f} "
                    f"cards/discard={discarded_cards / max(discards_in_batch, 1):.2f} "
                    f"cards/play={played_cards / max(plays_in_batch, 1):.2f} "
                    f"steps={total_steps} inference_batches={inference_batches} "
                    f"updates={update_count}"
                )
                if profile:
                    milliseconds_per_step = 1000.0 / total_steps
                    print(
                        "  profile: "
                        f"rollout={rollout_seconds:.3f}s "
                        f"gpu_inference={inference_seconds:.3f}s "
                        f"gae={gae_seconds:.3f}s "
                        f"transfer={transfer_seconds:.3f}s "
                        f"ppo={update_seconds:.3f}s "
                        f"total={iteration_seconds:.3f}s | "
                        "avg/step: "
                        f"score={buffers.score_seconds[valid].sum() * milliseconds_per_step:.3f}ms "
                        f"discard={buffers.discard_seconds[valid].sum() * milliseconds_per_step:.3f}ms "
                        f"encode={buffers.encode_seconds[valid].sum() * milliseconds_per_step:.3f}ms "
                        f"env={buffers.env_seconds[valid].sum() * milliseconds_per_step:.3f}ms"
                    )
    finally:
        for connection in connections:
            try:
                connection.send(("close", None))
            except (BrokenPipeError, EOFError):
                pass
            connection.close()
        for process in processes:
            process.join(timeout=5)
            if process.is_alive():
                process.terminate()
                process.join()

    torch.save(
        {
            "state_dict": _cpu_state_dict(model),
            "input_size": observation_dim(8),
            "hidden_size": hidden_size,
            "architecture_version": 2,
        },
        checkpoint_path,
    )
    print(f"saved {checkpoint_path}")


if __name__ == "__main__":
    main()
