import os
import secrets
from dataclasses import dataclass
from time import strftime

import torch


def _boolean_env(name: str, default: bool = False) -> bool:
    raw = os.getenv(name)
    if raw is None:
        return default
    return raw.lower() not in {"0", "false", "no", "off"}


def parse_training_targets(raw: str) -> tuple[int, ...]:
    """Expand `300:1,450:4` into a deterministic weighted target cycle."""
    targets: list[int] = []
    for item in raw.split(","):
        value = item.strip()
        if not value:
            continue
        target_text, separator, weight_text = value.partition(":")
        target = int(target_text)
        weight = int(weight_text) if separator else 1
        if target <= 0:
            raise ValueError("training targets must be positive")
        if weight <= 0:
            raise ValueError("training target weights must be positive")
        targets.extend([target] * weight)
    if not targets:
        raise ValueError("at least one training target is required")
    return tuple(targets)


def parse_evaluation_targets(raw: str) -> tuple[int, ...]:
    targets = tuple(int(value.strip()) for value in raw.split(",") if value.strip())
    if not targets or any(target <= 0 for target in targets):
        raise ValueError("evaluation targets must contain positive integers")
    return targets


def _default_device() -> str:
    if torch.cuda.is_available():
        return "cuda"
    if torch.backends.mps.is_available():
        return "mps"
    return "cpu"


@dataclass(frozen=True, slots=True)
class TrainingConfig:
    iterations: int
    episodes_per_update: int
    ppo_epochs: int
    minibatch_size: int
    max_steps_per_episode: int
    clip_ratio: float
    learning_rate: float
    entropy_coefficient: float
    gamma: float
    gae_lambda: float
    max_gradient_norm: float
    hidden_size: int
    rollout_workers: int
    device: str
    seed: int
    training_targets: tuple[int, ...]
    evaluation_targets: tuple[int, ...]
    evaluation_interval: int
    evaluation_episodes: int
    evaluation_seed: int
    checkpoint_path: str
    best_checkpoint_path: str
    resume_checkpoint: str | None
    resume_optimizer: bool
    tensorboard_log_dir: str
    profile: bool
    console_log_interval: int
    chart_interval: int

    @classmethod
    def from_env(cls) -> "TrainingConfig":
        python_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        checkpoint_directory = os.getenv(
            "BALATRO_CHECKPOINT_DIR",
            os.path.join(python_root, "tmp"),
        )
        checkpoint_path = os.getenv(
            "BALATRO_CHECKPOINT",
            os.path.join(checkpoint_directory, "last_trained.pt"),
        )
        best_checkpoint_path = os.getenv(
            "BALATRO_BEST_CHECKPOINT",
            os.path.join(checkpoint_directory, "best_model.pt"),
        )

        resume_checkpoint = os.getenv("BALATRO_RESUME_CHECKPOINT")
        if resume_checkpoint == "best":
            resume_checkpoint = best_checkpoint_path
        elif resume_checkpoint == "latest":
            resume_checkpoint = checkpoint_path

        seed_text = os.getenv("BALATRO_SEED")
        seed = int(seed_text) if seed_text is not None else secrets.randbits(63)
        run_name = os.getenv(
            "BALATRO_RUN_NAME",
            f"{strftime('%Y%m%d-%H%M%S')}_{seed}",
        )
        log_root = os.getenv(
            "BALATRO_LOG_DIR",
            os.path.join(python_root, "runs"),
        )

        config = cls(
            iterations=int(os.getenv("BALATRO_ITERATIONS", "2000")),
            episodes_per_update=int(os.getenv("BALATRO_EPISODES_PER_UPDATE", "1024")),
            ppo_epochs=int(os.getenv("BALATRO_PPO_EPOCHS", "4")),
            minibatch_size=int(os.getenv("BALATRO_MINIBATCH_SIZE", "512")),
            max_steps_per_episode=int(os.getenv("BALATRO_MAX_STEPS", "20")),
            clip_ratio=float(os.getenv("BALATRO_CLIP_RATIO", "0.2")),
            learning_rate=float(os.getenv("BALATRO_LEARNING_RATE", "3e-4")),
            entropy_coefficient=float(
                os.getenv("BALATRO_ENTROPY_COEFFICIENT", "0.001")
            ),
            gamma=float(os.getenv("BALATRO_GAMMA", "0.99")),
            gae_lambda=float(os.getenv("BALATRO_GAE_LAMBDA", "0.95")),
            max_gradient_norm=float(os.getenv("BALATRO_MAX_GRADIENT_NORM", "0.5")),
            hidden_size=int(os.getenv("BALATRO_HIDDEN_SIZE", "256")),
            rollout_workers=int(os.getenv("BALATRO_ROLLOUT_WORKERS", "15")),
            device=os.getenv("BALATRO_DEVICE", _default_device()),
            seed=seed,
            training_targets=parse_training_targets(
                os.getenv("BALATRO_TRAIN_TARGETS", "300,450")
            ),
            evaluation_targets=parse_evaluation_targets(
                os.getenv("BALATRO_EVAL_TARGETS", "300,450")
            ),
            evaluation_interval=int(os.getenv("BALATRO_EVAL_INTERVAL", "100")),
            evaluation_episodes=int(os.getenv("BALATRO_EVAL_EPISODES", "500")),
            evaluation_seed=int(os.getenv("BALATRO_EVAL_SEED", "20260728")),
            checkpoint_path=checkpoint_path,
            best_checkpoint_path=best_checkpoint_path,
            resume_checkpoint=resume_checkpoint,
            resume_optimizer=_boolean_env(
                "BALATRO_RESUME_OPTIMIZER",
                default=False,
            ),
            tensorboard_log_dir=os.path.join(log_root, run_name),
            profile=_boolean_env("BALATRO_PROFILE", default=False),
            console_log_interval=int(os.getenv("BALATRO_CONSOLE_LOG_INTERVAL", "5")),
            chart_interval=int(os.getenv("BALATRO_CHART_INTERVAL", "5")),
        )
        config.validate()
        return config

    def validate(self) -> None:
        positive_integer_fields = {
            "iterations": self.iterations,
            "episodes_per_update": self.episodes_per_update,
            "ppo_epochs": self.ppo_epochs,
            "minibatch_size": self.minibatch_size,
            "max_steps_per_episode": self.max_steps_per_episode,
            "hidden_size": self.hidden_size,
            "rollout_workers": self.rollout_workers,
            "evaluation_interval": self.evaluation_interval,
            "evaluation_episodes": self.evaluation_episodes,
            "console_log_interval": self.console_log_interval,
            "chart_interval": self.chart_interval,
        }
        for name, value in positive_integer_fields.items():
            if value <= 0:
                raise ValueError(f"{name} must be positive")
        if not 0.0 < self.clip_ratio < 1.0:
            raise ValueError("clip_ratio must be between zero and one")
        if self.learning_rate <= 0.0:
            raise ValueError("learning_rate must be positive")
        if self.entropy_coefficient < 0.0:
            raise ValueError("entropy_coefficient cannot be negative")
        if not 0.0 < self.gamma <= 1.0:
            raise ValueError("gamma must be in (0, 1]")
        if not 0.0 < self.gae_lambda <= 1.0:
            raise ValueError("gae_lambda must be in (0, 1]")
        if self.max_gradient_norm <= 0.0:
            raise ValueError("max_gradient_norm must be positive")
        if self.device.startswith("cuda") and not torch.cuda.is_available():
            raise RuntimeError("BALATRO_DEVICE requests CUDA, but CUDA is unavailable")

    @property
    def effective_rollout_workers(self) -> int:
        return min(self.rollout_workers, self.episodes_per_update)

    def worker_targets(self) -> list[int]:
        """Assign a deterministic weighted target mix across workers."""
        return [
            self.training_targets[index % len(self.training_targets)]
            for index in range(self.effective_rollout_workers)
        ]

    def summary(self) -> str:
        training_mix = ",".join(str(target) for target in self.training_targets)
        evaluation_mix = ",".join(str(target) for target in self.evaluation_targets)
        return (
            f"device={self.device} workers={self.effective_rollout_workers} "
            f"episodes/update={self.episodes_per_update} "
            f"minibatch={self.minibatch_size} seed={self.seed} "
            f"lr={self.learning_rate:g} train_targets={training_mix} "
            f"eval_targets={evaluation_mix} "
            f"eval_every={self.evaluation_interval} "
            f"eval_episodes={self.evaluation_episodes} "
            f"profile={str(self.profile).lower()} "
            f"tensorboard={self.tensorboard_log_dir}"
        )
