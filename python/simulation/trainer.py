import torch
from torch.optim import Adam
from torch.utils.tensorboard import SummaryWriter

import balatro_engine as engine
from simulation.policy import GamePolicy
from simulation.ppo import update_policy
from simulation.rollout import collect_runs
from simulation.training_config import TrainingConfig
from simulation.training_evaluation import (
    evaluate_model,
    load_checkpoint,
    save_checkpoint,
)
from simulation.training_logging import (
    write_evaluation_metrics,
    write_iteration_metrics,
)


def train(config):
    torch.manual_seed(config.seed)
    model = GamePolicy(config.model, config.limits).to(config.device)
    optimizer = Adam(model.parameters(), lr=config.learning_rate)
    start, evaluation = 0, None
    if config.resume_checkpoint:
        state = load_checkpoint(
            config.resume_checkpoint, model, optimizer, config.resume_optimizer
        )
        start, evaluation = state.iterations_completed, state.evaluation
        for group in optimizer.param_groups:
            group["lr"] = config.learning_rate
    env_options = {
        "max_consecutive_rejections": config.max_consecutive_rejections,
        "stake": getattr(engine.Stake, config.stake),
        "rewards": config.rewards,
    }
    completed = start
    with SummaryWriter(config.tensorboard_log_dir) as writer:
        try:
            for iteration in range(start, start + config.iterations):
                seeds = [
                    config.seed + iteration * config.episodes_per_update + index
                    for index in range(config.episodes_per_update)
                ]
                decks = [
                    getattr(
                        engine.Deck,
                        config.decks[
                            (iteration * config.episodes_per_update + index)
                            % len(config.decks)
                        ],
                    )
                    for index in range(config.episodes_per_update)
                ]
                episodes = collect_runs(
                    model,
                    seeds,
                    decks,
                    [env_options["stake"]] * len(seeds),
                    config.horizon,
                    config.rollout_batch_size,
                    config.max_consecutive_rejections,
                    config.rewards,
                )
                metrics = update_policy(model, optimizer, episodes, config)
                completed = iteration + 1
                print(
                    completed,
                    write_iteration_metrics(writer, completed, episodes, metrics),
                )
                if completed % config.evaluation_interval == 0:
                    evaluation = evaluate_model(
                        model,
                        config.evaluation_episodes,
                        config.evaluation_seed,
                        config.horizon,
                        batch_size=config.rollout_batch_size,
                        decks=tuple(
                            getattr(engine.Deck, name) for name in config.decks
                        ),
                        **env_options,
                    )
                    write_evaluation_metrics(writer, completed, evaluation)
                    save_checkpoint(
                        config.checkpoint_path,
                        model,
                        optimizer,
                        completed,
                        evaluation,
                        config,
                    )
        except KeyboardInterrupt:
            print("Training interrupted; saving completed updates")
        finally:
            save_checkpoint(
                config.checkpoint_path, model, optimizer, completed, evaluation, config
            )
    return model


def main():
    train(TrainingConfig.from_env())


if __name__ == "__main__":
    main()
