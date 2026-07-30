from dataclasses import dataclass
from time import perf_counter

import torch
from torch.optim import Optimizer

from simulation.blind_trainer import BlindModel
from simulation.decoder import evaluate_actions
from simulation.rollout import RolloutBatch
from simulation.training_config import TrainingConfig


@dataclass(frozen=True, slots=True)
class PPOUpdateMetrics:
    policy_loss: float
    value_loss: float
    entropy_loss: float
    update_count: int
    gae_seconds: float
    transfer_seconds: float
    update_seconds: float


def synchronize(device: torch.device) -> None:
    if device.type == "cuda":
        torch.cuda.synchronize(device)


def compute_gae(
    rewards: torch.Tensor,
    old_values: torch.Tensor,
    episode_ids: torch.Tensor,
    episode_count: int,
    gamma: float,
    gae_lambda: float,
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
            gae = delta + gamma * gae_lambda * (0.0 if terminal else gae)
            advantages[step] = gae

    returns = advantages + old_values
    normalized_advantages = (advantages - advantages.mean()) / (
        advantages.std(unbiased=False) + 1e-8
    )
    return returns, normalized_advantages


def update_policy(
    model: BlindModel,
    optimizer: Optimizer,
    batch: RolloutBatch,
    config: TrainingConfig,
    device: torch.device,
) -> PPOUpdateMetrics:
    gae_started = perf_counter()
    returns, advantages = compute_gae(
        batch.rewards,
        batch.old_values,
        batch.episode_ids,
        config.episodes_per_update,
        config.gamma,
        config.gae_lambda,
    )
    gae_seconds = perf_counter() - gae_started

    if config.profile:
        synchronize(device)
    transfer_started = perf_counter()
    observations = batch.observations.to(device)
    modes = batch.modes.to(device)
    counts = batch.counts.to(device)
    cards = batch.cards.to(device)
    card_valid = batch.card_valid.to(device)
    old_log_probs = batch.old_log_probs.to(device)
    returns = returns.to(device)
    advantages = advantages.to(device)
    mode_masks = batch.mode_masks.to(device)
    count_masks = batch.count_masks.to(device)
    card_masks = batch.card_masks.to(device)
    if config.profile:
        synchronize(device)
    transfer_seconds = perf_counter() - transfer_started

    policy_loss_total = 0.0
    value_loss_total = 0.0
    entropy_loss_total = 0.0
    update_count = 0
    if config.profile:
        synchronize(device)
    update_started = perf_counter()
    model.train()
    for _epoch in range(config.ppo_epochs):
        permutation = torch.randperm(batch.total_steps, device=device)
        for start in range(0, batch.total_steps, config.minibatch_size):
            indices = permutation[start : start + config.minibatch_size]
            outputs = model(observations[indices])
            new_log_probs, new_entropies = evaluate_actions(
                outputs,
                mode_masks[indices],
                count_masks[indices],
                card_masks[indices],
                modes[indices],
                counts[indices],
                cards[indices],
                card_valid[indices],
            )
            new_values = outputs["value"]

            ratio = torch.exp(new_log_probs - old_log_probs[indices])
            minibatch_advantages = advantages[indices]
            surrogate_1 = ratio * minibatch_advantages
            surrogate_2 = (
                torch.clamp(
                    ratio,
                    1 - config.clip_ratio,
                    1 + config.clip_ratio,
                )
                * minibatch_advantages
            )
            policy_loss = -torch.min(surrogate_1, surrogate_2).mean()
            value_loss = ((new_values - returns[indices]) ** 2).mean()
            entropy_loss = -new_entropies.mean()
            loss = (
                policy_loss
                + 0.5 * value_loss
                + config.entropy_coefficient * entropy_loss
            )

            optimizer.zero_grad(set_to_none=True)
            loss.backward()
            torch.nn.utils.clip_grad_norm_(
                model.parameters(),
                max_norm=config.max_gradient_norm,
            )
            optimizer.step()

            policy_loss_total += policy_loss.item()
            value_loss_total += value_loss.item()
            entropy_loss_total += entropy_loss.item()
            update_count += 1

    if config.profile:
        synchronize(device)
    update_seconds = perf_counter() - update_started
    return PPOUpdateMetrics(
        policy_loss=policy_loss_total / update_count,
        value_loss=value_loss_total / update_count,
        entropy_loss=entropy_loss_total / update_count,
        update_count=update_count,
        gae_seconds=gae_seconds,
        transfer_seconds=transfer_seconds,
        update_seconds=update_seconds,
    )
