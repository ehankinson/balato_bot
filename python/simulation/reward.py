"""Training rewards from public engine transitions; no scoring or pricing rules."""

from dataclasses import dataclass


@dataclass(frozen=True)
class RewardConfig:
    cleared_blind: float = 1.0
    ante_eight: float = 10.0
    loss: float = -1.0
    rejected_attempt: float = -0.1


def transition_reward(before, after, accepted, config=None):
    config = config if config is not None else RewardConfig()
    if not accepted:
        return config.rejected_attempt
    rounds = int(after["run.counts"][1]) - int(before["run.counts"][1])
    success = bool(after["run.flags"][1]) and not bool(before["run.flags"][1])
    loss = bool(after["run.flags"][0]) and not bool(before["run.flags"][0])
    return (
        rounds * config.cleared_blind + success * config.ante_eight + loss * config.loss
    )
