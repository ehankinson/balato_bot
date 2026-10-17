from dataclasses import dataclass

import torch

from simulation.decoder import evaluate_actions


@dataclass(frozen=True)
class PPOUpdateMetrics:
    policy_loss: float
    value_loss: float
    entropy: float
    update_count: int


def compute_gae(episodes, gamma, gae_lambda):
    """Bootstrap external cutoffs, zero-bootstrap wins/losses, never cross episodes."""
    transitions, returns, advantages = [], [], []
    for episode in episodes:
        next_value, gae = episode.bootstrap_value, 0.0
        episode_advantages = []
        for transition in reversed(episode.transitions):
            continuation = 0.0 if transition.terminated else 1.0
            delta = (
                transition.reward
                + gamma * next_value * continuation
                - transition.old_value
            )
            gae = delta + gamma * gae_lambda * continuation * gae
            episode_advantages.append(gae)
            next_value = transition.old_value
        episode_advantages.reverse()
        transitions.extend(episode.transitions)
        advantages.extend(episode_advantages)
        returns.extend(
            advantage + transition.old_value
            for advantage, transition in zip(episode_advantages, episode.transitions)
        )
    advantages = torch.tensor(advantages, dtype=torch.float32)
    if not len(advantages):
        raise ValueError("PPO requires transitions")
    advantages = (advantages - advantages.mean()) / (
        advantages.std(unbiased=False) + 1e-8
    )
    return transitions, torch.tensor(returns, dtype=torch.float32), advantages


def update_policy(model, optimizer, episodes, config):
    transitions, returns, advantages = compute_gae(
        episodes, config.gamma, config.gae_lambda
    )
    device = next(model.parameters()).device
    returns, advantages = returns.to(device), advantages.to(device)
    old_logs = torch.tensor(
        [transition.old_log_prob for transition in transitions], device=device
    )
    totals = torch.zeros(3)
    updates = 0
    model.train()
    for _ in range(config.ppo_epochs):
        for indices in torch.randperm(len(transitions)).split(config.minibatch_size):
            selected = [transitions[index] for index in indices.tolist()]
            output = model([transition.observation for transition in selected])
            logs, entropy = evaluate_actions(
                model, output, [transition.trace for transition in selected]
            )
            idx = indices.to(device)
            ratio = (logs - old_logs[idx]).exp()
            policy_loss = -torch.minimum(
                ratio * advantages[idx],
                ratio.clamp(1 - config.clip_ratio, 1 + config.clip_ratio)
                * advantages[idx],
            ).mean()
            value_loss = (output.value - returns[idx]).square().mean()
            loss = (
                policy_loss
                + 0.5 * value_loss
                - config.entropy_coefficient * entropy.mean()
            )
            if not torch.isfinite(loss):
                raise FloatingPointError("non-finite PPO loss")
            optimizer.zero_grad(set_to_none=True)
            loss.backward()
            torch.nn.utils.clip_grad_norm_(
                model.parameters(), config.max_gradient_norm, error_if_nonfinite=True
            )
            optimizer.step()
            totals += torch.tensor(
                [
                    float(policy_loss.detach()),
                    float(value_loss.detach()),
                    float(entropy.mean().detach()),
                ]
            )
            updates += 1
    return PPOUpdateMetrics(*(totals / updates).tolist(), updates)
