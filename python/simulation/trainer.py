import os
import random
import secrets
from dataclasses import dataclass
from multiprocessing import get_context
from multiprocessing.connection import Connection

import numpy as np
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


def _empty_observation_payload() -> dict[str, np.ndarray]:
    return {
        "slot_ids": np.empty(0, dtype=np.int64),
        "observations": np.empty((0, observation_dim(8)), dtype=np.float32),
        "mode_masks": np.empty((0, 2), dtype=np.float32),
        "count_masks": np.empty((0, 5), dtype=np.float32),
        "card_masks": np.empty((0, 8), dtype=np.float32),
    }


def _build_observation_payload(slots: list[GameSlot]) -> dict[str, np.ndarray]:
    slot_ids: list[int] = []
    observations: list[torch.Tensor] = []
    mode_masks: list[torch.Tensor] = []
    count_masks: list[torch.Tensor] = []
    card_masks: list[torch.Tensor] = []

    with torch.inference_mode():
        for slot_id, slot in enumerate(slots):
            if not slot.active:
                continue

            best_hand = get_best_scoring_hand(
                slot.hand, [], slot.game_state
            )
            discard_table = generate_discard_table(slot.deck, slot.hand)
            observation = encode_game_state(
                slot.hand,
                slot.game_state,
                best_hand,
                discard_table,
            )
            masks = build_mask(slot.game_state, slot.hand, torch.device("cpu"))

            slot_ids.append(slot_id)
            observations.append(observation)
            mode_masks.append(masks.mode)
            count_masks.append(masks.count)
            card_masks.append(masks.card)

    if not slot_ids:
        return _empty_observation_payload()

    return {
        "slot_ids": np.asarray(slot_ids, dtype=np.int64),
        "observations": torch.stack(observations).numpy(),
        "mode_masks": torch.stack(mode_masks).numpy(),
        "count_masks": torch.stack(count_masks).numpy(),
        "card_masks": torch.stack(card_masks).numpy(),
    }


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


def _apply_actions(
    slots: list[GameSlot],
    slot_ids: np.ndarray,
    modes: np.ndarray,
    counts: np.ndarray,
    cards: np.ndarray,
    max_steps: int,
) -> dict[str, np.ndarray]:
    rewards = np.zeros(len(slot_ids), dtype=np.float32)
    won = np.zeros(len(slot_ids), dtype=np.int64)
    terminated = np.zeros(len(slot_ids), dtype=np.bool_)

    for row, slot_id_value in enumerate(slot_ids):
        slot = slots[int(slot_id_value)]
        mode = HandAction(int(modes[row]))
        count = int(counts[row])
        card_indices = [int(index) for index in cards[row, :count]]
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
            rewards[row] += calculate_score_progress_reward(
                score_before, slot.game_state
            )

        is_terminal = (
            has_won
            or slot.game_state.hands == 0
            or slot.steps >= max_steps
        )
        if is_terminal:
            rewards[row] += calculate_terminal_reward(slot.game_state)
            slot.active = False
            won[row] = int(has_won)
            terminated[row] = True

    return {
        "rewards": rewards,
        "won": won,
        "terminated": terminated,
    }


def _rollout_actor(
    connection: Connection,
    episode_count: int,
    episode_offset: int,
    score_to_beat: int,
    max_steps: int,
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
            connection.send(_build_observation_payload(slots))

            while any(slot.active for slot in slots):
                command, action_payload = connection.recv()
                if command == "close":
                    return
                if command != "actions":
                    raise RuntimeError(f"unexpected actor command: {command}")

                results = _apply_actions(
                    slots,
                    action_payload["slot_ids"],
                    action_payload["modes"],
                    action_payload["counts"],
                    action_payload["cards"],
                    max_steps,
                )
                connection.send(results)
                connection.send(_build_observation_payload(slots))
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
            next_value = 0.0 if terminal else float(
                old_values[steps[position + 1]]
            )
            delta = float(rewards[step]) + gamma * next_value - float(
                old_values[step]
            )
            gae = delta + gamma * lam * (0.0 if terminal else gae)
            advantages[step] = gae

    returns = advantages + old_values
    normalized_advantages = (advantages - advantages.mean()) / (
        advantages.std(unbiased=False) + 1e-8
    )
    return returns, normalized_advantages


def _cpu_state_dict(model: BlindModel) -> dict[str, torch.Tensor]:
    return {
        name: parameter.detach().cpu()
        for name, parameter in model.state_dict().items()
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


def _concatenate_payload(
    payloads: list[dict[str, np.ndarray]], key: str
) -> torch.Tensor:
    return torch.from_numpy(np.concatenate([payload[key] for payload in payloads]))


def main() -> None:
    iterations = int(os.getenv("BALATRO_ITERATIONS", "100"))
    episodes_per_update = int(os.getenv("BALATRO_EPISODES_PER_UPDATE", "1024"))
    ppo_epochs = int(os.getenv("BALATRO_PPO_EPOCHS", "4"))
    minibatch_size = int(os.getenv("BALATRO_MINIBATCH_SIZE", "512"))
    max_steps_per_episode = 20
    clip_ratio = 0.2
    lr = 3e-4
    entropy_coef = 0.001
    gamma = 0.99
    lam = 0.95
    score_to_beat = 300
    hidden_size = 256
    checkpoint_path = os.getenv("BALATRO_CHECKPOINT", "ppo_blind.pt")
    configured_seed = os.getenv("BALATRO_SEED")
    run_seed = (
        int(configured_seed)
        if configured_seed is not None
        else secrets.randbits(63)
    )

    default_workers = min(8, max((os.cpu_count() or 2) - 1, 1))
    requested_workers = int(
        os.getenv("BALATRO_ROLLOUT_WORKERS", str(default_workers))
    )
    rollout_workers = min(requested_workers, episodes_per_update)
    requested_device = os.getenv(
        "BALATRO_DEVICE", "cuda" if torch.cuda.is_available() else "cpu"
    )
    if requested_device.startswith("cuda") and not torch.cuda.is_available():
        raise RuntimeError(
            "BALATRO_DEVICE requests CUDA, but torch.cuda.is_available() is false"
        )
    update_device = torch.device(requested_device)

    model = BlindModel(observation_dim(8), hidden_size=hidden_size).to(update_device)
    optimizer = Adam(model.parameters(), lr=lr)
    recent_wins: list[float] = []

    worker_counts, worker_offsets = _worker_distribution(
        episodes_per_update, rollout_workers
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
                worker_counts[worker],
                worker_offsets[worker],
                score_to_beat,
                max_steps_per_episode,
            ),
        )
        process.start()
        child_connection.close()
        connections.append(parent_connection)
        processes.append(process)

    print(
        f"device={update_device} workers={rollout_workers} "
        f"episodes/update={episodes_per_update} minibatch={minibatch_size} "
        f"seed={run_seed} batched_inference=true"
    )

    try:
        for outer in range(iterations):
            batch_seed = run_seed + outer * episodes_per_update
            for connection in connections:
                connection.send(("start", batch_seed))
            observation_payloads = [
                connection.recv() for connection in connections
            ]

            observation_batches: list[torch.Tensor] = []
            mode_batches: list[torch.Tensor] = []
            count_batches: list[torch.Tensor] = []
            card_batches: list[torch.Tensor] = []
            card_valid_batches: list[torch.Tensor] = []
            log_prob_batches: list[torch.Tensor] = []
            value_batches: list[torch.Tensor] = []
            reward_batches: list[torch.Tensor] = []
            mode_mask_batches: list[torch.Tensor] = []
            count_mask_batches: list[torch.Tensor] = []
            card_mask_batches: list[torch.Tensor] = []
            episode_id_batches: list[torch.Tensor] = []

            wins_in_batch = 0
            discards_in_batch = 0
            plays_in_batch = 0
            discarded_cards = 0
            played_cards = 0
            inference_batches = 0

            while any(len(payload["slot_ids"]) for payload in observation_payloads):
                active_workers = [
                    worker
                    for worker, payload in enumerate(observation_payloads)
                    if len(payload["slot_ids"]) > 0
                ]
                active_payloads = [
                    observation_payloads[worker] for worker in active_workers
                ]
                observation_cpu = _concatenate_payload(
                    active_payloads, "observations"
                )
                mode_masks_cpu = _concatenate_payload(
                    active_payloads, "mode_masks"
                )
                count_masks_cpu = _concatenate_payload(
                    active_payloads, "count_masks"
                )
                card_masks_cpu = _concatenate_payload(
                    active_payloads, "card_masks"
                )

                observation_gpu = observation_cpu.to(update_device)
                masks_gpu = ActionMasks(
                    mode=mode_masks_cpu.to(update_device),
                    count=count_masks_cpu.to(update_device),
                    card=card_masks_cpu.to(update_device),
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

                modes_cpu = modes_gpu.cpu()
                counts_cpu = counts_gpu.cpu()
                cards_cpu = cards_gpu.cpu()
                card_valid_cpu = card_valid_gpu.cpu()
                log_probs_cpu = log_probs_gpu.cpu()
                values_cpu = values_gpu.cpu()

                observation_batches.append(observation_cpu)
                mode_batches.append(modes_cpu)
                count_batches.append(counts_cpu)
                card_batches.append(cards_cpu)
                card_valid_batches.append(card_valid_cpu)
                log_prob_batches.append(log_probs_cpu)
                value_batches.append(values_cpu)
                mode_mask_batches.append(mode_masks_cpu)
                count_mask_batches.append(count_masks_cpu)
                card_mask_batches.append(card_masks_cpu)

                episode_ids = []
                cursor = 0
                for worker in active_workers:
                    payload = observation_payloads[worker]
                    batch_length = len(payload["slot_ids"])
                    next_cursor = cursor + batch_length
                    slot_ids = payload["slot_ids"]
                    episode_ids.append(
                        torch.from_numpy(slot_ids + worker_offsets[worker])
                    )
                    connections[worker].send(
                        (
                            "actions",
                            {
                                "slot_ids": slot_ids,
                                "modes": modes_cpu[cursor:next_cursor].numpy(),
                                "counts": counts_cpu[cursor:next_cursor].numpy(),
                                "cards": cards_cpu[cursor:next_cursor].numpy(),
                            },
                        )
                    )
                    cursor = next_cursor
                episode_id_batches.append(torch.cat(episode_ids))

                result_payloads = {
                    worker: connections[worker].recv()
                    for worker in active_workers
                }
                rewards = torch.from_numpy(
                    np.concatenate(
                        [
                            result_payloads[worker]["rewards"]
                            for worker in active_workers
                        ]
                    )
                )
                reward_batches.append(rewards)
                wins_in_batch += sum(
                    int(result_payloads[worker]["won"].sum())
                    for worker in active_workers
                )
                discards_in_batch += int((modes_cpu == HandAction.DISCARD).sum())
                plays_in_batch += int((modes_cpu == HandAction.PLAY_HAND).sum())
                discarded_cards += int(
                    counts_cpu[modes_cpu == HandAction.DISCARD].sum()
                )
                played_cards += int(
                    counts_cpu[modes_cpu == HandAction.PLAY_HAND].sum()
                )

                next_payloads = [_empty_observation_payload() for _ in connections]
                for worker in active_workers:
                    next_payloads[worker] = connections[worker].recv()
                observation_payloads = next_payloads
                inference_batches += 1

            batch_obs = torch.cat(observation_batches)
            batch_modes = torch.cat(mode_batches)
            batch_counts = torch.cat(count_batches)
            batch_cards = torch.cat(card_batches)
            batch_card_valid = torch.cat(card_valid_batches)
            batch_old_log_probs = torch.cat(log_prob_batches)
            batch_old_values = torch.cat(value_batches)
            batch_rewards = torch.cat(reward_batches)
            batch_mode_masks = torch.cat(mode_mask_batches)
            batch_count_masks = torch.cat(count_mask_batches)
            batch_card_masks = torch.cat(card_mask_batches)
            batch_episode_ids = torch.cat(episode_id_batches)
            total_steps = len(batch_rewards)

            batch_returns, advantages = _compute_gae(
                batch_rewards,
                batch_old_values,
                batch_episode_ids,
                episodes_per_update,
                gamma,
                lam,
            )

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

            policy_loss_total = 0.0
            value_loss_total = 0.0
            entropy_total = 0.0
            update_count = 0
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

                    ratio = torch.exp(
                        new_log_probs - batch_old_log_probs[indices]
                    )
                    minibatch_advantages = advantages[indices]
                    surrogate_1 = ratio * minibatch_advantages
                    surrogate_2 = torch.clamp(
                        ratio, 1 - clip_ratio, 1 + clip_ratio
                    ) * minibatch_advantages
                    policy_loss = -torch.min(surrogate_1, surrogate_2).mean()
                    value_loss = (
                        (new_values - batch_returns[indices]) ** 2
                    ).mean()
                    entropy_loss = -new_entropies.mean()
                    loss = (
                        policy_loss
                        + 0.5 * value_loss
                        + entropy_coef * entropy_loss
                    )

                    optimizer.zero_grad(set_to_none=True)
                    loss.backward()
                    torch.nn.utils.clip_grad_norm_(model.parameters(), max_norm=0.5)
                    optimizer.step()

                    policy_loss_total += policy_loss.item()
                    value_loss_total += value_loss.item()
                    entropy_total += entropy_loss.item()
                    update_count += 1

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
