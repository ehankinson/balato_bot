from dataclasses import asdict


def write_iteration_metrics(writer, iteration, episodes, metrics):
    transitions = [
        transition for episode in episodes for transition in episode.transitions
    ]
    summary = {
        "rollout/accepted_commands": sum(
            episode.accepted_commands for episode in episodes
        ),
        "rollout/rejected_attempts": sum(
            not transition.accepted for transition in transitions
        ),
        "rollout/losses": sum(episode.end_reason == "loss" for episode in episodes),
        "rollout/truncations": sum(
            episode.transitions[-1].truncated for episode in episodes
        ),
        "rollout/ante_eight_rate": sum(
            bool(episode.final_observation["run.flags"][1]) for episode in episodes
        )
        / len(episodes),
        "rollout/average_reward": sum(transition.reward for transition in transitions)
        / len(episodes),
        **{f"ppo/{key}": value for key, value in asdict(metrics).items()},
    }
    for name, value in summary.items():
        writer.add_scalar(name, value, iteration)
    return summary


def write_evaluation_metrics(writer, iteration, evaluation):
    for name, value in asdict(evaluation).items():
        writer.add_scalar(f"evaluation/{name}", value, iteration)
    writer.flush()
