import torch
from torch.optim import Adam

from simulation.blind_trainer import BlindModel
from simulation.training_config import (
    TrainingConfig,
    parse_evaluation_targets,
    parse_training_targets,
)
from simulation.training_evaluation import (
    EvaluationMetrics,
    TargetEvaluationMetrics,
    load_checkpoint,
    save_checkpoint,
)


def test_weighted_training_targets_expand_into_worker_cycle():
    assert parse_training_targets("300:1, 450:4") == (
        300,
        450,
        450,
        450,
        450,
    )
    assert parse_evaluation_targets("300,450") == (300, 450)


def test_configuration_resolves_best_resume_and_worker_targets(
    monkeypatch,
    tmp_path,
):
    monkeypatch.setenv("BALATRO_DEVICE", "cpu")
    monkeypatch.setenv("BALATRO_CHECKPOINT_DIR", str(tmp_path))
    monkeypatch.setenv("BALATRO_RESUME_CHECKPOINT", "best")
    monkeypatch.setenv("BALATRO_TRAIN_TARGETS", "300:1,450:4")
    monkeypatch.setenv("BALATRO_ROLLOUT_WORKERS", "6")
    config = TrainingConfig.from_env()

    assert config.resume_checkpoint == str(tmp_path / "best_model.pt")
    assert config.worker_targets() == [300, 450, 450, 450, 450, 300]


def test_checkpoint_round_trip_preserves_resume_metadata(monkeypatch, tmp_path):
    monkeypatch.setenv("BALATRO_DEVICE", "cpu")
    monkeypatch.setenv("BALATRO_CHECKPOINT_DIR", str(tmp_path))
    config = TrainingConfig.from_env()
    model = BlindModel(input_size=6, hidden_size=8)
    optimizer = Adam(model.parameters(), lr=config.learning_rate)
    evaluation = EvaluationMetrics(
        episodes=20,
        wins=18,
        win_rate=0.9,
        average_score=410.0,
        average_progress=0.97,
        average_hands_used_on_win=2.8,
        one_hand_win_rate=0.05,
        by_target=(
            TargetEvaluationMetrics(
                target=300,
                episodes=10,
                wins=10,
                win_rate=1.0,
                average_score=350.0,
                average_progress=1.0,
                average_hands_used_on_win=2.5,
                one_hand_win_rate=0.1,
            ),
            TargetEvaluationMetrics(
                target=450,
                episodes=10,
                wins=8,
                win_rate=0.8,
                average_score=470.0,
                average_progress=0.94,
                average_hands_used_on_win=3.0,
                one_hand_win_rate=0.0,
            ),
        ),
    )
    checkpoint_path = tmp_path / "checkpoint.pt"
    expected_weight = model.mode_head.weight.detach().clone()

    save_checkpoint(
        str(checkpoint_path),
        model,
        optimizer,
        input_size=6,
        hidden_size=8,
        iterations_completed=123,
        evaluation=evaluation,
        config=config,
    )
    with torch.no_grad():
        model.mode_head.weight.zero_()

    state = load_checkpoint(
        str(checkpoint_path),
        model,
        optimizer=None,
        device=torch.device("cpu"),
        load_optimizer=False,
    )

    assert state.iterations_completed == 123
    assert state.evaluation == evaluation
    assert torch.equal(model.mode_head.weight, expected_weight)
