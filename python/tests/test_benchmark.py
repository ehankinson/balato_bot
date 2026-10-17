import numpy as np
import torch
from simulation.benchmark import benchmark_case
from simulation.policy import GamePolicy, ModelConfig


def test_benchmark_reports_finite_separate_components_and_restores_model_mode():
    previous = torch.get_num_threads()
    torch.set_num_threads(1)
    try:
        model = GamePolicy(ModelConfig(16, 2, 1))
        result = benchmark_case(model, batch_size=2, iterations=2, warmup=1)
        assert model.training
        assert result["attempts"] == 4
        assert 0 < result["accepted_commands"] <= 4
        assert result["attempts_per_second"] > 0
        components = result["milliseconds_per_attempt"]
        assert {
            "engine_step_seconds",
            "observation_encode_seconds",
            "binding_overhead_seconds",
            "array_export_seconds",
            "tensor_conversion_seconds",
            "device_transfer_seconds",
            "inference_seconds",
            "decoding_seconds",
            "statistics_transfer_seconds",
            "rollout_storage_seconds",
            "end_to_end_seconds",
        }.issubset(components)
        assert np.isfinite(list(components.values())).all()
        assert all(value >= 0 for value in components.values())
        assert sum(result["phase_counts"].values()) == 4
    finally:
        torch.set_num_threads(previous)
