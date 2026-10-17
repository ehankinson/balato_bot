import os
from dataclasses import asdict, dataclass
from pathlib import Path

import torch

import balatro_engine as engine
from simulation.encoder import SCHEMA_VERSION, ResourceLimits
from simulation.policy import GamePolicy, ModelConfig
from simulation.rollout import collect_runs

ARCHITECTURE = "shared_attention_v1"


@dataclass(frozen=True)
class EvaluationMetrics:
    episodes: int
    ante_eight_rate: float
    average_rounds: float
    losses: int
    truncations: int
    rejected_attempts: int


def evaluate_model(
    model,
    episodes,
    seed,
    horizon,
    decks=(engine.Deck.Red, engine.Deck.Magic, engine.Deck.Painted),
    batch_size=16,
    stake=engine.Stake.White,
    **env_options,
):
    if episodes <= 0:
        raise ValueError("evaluation episodes must be positive")
    if not decks:
        raise ValueError("evaluation deck mix cannot be empty")
    seeds = [seed + index for index in range(episodes)]
    results = collect_runs(
        model,
        seeds,
        [decks[index % len(decks)] for index in range(episodes)],
        [stake] * episodes,
        horizon,
        batch_size,
        stochastic=False,
        **env_options,
    )
    return EvaluationMetrics(
        episodes,
        sum(result.end_reason == "win" for result in results) / episodes,
        sum(int(result.final_observation["run.counts"][1]) for result in results)
        / episodes,
        sum(result.end_reason == "loss" for result in results),
        sum(result.transitions[-1].truncated for result in results),
        sum(
            not transition.accepted
            for result in results
            for transition in result.transitions
        ),
    )


@dataclass(frozen=True)
class CheckpointState:
    iterations_completed: int
    evaluation: EvaluationMetrics | None


def _read_checkpoint(path, device):
    checkpoint = torch.load(path, map_location=device, weights_only=True)
    required = {
        "model_config",
        "resource_limits",
        "training_config",
        "state_dict",
        "iterations_completed",
    }
    if (
        checkpoint.get("schema_version") != SCHEMA_VERSION
        or checkpoint.get("architecture") != ARCHITECTURE
        or not required.issubset(checkpoint)
    ):
        raise ValueError(
            "incompatible checkpoint: requires schema 1 and shared_attention_v1; old blind weights cannot be loaded"
        )
    return checkpoint


def save_checkpoint(path, model, optimizer, iterations_completed, evaluation, config):
    path = Path(path).absolute()
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    torch.save(
        {
            "schema_version": SCHEMA_VERSION,
            "architecture": ARCHITECTURE,
            "model_config": asdict(model.config),
            "resource_limits": asdict(model.limits),
            "training_config": asdict(config),
            "state_dict": {
                key: value.detach().cpu() for key, value in model.state_dict().items()
            },
            "optimizer_state_dict": optimizer.state_dict(),
            "iterations_completed": iterations_completed,
            "evaluation": asdict(evaluation) if evaluation else None,
        },
        temporary,
    )
    os.replace(temporary, path)


def load_checkpoint(path, model, optimizer=None, load_optimizer=False):
    checkpoint = _read_checkpoint(path, next(model.parameters()).device)
    if checkpoint["model_config"] != asdict(model.config) or checkpoint[
        "resource_limits"
    ] != asdict(model.limits):
        raise ValueError(
            "checkpoint model configuration or resource limits do not match"
        )
    if load_optimizer:
        if optimizer is None:
            raise ValueError("an optimizer is required to resume optimizer state")
        if "optimizer_state_dict" not in checkpoint:
            raise ValueError("checkpoint has no optimizer state")
    model.load_state_dict(checkpoint["state_dict"], strict=True)
    if load_optimizer:
        optimizer.load_state_dict(checkpoint["optimizer_state_dict"])
    return CheckpointState(
        checkpoint["iterations_completed"],
        EvaluationMetrics(**checkpoint["evaluation"])
        if checkpoint.get("evaluation")
        else None,
    )


def load_policy(path, device="cpu"):
    checkpoint = _read_checkpoint(path, device)
    model = GamePolicy(
        ModelConfig(**checkpoint["model_config"]),
        ResourceLimits(**checkpoint["resource_limits"]),
    ).to(device)
    model.load_state_dict(checkpoint["state_dict"], strict=True)
    model.eval()
    return model
