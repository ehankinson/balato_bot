from dataclasses import asdict
from math import isclose, isfinite


def write_iteration_metrics(
    writer,
    iteration,
    episodes,
    metrics,
    rollout_seconds,
    *,
    train_seconds,
    eval_seconds,
    save_seconds,
    total_seconds,
):
    """Log rollout-only throughput and iteration time through evaluation/save.

    Total includes iteration preparation and evaluation logging, but excludes
    this metrics write, console output, setup, and any final checkpoint save.
    """
    if not isfinite(rollout_seconds) or rollout_seconds <= 0:
        raise ValueError("rollout_seconds must be finite and positive")
    timings = {
        "train_seconds": train_seconds,
        "eval_seconds": eval_seconds,
        "save_seconds": save_seconds,
        "total_seconds": total_seconds,
    }
    for name, seconds in timings.items():
        if not isfinite(seconds) or seconds < 0:
            raise ValueError(f"{name} must be finite and non-negative")
    stage_seconds = rollout_seconds + train_seconds + eval_seconds + save_seconds
    if total_seconds < stage_seconds and not isclose(total_seconds, stage_seconds):
        raise ValueError("total_seconds must include rollout, train, eval, and save")
    accepted_commands = sum(episode.accepted_commands for episode in episodes)
    transitions = [
        transition for episode in episodes for transition in episode.transitions
    ]
    blind_scores = [
        transition.completed_blind_score
        for transition in transitions
        if transition.completed_blind_score is not None
    ]
    summary = {
        "rollout/completed_blinds": len(blind_scores),
        "rollout/average_blind_score": (
            sum(blind_scores) / len(blind_scores) if blind_scores else 0.0
        ),
        "rollout/episodes": len(episodes),
        "rollout/average_actions": accepted_commands / len(episodes),
        "rollout/accepted_commands": accepted_commands,
        "rollout/seconds": rollout_seconds,
        "rollout/actions_per_second": accepted_commands / rollout_seconds,
        "rollout/valid_action_rate": accepted_commands / len(transitions),
        "rollout/rejected_attempts": sum(
            not transition.accepted for transition in transitions
        ),
        "rollout/wins": sum(episode.end_reason == "win" for episode in episodes),
        "rollout/losses": sum(episode.end_reason == "loss" for episode in episodes),
        "rollout/truncations": sum(
            episode.transitions[-1].truncated for episode in episodes
        ),
        "rollout/ante_eight_rate": sum(
            episode.end_reason == "win" for episode in episodes
        )
        / len(episodes),
        "rollout/average_reward": sum(transition.reward for transition in transitions)
        / len(episodes),
        **{f"ppo/{key}": value for key, value in asdict(metrics).items()},
        **{f"timing/{key}": value for key, value in timings.items()},
    }
    for name, value in summary.items():
        writer.add_scalar(name, value, iteration)
    return summary


def format_iteration_summary(iteration, summary):
    """Keep console output compact; detailed metrics remain in TensorBoard."""
    blind_score = (
        f"{summary['rollout/average_blind_score']:,.1f}"
        if summary["rollout/completed_blinds"]
        else "N/A"
    )
    return (
        f"Iteration {iteration} | "
        f"Games: {summary['rollout/episodes']} | "
        f"Wins: {summary['rollout/wins']} | "
        f"Losses: {summary['rollout/losses']} | "
        f"Truncated: {summary['rollout/truncations']} | "
        f"Ante 8: {summary['rollout/ante_eight_rate']:.1%} | "
        f"Avg reward: {summary['rollout/average_reward']:.2f} | "
        f"Avg blind score: {blind_score} | "
        f"Actions/s: {summary['rollout/actions_per_second']:.1f} | "
        f"Valid: {summary['rollout/valid_action_rate']:.1%} | "
        f"Rejected: {summary['rollout/rejected_attempts']} | "
        f"Avg actions/game: {summary['rollout/average_actions']:.1f} | "
        f"Rollout: {summary['rollout/seconds']:.2f}s | "
        f"Train: {summary['timing/train_seconds']:.2f}s | "
        f"Eval: {summary['timing/eval_seconds']:.2f}s | "
        f"Save: {summary['timing/save_seconds']:.2f}s | "
        f"Total: {summary['timing/total_seconds']:.2f}s | "
        f"PPO updates: {summary['ppo/update_count']}"
    )


def write_evaluation_metrics(writer, iteration, evaluation):
    for name, value in asdict(evaluation).items():
        writer.add_scalar(f"evaluation/{name}", value, iteration)
    writer.flush()
