"""Rust-owned single/batch games with shared episode accounting."""

from dataclasses import dataclass
from time import perf_counter

import numpy as np

import balatro_engine as engine
from simulation.encoder import unbatch_observations
from simulation.reward import RewardConfig, transition_reward


@dataclass(frozen=True)
class StepResult:
    observation: dict
    reward: float
    accepted: bool
    terminated: bool
    truncated: bool
    end_reason: str | None
    error: str | None = None


class EpisodeState:
    def __init__(self, horizon, rejection_limit, rewards):
        if horizon <= 0 or rejection_limit <= 0:
            raise ValueError("horizon and rejection limit must be positive")
        self.horizon, self.rejection_limit, self.rewards = (
            horizon,
            rejection_limit,
            rewards,
        )
        self.done = False
        self.accepted_commands = self.consecutive_rejections = 0
        self.observation = None

    def reset(self, observation):
        self.observation = observation
        self.done = False
        self.accepted_commands = self.consecutive_rejections = 0

    def finish(self, observation, accepted, error):
        before = self.observation
        if accepted:
            self.accepted_commands += 1
            self.consecutive_rejections = 0
        else:
            self.consecutive_rejections += 1
        self.observation = observation
        terminated = bool(observation["run.flags"][0])
        reason = (
            "loss"
            if terminated
            else (
                "horizon"
                if self.accepted_commands >= self.horizon
                else (
                    "rejection_limit"
                    if self.consecutive_rejections >= self.rejection_limit
                    else None
                )
            )
        )
        truncated = reason is not None and not terminated
        self.done = terminated or truncated
        return StepResult(
            observation,
            transition_reward(before, observation, accepted, self.rewards),
            accepted,
            terminated,
            truncated,
            reason,
            error,
        )


class GameEnv:
    def __init__(
        self,
        horizon=4096,
        max_consecutive_rejections=32,
        deck=engine.Deck.Red,
        stake=engine.Stake.White,
        rewards=None,
    ):
        self.state = EpisodeState(
            horizon,
            max_consecutive_rejections,
            rewards if rewards is not None else RewardConfig(),
        )
        self.deck, self.stake = deck, stake
        self.game = None

    @property
    def observation(self):
        return self.state.observation

    @property
    def done(self):
        return self.state.done

    @property
    def accepted_commands(self):
        return self.state.accepted_commands

    def reset(self, seed):
        self.game = engine.new(seed, deck=self.deck, stake=self.stake)
        self.state.reset(self.game.observe())
        return self.observation

    def step(self, command):
        if self.game is None or self.done:
            raise RuntimeError("reset the environment before stepping")
        error = None
        try:
            command.apply(self.game)
            accepted = True
        except ValueError as exc:
            accepted, error = False, str(exc)
        return self.state.finish(self.game.observe(), accepted, error)


@dataclass(frozen=True)
class BatchStep:
    results: tuple[StepResult | None, ...]
    timings: dict | None


class BatchEnv:
    def __init__(
        self,
        seeds,
        decks=None,
        stakes=None,
        horizon=4096,
        max_consecutive_rejections=32,
        rewards=None,
    ):
        self.games = engine.new_batch(
            np.array(seeds, dtype=np.uint64), decks=decks, stakes=stakes
        )
        rewards = rewards if rewards is not None else RewardConfig()
        self.states = [
            EpisodeState(horizon, max_consecutive_rejections, rewards) for _ in seeds
        ]
        self.observations = self.games.observe()
        self.snapshots = unbatch_observations(self.observations)
        for state, observation in zip(self.states, self.snapshots):
            state.reset(observation)

    @property
    def active(self):
        return np.array([not state.done for state in self.states], dtype=np.bool_)

    def reset(self, slots, seeds):
        self.games.reset(
            np.array(slots, dtype=np.int32), np.array(seeds, dtype=np.uint64)
        )
        self.observations = self.games.observe()
        self.snapshots = unbatch_observations(self.observations)
        for slot in slots:
            self.states[slot].reset(self.snapshots[slot])
        return self.observations

    def step(self, commands, profile=False):
        if any(
            state.done and kind != -1
            for state, kind in zip(self.states, commands[:, 0])
        ):
            raise RuntimeError("reset completed slots before stepping them")
        started = perf_counter() if profile else 0.0
        result = self.games.step(commands, profile=profile)
        boundary_finished = perf_counter() if profile else 0.0
        self.observations = result["observations"]
        self.snapshots = unbatch_observations(self.observations)
        results = []
        for index, state in enumerate(self.states):
            results.append(
                state.finish(
                    self.snapshots[index],
                    bool(result["accepted"][index]),
                    result["errors"][index],
                )
                if result["attempted"][index]
                else None
            )
        timings = result.get("timings")
        if profile:
            native_seconds = sum(timings.values())
            timings["binding_overhead_seconds"] = max(
                0.0, boundary_finished - started - native_seconds
            )
            timings["python_bookkeeping_seconds"] = perf_counter() - boundary_finished
        return BatchStep(tuple(results), timings)
