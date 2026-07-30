import os
import random
from dataclasses import asdict, dataclass
from typing import Any

import torch
from torch.optim import Optimizer

from calculation.poker_discards import generate_discard_table
from calculation.score import get_best_scoring_hand
from core.models import Deck, GameState
from simulation.blind_trainer import BlindModel
from simulation.decoder import build_mask, model_decoder
from simulation.encoder import encode_game_state
from simulation.training_config import TrainingConfig


@dataclass(frozen=True, slots=True)
class TargetEvaluationMetrics:
    target: int
    episodes: int
    wins: int
    win_rate: float
    average_score: float
    average_progress: float
    average_hands_used_on_win: float
    one_hand_win_rate: float


@dataclass(frozen=True, slots=True)
class EvaluationMetrics:
    episodes: int
    wins: int
    win_rate: float
    average_score: float
    average_progress: float
    average_hands_used_on_win: float
    one_hand_win_rate: float
    by_target: tuple[TargetEvaluationMetrics, ...] = ()

    @classmethod
    def from_dict(cls, values: dict[str, Any] | None) -> "EvaluationMetrics | None":
        if values is None:
            return None
        target_values = values.get("by_target", ())
        by_target = tuple(
            TargetEvaluationMetrics(**target_value)
            for target_value in target_values
        )
        return cls(
            episodes=int(values["episodes"]),
            wins=int(values["wins"]),
            win_rate=float(values["win_rate"]),
            average_score=float(values["average_score"]),
            average_progress=float(values["average_progress"]),
            average_hands_used_on_win=float(
                values["average_hands_used_on_win"]
            ),
            one_hand_win_rate=float(values["one_hand_win_rate"]),
            by_target=by_target,
        )


@dataclass(frozen=True, slots=True)
class CheckpointState:
    iterations_completed: int
    evaluation: EvaluationMetrics | None


@dataclass(slots=True)
class _EvaluationAccumulator:
    episodes: int = 0
    wins: int = 0
    total_score: float = 0.0
    total_progress: float = 0.0
    winning_hands_used: int = 0
    one_hand_wins: int = 0

    def add(self, game_state: GameState, target: int, has_won: bool) -> None:
        self.episodes += 1
        self.total_score += game_state.current_score
        self.total_progress += min(game_state.current_score / target, 1.0)
        if has_won:
            self.wins += 1
            self.winning_hands_used += game_state.hands_played
            if game_state.hands_played == 1:
                self.one_hand_wins += 1

    def metrics(self, target: int | None = None):
        common = {
            "episodes": self.episodes,
            "wins": self.wins,
            "win_rate": self.wins / self.episodes,
            "average_score": self.total_score / self.episodes,
            "average_progress": self.total_progress / self.episodes,
            "average_hands_used_on_win": (
                self.winning_hands_used / self.wins if self.wins else 0.0
            ),
            "one_hand_win_rate": self.one_hand_wins / self.episodes,
        }
        if target is None:
            return common
        return TargetEvaluationMetrics(target=target, **common)


def evaluate_model(
    model: BlindModel,
    device: torch.device,
    episodes: int,
    seed: int,
    max_steps: int,
    targets: tuple[int, ...],
) -> EvaluationMetrics:
    """Evaluate greedy actions on a fixed, reproducible set of blinds."""
    if episodes <= 0:
        raise ValueError("evaluation episodes must be positive")
    if not targets:
        raise ValueError("at least one evaluation target is required")

    was_training = model.training
    random_state = random.getstate()
    overall = _EvaluationAccumulator()
    per_target = {
        target: _EvaluationAccumulator() for target in dict.fromkeys(targets)
    }

    model.eval()
    try:
        for episode in range(episodes):
            random.seed(seed + episode)
            target = targets[episode % len(targets)]
            deck = Deck()
            game_state = GameState(score_to_beat=target)
            hand = deck.draw(game_state.hand_size)
            has_won = False

            for _step in range(max_steps):
                best_hand = get_best_scoring_hand(hand, [], game_state)
                discard_table = generate_discard_table(deck, hand)
                observation = encode_game_state(
                    hand,
                    game_state,
                    best_hand,
                    discard_table,
                )
                masks = build_mask(game_state, hand, device)

                with torch.inference_mode():
                    outputs = model(observation.unsqueeze(0).to(device))
                    mode, count, card_indices, _, _ = model_decoder(
                        outputs,
                        masks,
                        device,
                        stochastic=False,
                    )

                selected_cards = [hand[index] for index in card_indices[:count]]
                for card in selected_cards:
                    hand.remove(card)

                has_won = game_state.execute_hand_action(
                    mode,
                    selected_cards,
                    hand,
                    deck,
                )
                if has_won or game_state.hands == 0:
                    break

            overall.add(game_state, target, has_won)
            per_target[target].add(game_state, target, has_won)
    finally:
        random.setstate(random_state)
        model.train(was_training)

    return EvaluationMetrics(
        **overall.metrics(),
        by_target=tuple(
            per_target[target].metrics(target)
            for target in dict.fromkeys(targets)
        ),
    )


def cpu_state_dict(model: BlindModel) -> dict[str, torch.Tensor]:
    return {
        name: parameter.detach().cpu() for name, parameter in model.state_dict().items()
    }


def save_checkpoint(
    path: str,
    model: BlindModel,
    optimizer: Optimizer,
    input_size: int,
    hidden_size: int,
    iterations_completed: int,
    evaluation: EvaluationMetrics | None,
    config: TrainingConfig,
) -> None:
    """Atomically save model, optimizer, evaluation, and training metadata."""
    absolute_path = os.path.abspath(path)
    os.makedirs(os.path.dirname(absolute_path), exist_ok=True)
    temporary_path = f"{absolute_path}.tmp"
    torch.save(
        {
            "state_dict": cpu_state_dict(model),
            "optimizer_state_dict": optimizer.state_dict(),
            "input_size": input_size,
            "hidden_size": hidden_size,
            "architecture_version": 2,
            "iterations_completed": iterations_completed,
            "evaluation": asdict(evaluation) if evaluation is not None else None,
            "training_config": asdict(config),
        },
        temporary_path,
    )
    os.replace(temporary_path, absolute_path)


def load_checkpoint(
    path: str,
    model: BlindModel,
    optimizer: Optimizer | None,
    device: torch.device,
    load_optimizer: bool,
) -> CheckpointState:
    checkpoint = torch.load(path, map_location=device)
    model.load_state_dict(checkpoint["state_dict"])
    if (
        load_optimizer
        and optimizer is not None
        and "optimizer_state_dict" in checkpoint
    ):
        optimizer.load_state_dict(checkpoint["optimizer_state_dict"])
    return CheckpointState(
        iterations_completed=int(checkpoint.get("iterations_completed", 0)),
        evaluation=EvaluationMetrics.from_dict(checkpoint.get("evaluation")),
    )


def checkpoint_state(path: str) -> CheckpointState | None:
    if not os.path.exists(path):
        return None
    checkpoint = torch.load(path, map_location="cpu")
    return CheckpointState(
        iterations_completed=int(checkpoint.get("iterations_completed", 0)),
        evaluation=EvaluationMetrics.from_dict(checkpoint.get("evaluation")),
    )


def evaluation_is_better(
    candidate: EvaluationMetrics,
    incumbent: EvaluationMetrics | None,
) -> bool:
    return (
        incumbent is None
        or candidate.win_rate > incumbent.win_rate
        or (
            candidate.win_rate == incumbent.win_rate
            and candidate.average_hands_used_on_win
            < incumbent.average_hands_used_on_win
        )
    )
