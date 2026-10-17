from dataclasses import asdict, replace
from types import SimpleNamespace

import pytest
import torch
from torch.optim import Adam

from simulation.policy import GamePolicy, ModelConfig
from simulation.ppo import PPOUpdateMetrics
from simulation.trainer import train
from simulation.training_config import TrainingConfig
from simulation.training_evaluation import (
    EvaluationMetrics,
    evaluate_model,
    load_checkpoint,
    load_policy,
    save_checkpoint,
)
from simulation.training_logging import (
    format_iteration_summary,
    write_iteration_metrics,
)


@pytest.fixture(scope="module", autouse=True)
def single_threaded():
    previous = torch.get_num_threads()
    torch.set_num_threads(1)
    yield
    torch.set_num_threads(previous)


@pytest.mark.parametrize("from_env", [False, True])
def test_default_objective_is_undiscounted_without_an_accepted_action_cost(
    monkeypatch, from_env
):
    for name in (
        "GAMMA",
        "GAE_LAMBDA",
        "REWARDS_CLEARED_BLIND",
        "REWARDS_ANTE_EIGHT",
        "REWARDS_LOSS",
        "REWARDS_REJECTED_ATTEMPT",
    ):
        monkeypatch.delenv(f"BALATRO_{name}", raising=False)
    config = TrainingConfig.from_env() if from_env else TrainingConfig()
    assert config.gamma == 1.0
    assert config.gae_lambda == 0.95
    assert asdict(config.rewards) == {
        "cleared_blind": 1.0,
        "ante_eight": 25.0,
        "loss": -25.0,
        "rejected_attempt": -0.1,
    }


def test_configuration_uses_full_game_model_resource_and_reward_settings(monkeypatch):
    monkeypatch.setenv("BALATRO_HORIZON", "37")
    monkeypatch.setenv("BALATRO_MODEL_WIDTH", "32")
    monkeypatch.setenv("BALATRO_LIMITS_MAX_COLLECTION_ROWS", "100")
    monkeypatch.setenv("BALATRO_REWARDS_CLEARED_BLIND", "2.5")
    monkeypatch.setenv("BALATRO_REWARDS_ANTE_EIGHT", "30")
    monkeypatch.setenv("BALATRO_REWARDS_LOSS", "-30")
    monkeypatch.setenv("BALATRO_REWARDS_REJECTED_ATTEMPT", "-0.2")
    monkeypatch.setenv("BALATRO_GAMMA", "0.9")
    monkeypatch.setenv("BALATRO_GAE_LAMBDA", "0.8")
    config = TrainingConfig.from_env()
    assert config.horizon == 37 and config.model.width == 32
    assert config.limits.max_collection_rows == 100
    assert config.rewards.cleared_blind == 2.5
    assert config.rewards.ante_eight == 30
    assert config.rewards.loss == -30
    assert config.rewards.rejected_attempt == -0.2
    assert config.gamma == 0.9
    assert config.gae_lambda == 0.8


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
    assert contents["training_config"]["gamma"] == 1.0
    assert contents["training_config"]["gae_lambda"] == 0.95
    assert contents["training_config"]["rewards"] == asdict(config.rewards)
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


def test_metrics_distinguish_wins_losses_and_truncations(monkeypatch):
    episodes = [
        SimpleNamespace(
            end_reason=reason,
            accepted_commands=1,
            final_observation={"run.counts": [0, 1]},
            transitions=[
                SimpleNamespace(
                    accepted=True,
                    truncated=reason == "horizon",
                    reward=reward,
                    completed_blind_score=None,
                )
            ],
        )
        for reason, reward in [("win", 25.0), ("loss", -25.0), ("horizon", 0.0)]
    ]
    monkeypatch.setattr(
        "simulation.training_evaluation.collect_runs", lambda *args, **kwargs: episodes
    )
    evaluation = evaluate_model(None, episodes=3, seed=0, horizon=1)
    assert evaluation.ante_eight_rate == pytest.approx(1 / 3)
    assert evaluation.losses == 1 and evaluation.truncations == 1
    values = {}
    writer = SimpleNamespace(
        add_scalar=lambda name, value, step: values.update({name: value})
    )
    summary = write_iteration_metrics(
        writer,
        1,
        episodes,
        PPOUpdateMetrics(0, 0, 0, 1),
        rollout_seconds=2.0,
        train_seconds=3.0,
        eval_seconds=4.0,
        save_seconds=5.0,
        total_seconds=15.0,
    )
    assert summary["rollout/wins"] == 1
    assert summary["rollout/losses"] == 1
    assert summary["rollout/truncations"] == 1
    assert summary["rollout/ante_eight_rate"] == pytest.approx(1 / 3)
    assert summary["rollout/seconds"] == 2.0
    assert summary["timing/train_seconds"] == 3.0
    assert summary["timing/eval_seconds"] == 4.0
    assert summary["timing/save_seconds"] == 5.0
    assert summary["timing/total_seconds"] == 15.0
    assert summary["ppo/update_count"] == 1
    assert summary["rollout/actions_per_second"] == 1.5
    assert summary["rollout/valid_action_rate"] == 1.0
    assert summary["rollout/episodes"] == 3
    assert summary["rollout/average_actions"] == 1.0
    assert summary == values


@pytest.mark.parametrize(
    "games, expected_count, expected_average, displayed_score",
    [
        pytest.param(
            [
                ("horizon", [(True, 100), (True, 200), (True, 300)]),
                ("loss", [(True, 1000)]),
            ],
            4,
            400.0,
            "400.0",
            id="all-blinds-not-per-game-means",
        ),
        pytest.param(
            [("win", [(True, 1000)]), ("loss", [(True, 0)])],
            2,
            500.0,
            "500.0",
            id="zero-score-loss-counts-in-average",
        ),
        pytest.param(
            [("loss", [(True, 0)])],
            1,
            0.0,
            "0.0",
            id="true-zero-is-not-unavailable",
        ),
        pytest.param(
            # Accepted skips and unfinished actions, then a rejected command.
            [("horizon", [(True, None), (True, None), (False, None)])],
            0,
            0.0,
            "N/A",
            id="skipped-unfinished-rejected-have-no-completed-score",
        ),
        pytest.param(
            [
                ("horizon", [(True, None), (True, 100), (True, None), (False, None)]),
                ("horizon", [(True, None)]),
                ("loss", [(True, 300)]),
            ],
            2,
            200.0,
            "200.0",
            id="none-scores-and-empty-games-excluded-from-denominator",
        ),
        pytest.param(
            [("loss", [(True, 2**64 - 1), (True, 2**64 - 2048)])],
            2,
            float(2**64 - 2048),
            "18,446,744,073,709,549,568.0",
            # The sum exceeds uint64. Converting either score to float before
            # summing rounds the average up to 2**64 instead of this result.
            id="uint64-scores-summed-as-python-ints-before-division",
        ),
    ],
)
def test_completed_blind_score_logging(
    games, expected_count, expected_average, displayed_score
):
    episodes = [
        SimpleNamespace(
            end_reason=reason,
            accepted_commands=sum(accepted for accepted, _ in attempts),
            transitions=[
                SimpleNamespace(
                    accepted=accepted,
                    truncated=reason == "horizon" and index == len(attempts) - 1,
                    reward=0.0,
                    completed_blind_score=score,
                )
                for index, (accepted, score) in enumerate(attempts)
            ],
        )
        for reason, attempts in games
    ]
    scalars = []
    writer = SimpleNamespace(
        add_scalar=lambda name, value, step: scalars.append((name, value, step))
    )
    summary = write_iteration_metrics(
        writer,
        7,
        episodes,
        PPOUpdateMetrics(0, 0, 0, 1),
        rollout_seconds=1.0,
        train_seconds=0.0,
        eval_seconds=0.0,
        save_seconds=0.0,
        total_seconds=1.0,
    )
    assert summary["rollout/completed_blinds"] == expected_count
    # Exact comparison catches premature float conversion at uint64 scale.
    assert summary["rollout/average_blind_score"] == expected_average
    score_scalars = [
        scalar
        for scalar in scalars
        if scalar[0] in {"rollout/completed_blinds", "rollout/average_blind_score"}
    ]
    assert score_scalars == [
        ("rollout/completed_blinds", expected_count, 7),
        ("rollout/average_blind_score", expected_average, 7),
    ]
    line = format_iteration_summary(7, summary)
    assert f" | Avg blind score: {displayed_score} | " in line
    assert "\n" not in line


@pytest.mark.parametrize(
    "wins, rate, reward, expected",
    [
        (0, 0.0, -1.3025, "Ante 8: 0.0% | Avg reward: -1.30"),
        (1, 0.25, 5.126, "Ante 8: 25.0% | Avg reward: 5.13"),
        (4, 1.0, 25.0, "Ante 8: 100.0% | Avg reward: 25.00"),
    ],
)
def test_console_summary_is_compact(wins, rate, reward, expected):
    summary = {
        "rollout/episodes": wins + 3,
        "rollout/average_actions": 30.25,
        "rollout/seconds": 1.1421456249663606,
        "rollout/actions_per_second": 105.94095652519248,
        "rollout/valid_action_rate": 2 / 3,
        "rollout/losses": 2,
        "rollout/truncations": 1,
        "rollout/wins": wins,
        "rollout/ante_eight_rate": rate,
        "rollout/average_reward": reward,
        "rollout/completed_blinds": 2,
        "rollout/average_blind_score": 1234.56,
        "ppo/policy_loss": 0.005289161112159491,
        "ppo/update_count": 42,
        "timing/train_seconds": 10.126,
        "timing/eval_seconds": 0.0,
        "timing/save_seconds": 1.234,
        "timing/total_seconds": 12.502,
        "rollout/rejected_attempts": 2,
    }
    assert format_iteration_summary(7, summary) == (
        f"Iteration 7 | Games: {wins + 3} | Wins: {wins} | "
        f"Losses: 2 | Truncated: 1 | {expected} | "
        "Avg blind score: 1,234.6 | Actions/s: 105.9 | Valid: 66.7% | Rejected: 2 | "
        "Avg actions/game: 30.2 | Rollout: 1.14s | Train: 10.13s | "
        "Eval: 0.00s | Save: 1.23s | Total: 12.50s | PPO updates: 42"
    )


@pytest.mark.parametrize(
    "accepted", [[False, False], [True, False, True], [True, True]]
)
def test_action_throughput_counts_only_accepted_commands(accepted):
    episode = SimpleNamespace(
        accepted_commands=sum(accepted),
        end_reason="horizon",
        transitions=[
            SimpleNamespace(
                accepted=value, truncated=False, reward=0, completed_blind_score=None
            )
            for value in accepted
        ],
    )
    writer = SimpleNamespace(add_scalar=lambda *args: None)
    summary = write_iteration_metrics(
        writer,
        1,
        [episode],
        PPOUpdateMetrics(0, 0, 0, 1),
        rollout_seconds=0.5,
        train_seconds=100.0,
        eval_seconds=0.0,
        save_seconds=0.0,
        total_seconds=100.5,
    )
    assert summary["rollout/actions_per_second"] == sum(accepted) / 0.5
    assert summary["rollout/valid_action_rate"] == pytest.approx(
        sum(accepted) / len(accepted)
    )
    assert summary["rollout/rejected_attempts"] == len(accepted) - sum(accepted)
    assert summary["rollout/average_actions"] == sum(accepted)


def test_valid_action_rate_weights_attempts_not_games():
    episodes = [
        SimpleNamespace(
            accepted_commands=sum(accepted),
            end_reason="horizon",
            transitions=[
                SimpleNamespace(
                    accepted=value,
                    truncated=False,
                    reward=0,
                    completed_blind_score=None,
                )
                for value in accepted
            ],
        )
        for accepted in [[True], [False, False, True]]
    ]
    values = {}
    writer = SimpleNamespace(
        add_scalar=lambda name, value, step: values.update({name: value})
    )
    summary = write_iteration_metrics(
        writer,
        1,
        episodes,
        PPOUpdateMetrics(0, 0, 0, 1),
        rollout_seconds=0.5,
        train_seconds=0.0,
        eval_seconds=0.0,
        save_seconds=0.0,
        total_seconds=0.5,
    )
    assert summary["rollout/valid_action_rate"] == 0.5
    assert summary["rollout/average_actions"] == 1.0
    assert summary["rollout/episodes"] == 2
    assert summary == values


@pytest.mark.parametrize("seconds", [0, -1, float("nan"), float("inf")])
def test_action_throughput_rejects_invalid_elapsed_time(seconds):
    with pytest.raises(ValueError, match="finite and positive"):
        write_iteration_metrics(
            None,
            1,
            [],
            None,
            rollout_seconds=seconds,
            train_seconds=0.0,
            eval_seconds=0.0,
            save_seconds=0.0,
            total_seconds=1.0,
        )


@pytest.mark.parametrize(
    "name", ["train_seconds", "eval_seconds", "save_seconds", "total_seconds"]
)
@pytest.mark.parametrize("seconds", [-1, float("nan"), float("inf"), -float("inf")])
def test_iteration_metrics_reject_invalid_stage_durations(name, seconds):
    timings = {
        "train_seconds": 1.0,
        "eval_seconds": 2.0,
        "save_seconds": 3.0,
        "total_seconds": 7.0,
    }
    timings[name] = seconds
    with pytest.raises(ValueError, match=f"{name} must be finite and non-negative"):
        write_iteration_metrics(None, 1, [], None, rollout_seconds=1.0, **timings)


@pytest.mark.parametrize("total_seconds", [0.0, 1.0, 3.0, 6.0, 9.99])
def test_iteration_total_must_include_all_stages(total_seconds):
    with pytest.raises(ValueError, match="must include rollout, train, eval, and save"):
        write_iteration_metrics(
            None,
            1,
            [],
            None,
            rollout_seconds=1.0,
            train_seconds=2.0,
            eval_seconds=3.0,
            save_seconds=4.0,
            total_seconds=total_seconds,
        )


@pytest.fixture
def timed_trainer(monkeypatch):
    import simulation.trainer as trainer_module

    clock = [100.0]
    events, scalars, saves = [], {}, []
    model = torch.nn.Linear(1, 1, bias=False)
    with torch.no_grad():
        model.weight.zero_()
    episode = SimpleNamespace(
        accepted_commands=2,
        end_reason="horizon",
        transitions=[
            SimpleNamespace(
                accepted=True, truncated=True, reward=0.0, completed_blind_score=None
            )
        ]
        * 2,
    )
    state = SimpleNamespace(interrupt=None, interrupt_call=2)
    calls = {"collect": 0, "update": 0, "evaluate": 0, "save": 0}

    def advance(seconds):
        clock[0] += seconds

    def stage(name, seconds):
        calls[name] += 1
        events.append((name, clock[0]))
        advance(seconds)
        if state.interrupt == name and calls[name] == state.interrupt_call:
            raise KeyboardInterrupt

    def make_model(*args):
        advance(3.0)
        return model

    def make_optimizer(*args, **kwargs):
        advance(2.0)
        return SimpleNamespace(param_groups=[{"lr": 0.0}])

    class Writer:
        def __init__(self, *args):
            advance(1.0)

        def __enter__(self):
            advance(3.0)
            return self

        def __exit__(self, *args):
            advance(11.0)

        def add_scalar(self, name, value, step):
            scalars[step, name] = value
            events.append((name, clock[0]))
            if name == "rollout/episodes":
                advance(7.0)
            elif name == "timing/setup_seconds":
                advance(1.0)
            elif name == "timing/final_save_seconds":
                advance(2.0)

        def flush(self):
            advance(4.0)

    def collect(*args):
        stage("collect", 2.0)
        return [episode]

    def update(*args):
        with torch.no_grad():
            model.weight.add_(1.0)
        stage("update", 3.0)
        return PPOUpdateMetrics(0, 0, 0, 6)

    def evaluate(*args, **kwargs):
        stage("evaluate", 5.0)
        return EvaluationMetrics(1, 0, 1, 0, 1, 0)

    def save(path, model, optimizer, completed, evaluation, config):
        stage("save", 7.0)
        saves.append((completed, model.weight.item(), evaluation))

    def record_print(line, **kwargs):
        events.append((line, clock[0]))
        print(line, **kwargs)

    monkeypatch.setattr(trainer_module, "perf_counter", lambda: clock[0])
    monkeypatch.setattr(trainer_module, "GamePolicy", make_model)
    monkeypatch.setattr(trainer_module, "Adam", make_optimizer)
    monkeypatch.setattr(trainer_module, "SummaryWriter", Writer)
    monkeypatch.setattr(trainer_module, "collect_runs", collect)
    monkeypatch.setattr(trainer_module, "update_policy", update)
    monkeypatch.setattr(trainer_module, "evaluate_model", evaluate)
    monkeypatch.setattr(trainer_module, "save_checkpoint", save)
    monkeypatch.setattr(trainer_module, "print", record_print, raising=False)
    return SimpleNamespace(
        config=TrainingConfig(iterations=2, evaluation_interval=2, device="cpu"),
        model=model,
        scalars=scalars,
        saves=saves,
        events=events,
        state=state,
        calls=calls,
        clock=clock,
    )


def test_trainer_attributes_stages_to_same_update_and_logs_after_save(
    timed_trainer, capsys
):
    run = timed_trainer
    assert train(run.config) is run.model
    for step, eval_seconds, save_seconds, total_seconds in [
        (1, 0, 0, 5),
        (2, 5, 7, 21),
    ]:
        assert run.scalars[step, "rollout/seconds"] == 2
        assert run.scalars[step, "rollout/actions_per_second"] == 1
        assert run.scalars[step, "rollout/completed_blinds"] == 0
        assert run.scalars[step, "rollout/average_blind_score"] == 0.0
        assert run.scalars[step, "ppo/update_count"] == 6
        assert run.scalars[step, "timing/train_seconds"] == 3
        assert run.scalars[step, "timing/eval_seconds"] == eval_seconds
        assert run.scalars[step, "timing/save_seconds"] == save_seconds
        # Evaluation logging takes four seconds, included only in Total.
        assert run.scalars[step, "timing/total_seconds"] == total_seconds
    assert run.calls == {"collect": 2, "update": 2, "evaluate": 1, "save": 1}
    assert run.scalars[0, "timing/setup_seconds"] == 9
    assert run.scalars[2, "timing/final_save_seconds"] == 0
    lines = capsys.readouterr().out.splitlines()
    assert len(lines) == 3
    for line in lines[:2]:
        assert " | Avg blind score: N/A | " in line
    assert lines[0].endswith(
        "Rollout: 2.00s | Train: 3.00s | Eval: 0.00s | Save: 0.00s | "
        "Total: 5.00s | PPO updates: 6"
    )
    assert lines[1].endswith(
        "Rollout: 2.00s | Train: 3.00s | Eval: 5.00s | Save: 7.00s | "
        "Total: 21.00s | PPO updates: 6"
    )
    assert lines[2] == (
        "Train call | Setup: 9.00s | Iterations: 26.00s | Final save: 0.00s | "
        "Other (logging/shutdown/partial): 28.00s | "
        "Total: 63.00s (excludes process startup and this summary)"
    )
    event_names = [name for name, _ in run.events]
    assert event_names.index("save") < event_names.index(lines[1])
    iteration_logs = [
        index for index, name in enumerate(event_names) if name == "rollout/episodes"
    ]
    assert event_names.index("save") < iteration_logs[1]
    assert run.clock[0] - 100 == 63


def test_final_save_is_separate_from_iteration_and_included_in_run_total(
    timed_trainer, capsys
):
    run = timed_trainer
    train(replace(run.config, iterations=1))
    assert run.calls == {"collect": 1, "update": 1, "evaluate": 0, "save": 1}
    assert run.scalars[1, "timing/total_seconds"] == 5
    assert run.scalars[1, "timing/save_seconds"] == 0
    assert run.scalars[1, "timing/final_save_seconds"] == 7
    lines = capsys.readouterr().out.splitlines()
    assert lines[-1] == (
        "Train call | Setup: 9.00s | Iterations: 5.00s | Final save: 7.00s | "
        "Other (logging/shutdown/partial): 21.00s | "
        "Total: 42.00s (excludes process startup and this summary)"
    )
    event_names = [name for name, _ in run.events]
    assert event_names.index(lines[0]) < event_names.index("save")
    assert run.clock[0] - 100 == 42


@pytest.mark.parametrize(
    "stage, completed, weight, partial_seconds",
    [
        ("collect", 1, 1, 2),
        ("update", 1, 2, 5),
        ("evaluate", 2, 2, 10),
        ("save", 2, 2, 21),
    ],
)
def test_interruption_saves_current_weights_even_after_previous_checkpoint(
    timed_trainer, capsys, stage, completed, weight, partial_seconds
):
    run = timed_trainer
    run.state.interrupt = stage
    train(replace(run.config, evaluation_interval=1))
    assert len(run.saves) == 2
    assert run.saves[0][:2] == (1, 1)
    assert run.saves[1][:2] == (completed, weight)
    assert run.scalars[completed, "timing/final_save_seconds"] == 7
    assert (2, "timing/total_seconds") not in run.scalars
    lines = capsys.readouterr().out.splitlines()
    assert len(lines) == 3
    assert lines[1] == "Training interrupted; saving current state"
    other_seconds = 21 + partial_seconds
    total_seconds = 9 + 21 + 7 + other_seconds
    assert lines[-1] == (
        "Train call | Setup: 9.00s | Iterations: 21.00s | Final save: 7.00s | "
        f"Other (logging/shutdown/partial): {other_seconds:.2f}s | "
        f"Total: {total_seconds:.2f}s (excludes process startup and this summary)"
    )
    assert run.clock[0] - 100 == total_seconds


@pytest.mark.parametrize(
    "device",
    [
        "cpu",
        pytest.param(
            "mps",
            marks=pytest.mark.skipif(
                not torch.backends.mps.is_available(), reason="MPS unavailable"
            ),
        ),
    ],
)
def test_training_entry_point_saves_and_resumes_joint_policy(
    tmp_path, device, monkeypatch, capsys
):
    import simulation.trainer as trainer_module

    clock = [0.0]
    rollout_rewards = []
    update_configs = []
    collect = trainer_module.collect_runs
    update = trainer_module.update_policy

    def timed_collect(*args, **kwargs):
        rollout_rewards.append(args[-1])
        result = collect(*args, **kwargs)
        clock[0] += 2.0
        return result

    def timed_update(*args, **kwargs):
        update_configs.append(args[-1])
        result = update(*args, **kwargs)
        clock[0] += 100.0
        return result

    monkeypatch.setattr(trainer_module, "perf_counter", lambda: clock[0])
    monkeypatch.setattr(trainer_module, "collect_runs", timed_collect)
    monkeypatch.setattr(trainer_module, "update_policy", timed_update)
    config = TrainingConfig(
        device=device,
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
    resumed_config = replace(
        config,
        resume_checkpoint=config.checkpoint_path,
        resume_optimizer=True,
        rewards=replace(config.rewards, cleared_blind=2.0, loss=-30.0),
        gamma=0.9,
    )
    train(resumed_config)
    checkpoint = torch.load(config.checkpoint_path, weights_only=True)
    assert checkpoint["iterations_completed"] == 2
    assert rollout_rewards == [config.rewards, resumed_config.rewards]
    assert update_configs == [config, resumed_config]
    assert checkpoint["training_config"]["rewards"] == asdict(resumed_config.rewards)
    assert checkpoint["training_config"]["gamma"] == resumed_config.gamma
    # Three accepted commands / two rollout seconds; exclude the PPO time.
    lines = capsys.readouterr().out.splitlines()
    assert len(lines) == 4
    for line in lines[1::2]:
        assert line == (
            "Train call | Setup: 0.00s | Iterations: 102.00s | Final save: 0.00s | "
            "Other (logging/shutdown/partial): 0.00s | "
            "Total: 102.00s (excludes process startup and this summary)"
        )
    for iteration, line in enumerate(lines[::2], start=1):
        assert line.startswith(f"Iteration {iteration} | Games: 1 | Wins: 0 | ")
        assert "Losses: 0 | Truncated: 1 | Ante 8: 0.0% | Avg reward:" in line
        assert line.endswith(
            " | Avg blind score: N/A | Actions/s: 1.5 | Valid: 100.0% | Rejected: 0 | "
            "Avg actions/game: 3.0 | Rollout: 2.00s | Train: 100.00s | "
            "Eval: 0.00s | Save: 0.00s | Total: 102.00s | PPO updates: 2"
        )
        assert "rollout/" not in line and "ppo/" not in line
