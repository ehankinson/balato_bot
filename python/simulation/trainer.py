import os
import random
import secrets
from concurrent.futures import ProcessPoolExecutor
from dataclasses import dataclass
from multiprocessing import get_context

import numpy as np
import torch
from torch.optim import Adam

from calculation.poker_discards import generate_discard_table
from calculation.score import get_best_scoring_hand
from core.enums import HandAction
from core.models import Deck, GameState
from simulation.blind_trainer import BlindModel
from simulation.decoder import build_mask, evaluate_actions, model_decoder
from simulation.encoder import encode_game_state, observation_dim
from simulation.reward import calculate_score_progress_reward, calculate_terminal_reward


@dataclass(slots=True)
class EpisodeRollout:
    observations: np.ndarray
    modes: np.ndarray
    counts: np.ndarray
    cards: np.ndarray
    card_valid: np.ndarray
    old_log_probs: np.ndarray
    old_values: np.ndarray
    rewards: np.ndarray
    mode_masks: np.ndarray
    count_masks: np.ndarray
    card_masks: np.ndarray
    won: int
    discards: int
    plays: int
    discarded_cards: int
    played_cards: int


_ROLLOUT_MODEL: BlindModel | None = None
_CPU_DEVICE = torch.device("cpu")


def _init_rollout_worker(model: BlindModel) -> None:
    """Give each CPU worker access to the shared, read-only rollout policy."""
    global _ROLLOUT_MODEL
    torch.set_num_threads(1)
    _ROLLOUT_MODEL = model
    _ROLLOUT_MODEL.eval()


def _collect_episode(args: tuple[int, int, int]) -> EpisodeRollout:
    """Play one complete episode on a CPU worker."""
    seed, score_to_beat, max_steps = args
    random.seed(seed)
    torch.manual_seed(seed)

    if _ROLLOUT_MODEL is None:
        raise RuntimeError("rollout worker was not initialized")

    deck = Deck()
    game_state = GameState(score_to_beat=score_to_beat)
    hand = deck.draw(game_state.hand_size)

    observations: list[torch.Tensor] = []
    modes: list[int] = []
    counts: list[int] = []
    cards: list[list[int]] = []
    card_valid: list[list[int]] = []
    old_log_probs: list[torch.Tensor] = []
    old_values: list[torch.Tensor] = []
    rewards: list[float] = []
    mode_masks: list[torch.Tensor] = []
    count_masks: list[torch.Tensor] = []
    card_masks: list[torch.Tensor] = []

    won = 0
    discards = 0
    plays = 0
    discarded_cards = 0
    played_cards = 0

    with torch.inference_mode():
        for _step in range(max_steps):
            best_hand = get_best_scoring_hand(hand, [], game_state)
            discard_table = generate_discard_table(deck, hand)
            observation = encode_game_state(
                hand, game_state, best_hand, discard_table
            )
            masks = build_mask(game_state, hand, _CPU_DEVICE)
            outputs = _ROLLOUT_MODEL(observation.unsqueeze(0))
            mode, count, card_indices, log_prob, _entropy = model_decoder(
                outputs, masks, _CPU_DEVICE, stochastic=True
            )

            observations.append(observation)
            modes.append(int(mode))
            counts.append(count)
            cards.append(card_indices + [-1] * (5 - len(card_indices)))
            card_valid.append(
                [1] * len(card_indices) + [0] * (5 - len(card_indices))
            )
            old_log_probs.append(log_prob.detach())
            old_values.append(outputs["value"].squeeze().detach())
            rewards.append(0.0)
            mode_masks.append(masks.mode)
            count_masks.append(masks.count)
            card_masks.append(masks.card)

            selected_cards = [hand[index] for index in card_indices]
            for selected_card in selected_cards:
                hand.remove(selected_card)

            score_before = game_state.current_score
            has_won = game_state.execute_hand_action(
                mode, selected_cards, hand, deck
            )

            if mode == HandAction.DISCARD:
                discards += 1
                discarded_cards += count
            else:
                plays += 1
                played_cards += count
                rewards[-1] += calculate_score_progress_reward(
                    score_before, game_state
                )

            terminated = has_won or game_state.hands == 0
            if terminated:
                rewards[-1] += calculate_terminal_reward(game_state)
                won = int(has_won)
                break
        else:
            rewards[-1] += calculate_terminal_reward(game_state)

    return EpisodeRollout(
        observations=torch.stack(observations).numpy(),
        modes=np.asarray(modes, dtype=np.int64),
        counts=np.asarray(counts, dtype=np.int64),
        cards=np.asarray(cards, dtype=np.int64),
        card_valid=np.asarray(card_valid, dtype=np.float32),
        old_log_probs=torch.stack(old_log_probs).numpy(),
        old_values=torch.stack(old_values).numpy(),
        rewards=np.asarray(rewards, dtype=np.float32),
        mode_masks=torch.stack(mode_masks).numpy(),
        count_masks=torch.stack(count_masks).numpy(),
        card_masks=torch.stack(card_masks).numpy(),
        won=won,
        discards=discards,
        plays=plays,
        discarded_cards=discarded_cards,
        played_cards=played_cards,
    )


def _compute_gae(
    rewards: torch.Tensor,
    old_values: torch.Tensor,
    episode_ends: list[int],
    gamma: float,
    lam: float,
) -> tuple[torch.Tensor, torch.Tensor]:
    advantages = torch.zeros_like(rewards)
    start = 0
    for end in episode_ends:
        gae = 0.0
        for step in range(end - 1, start - 1, -1):
            terminal = step == end - 1
            next_value = 0.0 if terminal else float(old_values[step + 1])
            delta = float(rewards[step]) + gamma * next_value - float(
                old_values[step]
            )
            gae = delta + gamma * lam * (0.0 if terminal else gae)
            advantages[step] = gae
        start = end

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
    rollout_workers = int(os.getenv("BALATRO_ROLLOUT_WORKERS", str(default_workers)))
    requested_device = os.getenv(
        "BALATRO_DEVICE", "cuda" if torch.cuda.is_available() else "cpu"
    )
    if requested_device.startswith("cuda") and not torch.cuda.is_available():
        raise RuntimeError(
            "BALATRO_DEVICE requests CUDA, but torch.cuda.is_available() is false"
        )
    update_device = torch.device(requested_device)

    rollout_model = BlindModel(observation_dim(8), hidden_size=hidden_size)
    rollout_model.share_memory()
    rollout_model.eval()

    model = BlindModel(observation_dim(8), hidden_size=hidden_size).to(update_device)
    model.load_state_dict(rollout_model.state_dict())
    optimizer = Adam(model.parameters(), lr=lr)
    recent_wins: list[float] = []

    print(
        f"device={update_device} workers={rollout_workers} "
        f"episodes/update={episodes_per_update} minibatch={minibatch_size} "
        f"seed={run_seed}"
    )

    worker_context = get_context("spawn")
    with ProcessPoolExecutor(
        max_workers=rollout_workers,
        mp_context=worker_context,
        initializer=_init_rollout_worker,
        initargs=(rollout_model,),
    ) as executor:
        for outer in range(iterations):
            task_args = (
                (
                    run_seed + outer * episodes_per_update + episode,
                    score_to_beat,
                    max_steps_per_episode,
                )
                for episode in range(episodes_per_update)
            )
            chunksize = max(episodes_per_update // (rollout_workers * 4), 1)
            episodes = list(
                executor.map(_collect_episode, task_args, chunksize=chunksize)
            )

            episode_ends: list[int] = []
            total_steps = 0
            for episode in episodes:
                total_steps += len(episode.rewards)
                episode_ends.append(total_steps)

            batch_obs = torch.from_numpy(
                np.concatenate([episode.observations for episode in episodes])
            )
            batch_modes = torch.from_numpy(
                np.concatenate([episode.modes for episode in episodes])
            )
            batch_counts = torch.from_numpy(
                np.concatenate([episode.counts for episode in episodes])
            )
            batch_cards = torch.from_numpy(
                np.concatenate([episode.cards for episode in episodes])
            )
            batch_card_valid = torch.from_numpy(
                np.concatenate([episode.card_valid for episode in episodes])
            )
            batch_old_log_probs = torch.from_numpy(
                np.concatenate([episode.old_log_probs for episode in episodes])
            )
            batch_old_values = torch.from_numpy(
                np.concatenate([episode.old_values for episode in episodes])
            )
            batch_rewards = torch.from_numpy(
                np.concatenate([episode.rewards for episode in episodes])
            )
            batch_mode_masks = torch.from_numpy(
                np.concatenate([episode.mode_masks for episode in episodes])
            )
            batch_count_masks = torch.from_numpy(
                np.concatenate([episode.count_masks for episode in episodes])
            )
            batch_card_masks = torch.from_numpy(
                np.concatenate([episode.card_masks for episode in episodes])
            )

            batch_returns, advantages = _compute_gae(
                batch_rewards,
                batch_old_values,
                episode_ends,
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

            loss_total = 0.0
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

                    loss_total += loss.item()
                    policy_loss_total += policy_loss.item()
                    value_loss_total += value_loss.item()
                    entropy_total += entropy_loss.item()
                    update_count += 1

            # Workers are idle after executor.map, so updating shared weights is safe.
            rollout_model.load_state_dict(_cpu_state_dict(model))

            wins_in_batch = sum(episode.won for episode in episodes)
            discards_in_batch = sum(episode.discards for episode in episodes)
            plays_in_batch = sum(episode.plays for episode in episodes)
            discarded_cards = sum(episode.discarded_cards for episode in episodes)
            played_cards = sum(episode.played_cards for episode in episodes)
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
                    f"steps={total_steps} updates={update_count}"
                )

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
