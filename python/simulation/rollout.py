"""Retained variable-length snapshots and exact conditional decoding traces."""

from dataclasses import dataclass

import torch

from simulation.decoder import ActionTrace, decode_action


@dataclass(frozen=True)
class Transition:
    observation: dict
    next_observation: dict
    trace: ActionTrace
    old_log_prob: float
    old_value: float
    reward: float
    accepted: bool
    terminated: bool
    truncated: bool
    error: str | None
    completed_blind_score: int | None


@dataclass(frozen=True)
class Episode:
    transitions: tuple[Transition, ...]
    bootstrap_value: float
    end_reason: str
    accepted_commands: int

    @property
    def final_observation(self):
        return self.transitions[-1].next_observation


def collect_episode(model, env, seed, stochastic=True):
    observation = env.reset(seed)
    transitions = []
    was_training = model.training
    model.eval()
    try:
        with torch.no_grad():
            while not env.done:
                output = model([observation])
                decoded = decode_action(
                    model, output, env.game, observation, stochastic
                )
                result = env.step(decoded.trace.command)
                if not result.accepted and decoded.trace.source_known:
                    raise RuntimeError(
                        f"Rust rejected a guaranteed-legal decoded command: {result.error}"
                    )
                transitions.append(
                    Transition(
                        observation,
                        result.observation,
                        decoded.trace,
                        float(decoded.log_prob),
                        float(output.value[0]),
                        result.reward,
                        result.accepted,
                        result.terminated,
                        result.truncated,
                        result.error,
                        result.completed_blind_score,
                    )
                )
                observation = result.observation
            bootstrap = (
                0.0 if result.terminated else float(model([observation]).value[0])
            )
    finally:
        model.train(was_training)
    return Episode(
        tuple(transitions), bootstrap, result.end_reason, env.accepted_commands
    )


def collect_episodes(model, env, stochastic=True):
    """Advance live slots together. Finished observations survive until explicit reset."""
    from simulation.decoder import decode_action_batch

    transitions = [[] for _ in env.states]
    completed = [None for _ in env.states]
    was_training = model.training
    model.eval()
    try:
        with torch.no_grad():
            while env.active.any():
                observations = env.snapshots
                active = env.active
                output = model(observations)
                decoded = decode_action_batch(
                    model, output, env.games, observations, active, stochastic
                )
                # Transfer statistics together, rather than reading a GPU scalar per slot.
                stats = (
                    torch.stack((decoded.log_probs, output.value), -1)
                    .detach()
                    .cpu()
                    .numpy()
                )
                results = env.step(decoded.commands)
                for index, result in enumerate(results.results):
                    if result is None:
                        continue
                    trace = decoded.traces[index]
                    if not result.accepted and trace.source_known:
                        raise RuntimeError(
                            f"Rust rejected a guaranteed-legal decoded command: {result.error}"
                        )
                    transitions[index].append(
                        Transition(
                            observations[index],
                            result.observation,
                            trace,
                            float(stats[index, 0]),
                            float(stats[index, 1]),
                            result.reward,
                            result.accepted,
                            result.terminated,
                            result.truncated,
                            result.error,
                            result.completed_blind_score,
                        )
                    )
                finished = [
                    index
                    for index, result in enumerate(results.results)
                    if result is not None and (result.terminated or result.truncated)
                ]
                truncated = [
                    index for index in finished if results.results[index].truncated
                ]
                bootstrap = {}
                if truncated:
                    values = (
                        model([env.snapshots[index] for index in truncated])
                        .value.detach()
                        .cpu()
                        .numpy()
                    )
                    bootstrap = dict(zip(truncated, values.tolist()))
                for index in finished:
                    completed[index] = Episode(
                        tuple(transitions[index]),
                        bootstrap.get(index, 0.0),
                        results.results[index].end_reason,
                        env.states[index].accepted_commands,
                    )
    finally:
        model.train(was_training)
    return completed


def collect_runs(
    model,
    seeds,
    decks,
    stakes,
    horizon,
    batch_size=16,
    max_consecutive_rejections=32,
    rewards=None,
    stochastic=True,
):
    from simulation.env import BatchEnv

    if batch_size <= 0:
        raise ValueError("rollout batch size must be positive")
    if not (len(seeds) == len(decks) == len(stakes)):
        raise ValueError("run configuration lengths differ")
    episodes = []
    for start in range(0, len(seeds), batch_size):
        stop = start + batch_size
        env = BatchEnv(
            seeds[start:stop],
            decks[start:stop],
            stakes[start:stop],
            horizon,
            max_consecutive_rejections,
            rewards,
        )
        episodes.extend(collect_episodes(model, env, stochastic))
    return episodes
