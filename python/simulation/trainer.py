import os
import random
from time import perf_counter

import torch
from torch.optim import Adam
from torch.utils.tensorboard import SummaryWriter

from core.enums import PokerHand
from simulation.blind_trainer import BlindModel
from simulation.encoder import observation_dim
from simulation.ppo import update_policy
from simulation.rollout import RolloutPool
from simulation.training_config import TrainingConfig
from simulation.training_evaluation import (
    EvaluationMetrics,
    checkpoint_state,
    evaluate_model,
    evaluation_is_better,
    load_checkpoint,
    save_checkpoint,
)
from simulation.training_logging import (
    format_evaluation,
    format_iteration,
    write_evaluation_metrics,
    write_iteration_metrics,
)


def _seed_everything(seed: int, device: torch.device) -> None:
    random.seed(seed)
    torch.manual_seed(seed)
    if device.type == "cuda":
        torch.cuda.manual_seed_all(seed)


def _load_resume_checkpoint(
    config: TrainingConfig,
    model: BlindModel,
    optimizer: Adam,
    device: torch.device,
) -> tuple[int, EvaluationMetrics | None]:
    if config.resume_checkpoint is None:
        return 0, None
    if not os.path.exists(config.resume_checkpoint):
        raise FileNotFoundError(
            f"resume checkpoint does not exist: {config.resume_checkpoint}"
        )

    state = load_checkpoint(
        config.resume_checkpoint,
        model,
        optimizer,
        device,
        load_optimizer=config.resume_optimizer,
    )
    # A resumed optimizer may contain its original learning rate. The current
    # configuration is authoritative so fine-tuning can deliberately lower it.
    for parameter_group in optimizer.param_groups:
        parameter_group["lr"] = config.learning_rate

    print(
        f"resumed {config.resume_checkpoint} after "
        f"{state.iterations_completed} completed iterations "
        f"(optimizer={str(config.resume_optimizer).lower()})"
    )
    return state.iterations_completed, state.evaluation


def train(config: TrainingConfig) -> None:
    device = torch.device(config.device)
    _seed_everything(config.seed, device)

    input_size = observation_dim(8)
    model = BlindModel(input_size, hidden_size=config.hidden_size).to(device)
    optimizer = Adam(model.parameters(), lr=config.learning_rate)
    start_iteration, last_evaluation = _load_resume_checkpoint(
        config,
        model,
        optimizer,
        device,
    )

    saved_best = checkpoint_state(config.best_checkpoint_path)
    best_evaluation = saved_best.evaluation if saved_best is not None else None
    if saved_best is None and last_evaluation is not None:
        save_checkpoint(
            config.best_checkpoint_path,
            model,
            optimizer,
            input_size,
            config.hidden_size,
            start_iteration,
            last_evaluation,
            config,
        )
        best_evaluation = last_evaluation
        print(f"seeded best checkpoint from {config.resume_checkpoint}")
    cumulative_hand_counts = torch.zeros(len(PokerHand), dtype=torch.long)
    writer = SummaryWriter(log_dir=config.tensorboard_log_dir, flush_secs=10)

    print(config.summary())
    interrupted = False
    last_completed_iteration = start_iteration

    try:
        with RolloutPool(
            episode_count=config.episodes_per_update,
            worker_count=config.effective_rollout_workers,
            worker_targets=config.worker_targets(),
            max_steps=config.max_steps_per_episode,
            input_size=input_size,
            profile=config.profile,
        ) as rollouts:
            try:
                for iteration in range(start_iteration, config.iterations):
                    iteration_started = perf_counter()
                    batch_seed = (
                        config.seed + iteration * config.episodes_per_update
                    )
                    batch, rollout_timings = rollouts.collect(
                        model,
                        device,
                        batch_seed,
                    )
                    ppo_metrics = update_policy(
                        model,
                        optimizer,
                        batch,
                        config,
                        device,
                    )
                    iteration_seconds = perf_counter() - iteration_started
                    summary = write_iteration_metrics(
                        writer,
                        iteration,
                        batch,
                        rollout_timings,
                        ppo_metrics,
                        iteration_seconds,
                        cumulative_hand_counts,
                        config.chart_interval,
                        config.profile,
                    )

                    completed_iterations = iteration + 1
                    last_completed_iteration = completed_iterations
                    if completed_iterations % config.evaluation_interval == 0:
                        evaluation_started = perf_counter()
                        evaluation = evaluate_model(
                            model,
                            device,
                            config.evaluation_episodes,
                            config.evaluation_seed,
                            config.max_steps_per_episode,
                            config.evaluation_targets,
                        )
                        evaluation_seconds = (
                            perf_counter() - evaluation_started
                        )
                        last_evaluation = evaluation
                        write_evaluation_metrics(
                            writer,
                            iteration,
                            evaluation,
                            evaluation_seconds,
                        )
                        save_checkpoint(
                            config.checkpoint_path,
                            model,
                            optimizer,
                            input_size,
                            config.hidden_size,
                            completed_iterations,
                            evaluation,
                            config,
                        )

                        is_best = evaluation_is_better(
                            evaluation,
                            best_evaluation,
                        )
                        if is_best:
                            best_evaluation = evaluation
                            save_checkpoint(
                                config.best_checkpoint_path,
                                model,
                                optimizer,
                                input_size,
                                config.hidden_size,
                                completed_iterations,
                                evaluation,
                                config,
                            )

                        print(
                            format_evaluation(
                                completed_iterations,
                                evaluation,
                                config.checkpoint_path,
                                (
                                    config.best_checkpoint_path
                                    if is_best
                                    else None
                                ),
                            )
                        )

                    if iteration % config.console_log_interval == 0:
                        print(format_iteration(iteration, summary))
            except KeyboardInterrupt:
                interrupted = True
                print("\ntraining interrupted; saving the latest model")
    finally:
        writer.close()

    save_checkpoint(
        config.checkpoint_path,
        model,
        optimizer,
        input_size,
        config.hidden_size,
        last_completed_iteration,
        last_evaluation,
        config,
    )
    print(
        f"saved {config.checkpoint_path} after "
        f"{last_completed_iteration} completed iterations"
    )
    if interrupted:
        return


def main() -> None:
    train(TrainingConfig.from_env())


if __name__ == "__main__":
    main()
