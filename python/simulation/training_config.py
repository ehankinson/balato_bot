import math
import os
from dataclasses import dataclass, field

import balatro_engine as engine
from simulation.encoder import ResourceLimits
from simulation.policy import ModelConfig
from simulation.reward import RewardConfig


@dataclass(frozen=True)
class TrainingConfig:
    iterations: int = 1000
    episodes_per_update: int = 4
    rollout_batch_size: int = 16
    ppo_epochs: int = 4
    minibatch_size: int = 16
    horizon: int = 4096
    max_consecutive_rejections: int = 32
    learning_rate: float = 3e-4
    clip_ratio: float = 0.2
    entropy_coefficient: float = 0.001
    gamma: float = 1.0
    gae_lambda: float = 0.95
    max_gradient_norm: float = 0.5
    seed: int = 0
    device: str = "cpu"
    decks: tuple[str, ...] = ("Red", "Magic", "Painted")
    stake: str = "White"
    model: ModelConfig = field(default_factory=ModelConfig)
    limits: ResourceLimits = field(default_factory=ResourceLimits)
    rewards: RewardConfig = field(default_factory=RewardConfig)
    checkpoint_path: str = "tmp/full_game.pt"
    resume_checkpoint: str | None = None
    resume_optimizer: bool = False
    evaluation_interval: int = 10
    evaluation_episodes: int = 4
    evaluation_seed: int = 1_000_000
    tensorboard_log_dir: str = "runs/full_game"

    def __post_init__(self):
        for name in (
            "iterations",
            "episodes_per_update",
            "rollout_batch_size",
            "ppo_epochs",
            "minibatch_size",
            "horizon",
            "max_consecutive_rejections",
            "evaluation_interval",
            "evaluation_episodes",
        ):
            if getattr(self, name) <= 0:
                raise ValueError(f"{name} must be positive")
        if (
            not 0 < self.gamma <= 1
            or not 0 < self.gae_lambda <= 1
            or not 0 < self.clip_ratio < 1
        ):
            raise ValueError("invalid gamma, gae_lambda or clip_ratio")
        if (
            self.learning_rate <= 0
            or self.max_gradient_norm <= 0
            or self.entropy_coefficient < 0
        ):
            raise ValueError("invalid optimizer configuration")
        if not all(
            math.isfinite(value)
            for value in (
                self.learning_rate,
                self.max_gradient_norm,
                self.entropy_coefficient,
                *self.rewards.__dict__.values(),
            )
        ):
            raise ValueError("optimizer and reward settings must be finite")
        if not 0 <= self.seed < 2**64 or not 0 <= self.evaluation_seed < 2**64:
            raise ValueError("seeds must be unsigned 64-bit integers")
        deck_names = {
            name
            for name, value in vars(engine.Deck).items()
            if isinstance(value, engine.Deck)
        }
        if not self.decks or any(deck not in deck_names for deck in self.decks):
            raise ValueError("unknown or empty engine deck mix")
        stake_names = {
            name
            for name, value in vars(engine.Stake).items()
            if isinstance(value, engine.Stake)
        }
        if self.stake not in stake_names:
            raise ValueError("unknown engine stake")

    @classmethod
    def from_env(cls):
        values = {}
        defaults = cls()
        for name in cls.__dataclass_fields__:
            if name in {"model", "limits", "rewards"}:
                continue
            raw = os.getenv(f"BALATRO_{name.upper()}")
            if raw is not None:
                default = getattr(defaults, name)
                values[name] = (
                    raw.lower() in {"1", "true", "yes"}
                    if isinstance(default, bool)
                    else (
                        int(raw)
                        if isinstance(default, int)
                        else float(raw)
                        if isinstance(default, float)
                        else tuple(item.strip() for item in raw.split(","))
                        if isinstance(default, tuple)
                        else raw
                    )
                )
        for name, config_type in (
            ("model", ModelConfig),
            ("limits", ResourceLimits),
            ("rewards", RewardConfig),
        ):
            options = {}
            for key, default in config_type().__dict__.items():
                raw = os.getenv(f"BALATRO_{name.upper()}_{key.upper()}")
                if raw is not None:
                    options[key] = type(default)(raw)
            values[name] = config_type(**options)
        return cls(**values)
