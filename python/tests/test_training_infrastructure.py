from dataclasses import replace

import pytest
import torch
from simulation.policy import GamePolicy, ModelConfig
from simulation.trainer import train
from simulation.training_config import TrainingConfig
from simulation.training_evaluation import (
    EvaluationMetrics,
    evaluate_model,
    load_checkpoint,
    load_policy,
    save_checkpoint,
)
from torch.optim import Adam


@pytest.fixture(scope="module", autouse=True)
def single_threaded():
    previous = torch.get_num_threads()
    torch.set_num_threads(1)
    yield
    torch.set_num_threads(previous)


def test_configuration_uses_full_game_model_resource_and_reward_settings(monkeypatch):
    monkeypatch.setenv("BALATRO_HORIZON", "37")
    monkeypatch.setenv("BALATRO_MODEL_WIDTH", "32")
    monkeypatch.setenv("BALATRO_LIMITS_MAX_COLLECTION_ROWS", "100")
    monkeypatch.setenv("BALATRO_REWARDS_CLEARED_BLIND", "2.5")
    config = TrainingConfig.from_env()
    assert config.horizon == 37 and config.model.width == 32
    assert config.limits.max_collection_rows == 100
    assert config.rewards.cleared_blind == 2.5


@pytest.mark.parametrize(
    "options",
    [
        {"horizon": 0},
        {"decks": ("fake",)},
        {"minibatch_size": 0},
        {"gamma": 2},
        {"seed": -1},
        {"stake": "fake"},
    ],
)
def test_invalid_training_configuration_is_rejected(options):
    with pytest.raises(ValueError):
        TrainingConfig(**options)


def test_checkpoint_round_trip_saves_the_entire_joint_model_and_configuration(tmp_path):
    config = TrainingConfig(model=ModelConfig(16, 2, 1))
    model = GamePolicy(config.model, config.limits)
    optimizer = Adam(model.parameters(), lr=config.learning_rate)
    evaluation = EvaluationMetrics(2, 0, 1.5, 1, 1, 0)
    path = tmp_path / "policy.pt"
    expected = {
        key: tensor.detach().clone() for key, tensor in model.state_dict().items()
    }
    save_checkpoint(path, model, optimizer, 12, evaluation, config)
    with torch.no_grad():
        for parameter in model.parameters():
            parameter.zero_()
    state = load_checkpoint(path, model, optimizer, load_optimizer=True)
    assert state.iterations_completed == 12 and state.evaluation == evaluation
    for key, value in model.state_dict().items():
        torch.testing.assert_close(value, expected[key])
    loaded = load_policy(path)
    assert loaded.config == config.model and loaded.limits == config.limits
    contents = torch.load(path, weights_only=True)
    assert contents["schema_version"] == 1
    assert contents["training_config"]["horizon"] == 4096
    assert contents["resource_limits"]["max_tokens"] == config.limits.max_tokens


@pytest.mark.parametrize(
    "mismatch", ["old", "schema", "architecture", "config", "limits"]
)
def test_incompatible_checkpoint_rejection_precedes_weight_mutation(tmp_path, mismatch):
    config = TrainingConfig(model=ModelConfig(16, 2, 1))
    model = GamePolicy(config.model)
    path = tmp_path / "bad.pt"
    save_checkpoint(path, model, Adam(model.parameters()), 0, None, config)
    checkpoint = torch.load(path, weights_only=True)
    if mismatch == "old":
        checkpoint = {
            "state_dict": checkpoint["state_dict"],
            "input_size": 275,
            "architecture_version": 2,
        }
    elif mismatch == "schema":
        checkpoint["schema_version"] = 2
    elif mismatch == "architecture":
        checkpoint["architecture"] = "other"
    elif mismatch == "config":
        checkpoint["model_config"]["depth"] += 1
    else:
        checkpoint["resource_limits"]["max_tokens"] += 1
    torch.save(checkpoint, path)
    before = model.action_head.weight.detach().clone()
    with pytest.raises(ValueError, match="incompatible|do not match"):
        load_checkpoint(path, model)
    torch.testing.assert_close(model.action_head.weight, before)


def test_seeded_evaluation_is_repeatable_and_preserves_model_mode():
    torch.manual_seed(5)
    model = GamePolicy(ModelConfig(16, 2, 1))
    model.train()
    left = evaluate_model(model, episodes=2, seed=10, horizon=4)
    right = evaluate_model(model, episodes=2, seed=10, horizon=4)
    assert left == right and model.training
    assert left.episodes == 2
    assert left.losses + left.truncations == 2


def test_training_entry_point_saves_and_resumes_joint_policy(tmp_path):
    config = TrainingConfig(
        iterations=1,
        episodes_per_update=1,
        ppo_epochs=1,
        minibatch_size=2,
        horizon=3,
        model=ModelConfig(16, 2, 1),
        evaluation_interval=1,
        evaluation_episodes=1,
        checkpoint_path=str(tmp_path / "policy.pt"),
        tensorboard_log_dir=str(tmp_path / "logs"),
    )
    train(config)
    checkpoint = torch.load(config.checkpoint_path, weights_only=True)
    assert checkpoint["iterations_completed"] == 1
    assert checkpoint["evaluation"]["episodes"] == 1
    train(
        replace(config, resume_checkpoint=config.checkpoint_path, resume_optimizer=True)
    )
    checkpoint = torch.load(config.checkpoint_path, weights_only=True)
    assert checkpoint["iterations_completed"] == 2
