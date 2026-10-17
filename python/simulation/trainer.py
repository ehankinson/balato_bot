from time import perf_counter

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
    format_iteration_summary,
    write_evaluation_metrics,
    write_iteration_metrics,
)


def train(config):
    run_started = perf_counter()
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
    save_needed = True
    iteration_seconds = final_save_seconds = 0.0
    with SummaryWriter(config.tensorboard_log_dir) as writer:
        setup_seconds = perf_counter() - run_started
        writer.add_scalar("timing/setup_seconds", setup_seconds, start)
        try:
            for iteration in range(start, start + config.iterations):
                iteration_started = perf_counter()
                # An interrupted PPO update may have changed weights without
                # advancing completed, so completion alone cannot track saves.
                save_needed = True
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
                rollout_started = perf_counter()
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
                rollout_seconds = perf_counter() - rollout_started
                train_started = perf_counter()
                metrics = update_policy(model, optimizer, episodes, config)
                # update_policy's CPU metric reads synchronize optimizer work.
                train_seconds = perf_counter() - train_started
                completed = iteration + 1
                eval_seconds = save_seconds = 0.0
                if completed % config.evaluation_interval == 0:
                    eval_started = perf_counter()
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
                    eval_seconds = perf_counter() - eval_started
                    write_evaluation_metrics(writer, completed, evaluation)
                    save_started = perf_counter()
                    save_checkpoint(
                        config.checkpoint_path,
                        model,
                        optimizer,
                        completed,
                        evaluation,
                        config,
                    )
                    save_seconds = perf_counter() - save_started
                    save_needed = False
                total_seconds = perf_counter() - iteration_started
                iteration_seconds += total_seconds
                summary = write_iteration_metrics(
                    writer,
                    completed,
                    episodes,
                    metrics,
                    rollout_seconds,
                    train_seconds=train_seconds,
                    eval_seconds=eval_seconds,
                    save_seconds=save_seconds,
                    total_seconds=total_seconds,
                )
                print(format_iteration_summary(completed, summary), flush=True)
        except KeyboardInterrupt:
            print("Training interrupted; saving current state")
        finally:
            if save_needed:
                save_started = perf_counter()
                save_checkpoint(
                    config.checkpoint_path,
                    model,
                    optimizer,
                    completed,
                    evaluation,
                    config,
                )
                final_save_seconds = perf_counter() - save_started
            writer.add_scalar(
                "timing/final_save_seconds", final_save_seconds, completed
            )
    run_seconds = perf_counter() - run_started
    other_seconds = run_seconds - setup_seconds - iteration_seconds - final_save_seconds
    print(
        f"Train call | Setup: {setup_seconds:.2f}s | "
        f"Iterations: {iteration_seconds:.2f}s | Final save: {final_save_seconds:.2f}s | "
        f"Other (logging/shutdown/partial): {other_seconds:.2f}s | "
        f"Total: {run_seconds:.2f}s (excludes process startup and this summary)",
        flush=True,
    )
    return model


def main():
    train(TrainingConfig.from_env())


if __name__ == "__main__":
    main()
