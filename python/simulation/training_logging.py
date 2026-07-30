from dataclasses import dataclass

import torch
from PIL import Image, ImageDraw, ImageFont
from torch.utils.tensorboard import SummaryWriter

from core.enums import HandAction, PokerHand
from simulation.ppo import PPOUpdateMetrics
from simulation.rollout import RolloutBatch, RolloutTimings
from simulation.training_evaluation import EvaluationMetrics


@dataclass(frozen=True, slots=True)
class IterationSummary:
    win_rate: float
    average_score: float
    average_progress: float
    average_hands_used_on_win: float
    one_hand_win_rate: float
    steps_per_second: float


def _chart_font(size: int):
    try:
        return ImageFont.truetype("DejaVuSans.ttf", size)
    except OSError:
        return ImageFont.load_default()


def _image_tensor(image: Image.Image) -> torch.Tensor:
    pixels = torch.frombuffer(bytearray(image.tobytes()), dtype=torch.uint8)
    return pixels.reshape(image.height, image.width, 3).permute(2, 0, 1).contiguous()


def played_hand_bar_chart(
    batch_counts: torch.Tensor,
    cumulative_counts: torch.Tensor,
) -> torch.Tensor:
    width = 1200
    row_height = 46
    top = 100
    height = top + len(PokerHand) * row_height + 55
    label_width = 215
    chart_width = 760
    background = (24, 24, 27)
    text_color = (235, 235, 240)
    grid_color = (62, 62, 70)
    batch_color = (244, 78, 166)
    cumulative_color = (91, 164, 245)

    image = Image.new("RGB", (width, height), background)
    draw = ImageDraw.Draw(image)
    title_font = _chart_font(24)
    label_font = _chart_font(16)
    value_font = _chart_font(14)
    draw.text((20, 14), "Poker hands played", fill=text_color, font=title_font)
    draw.rectangle((20, 55, 44, 67), fill=batch_color)
    draw.text((52, 50), "This update", fill=text_color, font=label_font)
    draw.rectangle((180, 55, 204, 67), fill=cumulative_color)
    draw.text((212, 50), "Cumulative", fill=text_color, font=label_font)

    for fraction in (0.0, 0.25, 0.5, 0.75, 1.0):
        x = label_width + int(chart_width * fraction)
        draw.line((x, top - 14, x, height - 35), fill=grid_color, width=1)
        draw.text(
            (x - 14, height - 30),
            f"{fraction:.0%}",
            fill=text_color,
            font=value_font,
        )

    batch_total = max(int(batch_counts.sum()), 1)
    cumulative_total = max(int(cumulative_counts.sum()), 1)
    for index, poker_hand in enumerate(PokerHand):
        y = top + index * row_height
        batch_count = int(batch_counts[index])
        cumulative_count = int(cumulative_counts[index])
        batch_share = batch_count / batch_total
        cumulative_share = cumulative_count / cumulative_total
        name = poker_hand.name.replace("_", " ").title()

        draw.text((20, y + 10), name, fill=text_color, font=label_font)
        if batch_count:
            draw.rectangle(
                (
                    label_width,
                    y + 5,
                    label_width + int(chart_width * batch_share),
                    y + 18,
                ),
                fill=batch_color,
            )
        if cumulative_count:
            draw.rectangle(
                (
                    label_width,
                    y + 23,
                    label_width + int(chart_width * cumulative_share),
                    y + 36,
                ),
                fill=cumulative_color,
            )
        draw.text(
            (label_width + chart_width + 12, y + 3),
            f"{batch_count:,} ({batch_share:.1%})",
            fill=batch_color,
            font=value_font,
        )
        draw.text(
            (label_width + chart_width + 12, y + 21),
            f"{cumulative_count:,} ({cumulative_share:.1%})",
            fill=cumulative_color,
            font=value_font,
        )

    return _image_tensor(image)


def write_iteration_metrics(
    writer: SummaryWriter,
    iteration: int,
    batch: RolloutBatch,
    rollout_timings: RolloutTimings,
    ppo_metrics: PPOUpdateMetrics,
    iteration_seconds: float,
    cumulative_hand_counts: torch.Tensor,
    chart_interval: int,
    profile: bool,
) -> IterationSummary:
    completed_episodes = max(batch.completed_episodes, 1)
    wins = int(batch.terminal_wins.sum())
    win_rate = wins / completed_episodes
    average_score = batch.terminal_scores.mean().item()
    average_progress = (
        (batch.terminal_scores / batch.terminal_targets)
        .clamp(max=1.0)
        .mean()
        .item()
    )

    winning_hands_used = batch.terminal_hands_used[batch.terminal_wins]
    average_hands_used_on_win = (
        winning_hands_used.double().mean().item() if wins else 0.0
    )
    one_hand_wins = int((winning_hands_used == 1).sum())
    one_hand_win_rate = one_hand_wins / completed_episodes
    one_hand_share_of_wins = one_hand_wins / max(wins, 1)

    losing_scores = batch.terminal_scores[~batch.terminal_wins]
    average_losing_score = (
        losing_scores.mean().item() if len(losing_scores) else 0.0
    )

    discard_mask = batch.modes == HandAction.DISCARD
    play_mask = batch.modes == HandAction.PLAY_HAND
    discards = int(discard_mask.sum())
    plays = int(play_mask.sum())
    discarded_cards = int(batch.counts[discard_mask].sum())
    played_cards = int(batch.counts[play_mask].sum())
    played_hand_types = batch.played_hand_types[play_mask]
    played_hand_types = played_hand_types[
        played_hand_types >= int(PokerHand.HIGH_CARD)
    ]
    batch_hand_counts = torch.bincount(
        played_hand_types - int(PokerHand.HIGH_CARD),
        minlength=len(PokerHand),
    )
    cumulative_hand_counts += batch_hand_counts

    steps_per_second = batch.total_steps / max(iteration_seconds, 1e-9)
    average_episode_reward = batch.rewards.sum().item() / completed_episodes
    metrics = {
        "performance/win_rate": win_rate,
        "performance/average_score": average_score,
        "performance/average_progress": average_progress,
        "performance/average_losing_score": average_losing_score,
        "performance/average_hands_used_on_win": average_hands_used_on_win,
        "performance/one_hand_win_rate": one_hand_win_rate,
        "performance/one_hand_share_of_wins": one_hand_share_of_wins,
        "reward/average_episode_reward": average_episode_reward,
        "loss/policy": ppo_metrics.policy_loss,
        "loss/value": ppo_metrics.value_loss,
        "loss/entropy": ppo_metrics.entropy_loss,
        "behavior/discards_per_episode": discards / completed_episodes,
        "behavior/cards_per_discard": discarded_cards / max(discards, 1),
        "behavior/cards_per_play": played_cards / max(plays, 1),
        "throughput/steps": batch.total_steps,
        "throughput/steps_per_second": steps_per_second,
        "throughput/inference_batches": rollout_timings.inference_batches,
        "throughput/optimizer_updates": ppo_metrics.update_count,
        "timing/iteration_seconds": iteration_seconds,
    }
    for name, value in metrics.items():
        writer.add_scalar(name, value, iteration)

    batch_hand_total = max(int(batch_hand_counts.sum()), 1)
    cumulative_hand_total = max(int(cumulative_hand_counts.sum()), 1)
    for index, poker_hand in enumerate(PokerHand):
        tag_name = poker_hand.name.lower()
        writer.add_scalar(
            f"played_hands/batch_share/{tag_name}",
            int(batch_hand_counts[index]) / batch_hand_total,
            iteration,
        )
        writer.add_scalar(
            f"played_hands/cumulative_share/{tag_name}",
            int(cumulative_hand_counts[index]) / cumulative_hand_total,
            iteration,
        )

    if iteration % chart_interval == 0:
        writer.add_image(
            "played_hands/bar_chart",
            played_hand_bar_chart(batch_hand_counts, cumulative_hand_counts),
            iteration,
        )

    if profile:
        milliseconds_per_step = 1000.0 / batch.total_steps
        profile_metrics = {
            "timing/rollout_seconds": rollout_timings.rollout_seconds,
            "timing/gpu_inference_seconds": rollout_timings.inference_seconds,
            "timing/gae_seconds": ppo_metrics.gae_seconds,
            "timing/transfer_seconds": ppo_metrics.transfer_seconds,
            "timing/ppo_seconds": ppo_metrics.update_seconds,
            "timing_ms_per_step/scoring": (
                batch.score_seconds * milliseconds_per_step
            ),
            "timing_ms_per_step/discard_table": (
                batch.discard_seconds * milliseconds_per_step
            ),
            "timing_ms_per_step/encoding": (
                batch.encoding_seconds * milliseconds_per_step
            ),
            "timing_ms_per_step/environment": (
                batch.environment_seconds * milliseconds_per_step
            ),
        }
        for name, value in profile_metrics.items():
            writer.add_scalar(name, value, iteration)

    return IterationSummary(
        win_rate=win_rate,
        average_score=average_score,
        average_progress=average_progress,
        average_hands_used_on_win=average_hands_used_on_win,
        one_hand_win_rate=one_hand_win_rate,
        steps_per_second=steps_per_second,
    )


def write_evaluation_metrics(
    writer: SummaryWriter,
    iteration: int,
    evaluation: EvaluationMetrics,
    evaluation_seconds: float,
) -> None:
    metrics = {
        "evaluation/win_rate": evaluation.win_rate,
        "evaluation/average_score": evaluation.average_score,
        "evaluation/average_progress": evaluation.average_progress,
        "evaluation/average_hands_used_on_win": (
            evaluation.average_hands_used_on_win
        ),
        "evaluation/one_hand_win_rate": evaluation.one_hand_win_rate,
        "evaluation/seconds": evaluation_seconds,
    }
    for target_metrics in evaluation.by_target:
        prefix = f"evaluation/by_target/{target_metrics.target}"
        metrics[f"{prefix}/win_rate"] = target_metrics.win_rate
        metrics[f"{prefix}/average_score"] = target_metrics.average_score
        metrics[f"{prefix}/average_progress"] = target_metrics.average_progress
        metrics[f"{prefix}/average_hands_used_on_win"] = (
            target_metrics.average_hands_used_on_win
        )
        metrics[f"{prefix}/one_hand_win_rate"] = (
            target_metrics.one_hand_win_rate
        )
    for name, value in metrics.items():
        writer.add_scalar(name, value, iteration)
    writer.flush()


def format_iteration(iteration: int, summary: IterationSummary) -> str:
    return (
        f"iter {iteration:4d}: win={summary.win_rate:.1%} "
        f"score={summary.average_score:.1f} "
        f"progress={summary.average_progress:.1%} "
        f"hands/win={summary.average_hands_used_on_win:.2f} "
        f"one-hand={summary.one_hand_win_rate:.1%} "
        f"steps/s={summary.steps_per_second:.0f}"
    )


def format_evaluation(
    completed_iterations: int,
    evaluation: EvaluationMetrics,
    checkpoint_path: str,
    best_checkpoint_path: str | None,
) -> str:
    target_results = " ".join(
        f"{target.target}={target.win_rate:.1%}"
        for target in evaluation.by_target
    )
    best_text = (
        f" best={best_checkpoint_path}" if best_checkpoint_path is not None else ""
    )
    return (
        f"eval {completed_iterations:4d}: win={evaluation.win_rate:.1%} "
        f"[{target_results}] score={evaluation.average_score:.1f} "
        f"progress={evaluation.average_progress:.1%} "
        f"hands/win={evaluation.average_hands_used_on_win:.2f} "
        f"one-hand={evaluation.one_hand_win_rate:.1%} "
        f"saved={checkpoint_path}{best_text}"
    )
