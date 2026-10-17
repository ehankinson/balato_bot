"""Reproducible continuous rollout benchmark with separate pipeline timings."""

import argparse
import json
import platform
from dataclasses import asdict
from pathlib import Path
from statistics import median
from time import perf_counter

import numpy as np
import torch

import balatro_engine as engine
from simulation.decoder import decode_action_batch
from simulation.env import BatchEnv
from simulation.policy import GamePolicy, ModelConfig
from simulation.rollout import Transition


def synchronize(device):
    if device.type == "cuda":
        torch.cuda.synchronize(device)
    elif device.type == "mps":
        torch.mps.synchronize()


class TimedConstraints:
    def __init__(self, games):
        self.games, self.seconds = games, 0.0

    def constraints(self, kinds, sources, prefixes):
        started = perf_counter()
        result = self.games.constraints(kinds, sources, prefixes)
        self.seconds += perf_counter() - started
        return result


def benchmark_case(model, batch_size, iterations=32, warmup=4, seed=0):
    if min(batch_size, iterations) <= 0 or warmup < 0:
        raise ValueError(
            "batch size/iterations must be positive and warmup nonnegative"
        )
    device = next(model.parameters()).device
    decks = [
        getattr(engine.Deck, ("Red", "Magic", "Painted")[index % 3])
        for index in range(batch_size)
    ]
    env = BatchEnv(list(range(seed, seed + batch_size)), decks=decks)
    totals, records, phases = {}, [], {}
    accepted = attempts = 0
    torch.manual_seed(seed + 1)
    was_training = model.training
    model.eval()
    try:
        with torch.no_grad():
            for iteration in range(-warmup, iterations):
                synchronize(device)
                started = perf_counter()
                stage = started
                finished = np.flatnonzero(~env.active).tolist()
                if finished:
                    env.reset(
                        finished,
                        [
                            seed + (iteration + warmup + 1) * batch_size + slot
                            for slot in finished
                        ],
                    )
                reset_seconds = perf_counter() - stage
                observations = env.snapshots
                stage = perf_counter()
                prepared = model.prepare(observations)
                conversion_seconds = perf_counter() - stage
                stage = perf_counter()
                transferred = prepared.to(device)
                synchronize(device)
                transfer_seconds = perf_counter() - stage
                stage = perf_counter()
                output = model.forward_batch(transferred)
                synchronize(device)
                inference_seconds = perf_counter() - stage
                stage = perf_counter()
                queries = TimedConstraints(env.games)
                decoded = decode_action_batch(
                    model, output, queries, observations, env.active
                )
                synchronize(device)
                decoding_seconds = perf_counter() - stage
                stage = perf_counter()
                stats = (
                    torch.stack((decoded.log_probs, output.value), -1)
                    .detach()
                    .cpu()
                    .numpy()
                )
                synchronize(device)
                statistics_transfer_seconds = perf_counter() - stage
                results = env.step(decoded.commands, profile=True)
                stage = perf_counter()
                retained = []
                for index, result in enumerate(results.results):
                    trace = decoded.traces[index]
                    if not result.accepted and trace.source_known:
                        raise RuntimeError(
                            f"guaranteed-legal benchmark command rejected: {result.error}"
                        )
                    retained.append(
                        Transition(
                            observations[index],
                            result.observation,
                            trace,
                            float(stats[index, 0]),
                            float(stats[index, 1]),
                            result.reward,
                            result.accepted,
                            result.terminated,
                            result.truncated,
                            result.error,
                        )
                    )
                storage_seconds = perf_counter() - stage
                end_to_end_seconds = perf_counter() - started
                if iteration >= 0:
                    records.extend(retained)
                    attempts += batch_size
                    accepted += sum(result.accepted for result in results.results)
                    for observation in observations:
                        phase = str(int(observation["run.cat"][0]))
                        phases[phase] = phases.get(phase, 0) + 1
                    timings = {
                        "reset_seconds": reset_seconds,
                        "tensor_conversion_seconds": conversion_seconds,
                        "device_transfer_seconds": transfer_seconds,
                        "inference_seconds": inference_seconds,
                        "decoding_seconds": decoding_seconds,
                        "constraint_binding_seconds": queries.seconds,
                        "statistics_transfer_seconds": statistics_transfer_seconds,
                        "rollout_storage_seconds": storage_seconds,
                        "end_to_end_seconds": end_to_end_seconds,
                        **results.timings,
                    }
                    for name, seconds in timings.items():
                        totals[name] = totals.get(name, 0.0) + seconds
    finally:
        model.train(was_training)
    return {
        "batch_size": batch_size,
        "iterations": iterations,
        "warmup": warmup,
        "attempts": attempts,
        "accepted_commands": accepted,
        "attempts_per_second": attempts / totals["end_to_end_seconds"],
        "accepted_commands_per_second": accepted / totals["end_to_end_seconds"],
        "milliseconds_per_attempt": {
            name: seconds * 1000 / attempts for name, seconds in totals.items()
        },
        "phase_counts": phases,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--batches", default="1,4,16")
    parser.add_argument("--iterations", type=int, default=32)
    parser.add_argument("--warmup", type=int, default=4)
    parser.add_argument("--repeats", type=int, default=3)
    parser.add_argument("--threads", type=int, default=1)
    parser.add_argument("--device", default="cpu")
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if args.repeats <= 0 or args.threads <= 0:
        parser.error("repeats/threads must be positive")
    torch.set_num_threads(args.threads)
    torch.manual_seed(args.seed)
    model = GamePolicy(ModelConfig()).to(args.device)
    cases = []
    for size in [int(value) for value in args.batches.split(",")]:
        runs = [
            benchmark_case(model, size, args.iterations, args.warmup, args.seed)
            for _ in range(args.repeats)
        ]
        cases.append(
            {
                "batch_size": size,
                "median_attempts_per_second": median(
                    run["attempts_per_second"] for run in runs
                ),
                "runs": runs,
            }
        )
    report = {
        "python": platform.python_version(),
        "platform": platform.platform(),
        "machine": platform.machine(),
        "torch": torch.__version__,
        "numpy": np.__version__,
        "engine_build_profile": engine.build_profile(),
        "device": args.device,
        "cuda_available": torch.cuda.is_available(),
        "mps_available": torch.backends.mps.is_available(),
        "torch_threads": args.threads,
        "seed": args.seed,
        "model_config": asdict(model.config),
        "resource_limits": asdict(model.limits),
        "notes": "Untrained seeded policy; continuous retained rollouts with explicit resets. Excludes PPO updates. Constraint timing is a subset of decoding, not additive. No asynchronous overlap or buffer reuse.",
        "cases": cases,
    }
    rendered = json.dumps(report, indent=2)
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered + "\n")
    print(rendered)


if __name__ == "__main__":
    main()
