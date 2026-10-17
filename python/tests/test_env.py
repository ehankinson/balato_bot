from dataclasses import replace

import numpy as np
import pytest
import torch

import balatro_engine as engine
from simulation.decoder import Command, evaluate_actions
from simulation.env import EpisodeState, GameEnv
from simulation.policy import GamePolicy, ModelConfig
from simulation.ppo import compute_gae, update_policy
from simulation.reward import RewardConfig, transition_reward
from simulation.rollout import collect_episode
from simulation.training_config import TrainingConfig


@pytest.fixture(scope="module", autouse=True)
def single_threaded():
    previous = torch.get_num_threads()
    torch.set_num_threads(1)
    yield
    torch.set_num_threads(previous)


def test_seeded_baseline_traverses_full_game_boundaries_with_retained_snapshots():
    env = GameEnv(horizon=30)
    first = env.reset(0)
    assert env.step(Command(0, source=0)).observation["run.cat"][0] == 1
    before = env.observation
    expected = {key: array.copy() for key, array in before.items()}
    for _ in range(4):
        result = env.step(Command(2, targets=(0, 1, 2, 3, 4)))
        if result.observation["run.cat"][0] != 1:
            break
    assert result.observation["run.cat"][0] == 2
    assert result.reward == pytest.approx(1.0)
    assert not result.terminated
    assert env.step(Command(7, source=0)).observation["run.cat"][0] == 3
    assert env.step(Command(13)).observation["run.cat"][0] == 2
    assert env.step(Command(12)).observation["run.cat"][0] == 0
    env.step(Command(0, source=1))
    for _ in range(4):
        result = env.step(Command(2, targets=(0,)))
        if result.terminated:
            break
    assert result.terminated and not result.truncated
    assert result.end_reason == "loss" and result.reward == pytest.approx(-25.0)
    assert first["run.cat"][0] == 0
    for key, value in expected.items():
        np.testing.assert_array_equal(before[key], value)
    with pytest.raises(RuntimeError, match="reset"):
        env.step(Command(15, order=()))


@pytest.mark.parametrize("seed, final_score", [(0, 472), (1, 252)])
def test_completed_blind_score_includes_final_hand_once(seed, final_score):
    env = GameEnv(horizon=20)
    env.reset(seed)
    assert env.step(Command(0, source=0)).completed_blind_score is None
    for _ in range(3):
        assert (
            env.step(Command(2, targets=tuple(range(5)))).completed_blind_score is None
        )
    result = env.step(Command(2, targets=tuple(range(5))))
    assert result.completed_blind_score == final_score
    assert type(result.completed_blind_score) is int
    assert not result.observation["blind.visible"].any()
    if not result.terminated:
        # Shop actions retain the engine's last score, but must not count it again.
        rejected = env.step(Command(4, source=1000))
        assert not rejected.accepted and rejected.completed_blind_score is None
        assert env.step(Command(7, source=0)).completed_blind_score is None
        assert env.step(Command(13)).completed_blind_score is None
        assert env.step(Command(12)).completed_blind_score is None
        assert env.step(Command(1, source=1)).completed_blind_score is None


def test_unfinished_blinds_at_horizon_are_not_completed_scores():
    env = GameEnv(horizon=2)
    env.reset(0)
    env.step(Command(0, source=0))
    result = env.step(Command(2, targets=tuple(range(5))))
    assert env.game.blind_score == 44
    assert result.truncated and result.end_reason == "horizon"
    assert result.completed_blind_score is None


def test_immediate_blind_loss_counts_zero_score_not_a_missing_score():
    before = engine.new(0).observe()
    after = {key: value.copy() for key, value in before.items()}
    after["run.cat"][0] = 5
    after["run.flags"][0] = True
    state = EpisodeState(10, 3, RewardConfig())
    state.reset(before)
    result = state.finish(after, True, None, command_kind=0, blind_score=0)
    assert result.terminated and result.completed_blind_score == 0


def test_rejected_commands_do_not_count_toward_horizon_or_mutate_state():
    env = GameEnv(horizon=1)
    before = env.reset(0)
    failed = env.step(Command(0, source=2))
    assert not failed.accepted and not failed.truncated and not failed.terminated
    assert env.accepted_commands == 0
    assert failed.reward == -0.1
    for key in before:
        np.testing.assert_array_equal(before[key], failed.observation[key])
    accepted = env.step(Command(0, source=0))
    assert accepted.accepted and accepted.truncated and not accepted.terminated
    assert accepted.observation["run.cat"][0] == 1
    assert accepted.end_reason == "horizon"


@pytest.mark.parametrize("horizon", [1, 10])
@pytest.mark.parametrize(
    "lost,won,reason",
    [(False, True, "win"), (True, False, "loss"), (True, True, "loss")],
)
def test_episode_outcomes_are_terminal_even_at_the_horizon(horizon, lost, won, reason):
    before = engine.new(0).observe()
    after = {key: value.copy() for key, value in before.items()}
    after["run.flags"][:2] = [lost, won]
    state = EpisodeState(horizon, 3, RewardConfig())
    state.reset(before)
    result = state.finish(after, True, None, command_kind=0, blind_score=0)
    assert result.terminated and not result.truncated
    assert result.end_reason == reason and state.done
    assert state.accepted_commands == 1
    assert result.reward == pytest.approx(25.0 if reason == "win" else -25.0)
    assert not before["run.flags"][:2].any()
    state.reset(before)
    assert not state.done and state.accepted_commands == 0


def test_direct_identity_reorders_remain_legal_without_reward_or_progress():
    env = GameEnv(horizon=3)
    env.reset(0)
    env.step(Command(0, source=0))
    before = env.observation
    identity = tuple(range(len(before["hand.present"])))
    results = [env.step(Command(16, order=identity)) for _ in range(2)]
    assert all(result.accepted and result.reward == 0 for result in results)
    assert results[-1].truncated and results[-1].end_reason == "horizon"
    np.testing.assert_array_equal(before["hand.cat"], env.observation["hand.cat"])
    np.testing.assert_array_equal(before["run.counts"], env.observation["run.counts"])


def test_single_env_reorder_allowance_survives_rejection_and_resets_on_progress():
    env = GameEnv(horizon=20)
    env.reset(0)
    env.step(Command(0, source=0))
    before = env.observation
    assert before["action_kinds"][16]
    assert not before["action_kinds"][15]  # No Jokers to rearrange.
    result = env.step(Command(16, order=tuple(range(7, -1, -1))))
    assert result.accepted and result.reward == 0
    assert not result.observation["action_kinds"][16]
    assert before["action_kinds"][16]  # Retained masks are not mutated.
    assert env.game.observe()["action_kinds"][16]  # Still legal in Rust.

    rejected = env.step(Command(3, targets=(99,)))
    assert not rejected.accepted
    assert not rejected.observation["action_kinds"][16]
    progressed = env.step(Command(3, targets=(0,)))
    assert progressed.accepted and progressed.reward == 0
    assert progressed.observation["action_kinds"][16]

    env.step(Command(16, order=tuple(range(7, -1, -1))))
    env.reset(0)
    assert env.step(Command(0, source=0)).observation["action_kinds"][16]


def test_loss_takes_precedence_over_accepted_command_horizon():
    env = GameEnv(horizon=5)
    env.reset(1)
    env.step(Command(0, source=0))
    for _ in range(4):
        result = env.step(Command(2, targets=(0,)))
    assert result.terminated and not result.truncated


class RejectingGame:
    def __init__(self, game, known):
        self.game, self.known = game, known

    def observe(self):
        return self.game.observe()

    def action_constraints(self, action, source=None):
        result = self.game.action_constraints(action, source=source)
        result["source_known"][:] = self.known
        return result

    def apply_action(self, *args, **kwargs):
        raise ValueError("unavailable action")


class RejectionEnv(GameEnv):
    def __init__(self, known=False):
        super().__init__(horizon=1, max_consecutive_rejections=3)
        self.known = known

    def reset(self, seed):
        observation = super().reset(seed)
        self.game = RejectingGame(self.game, self.known)
        return observation


def selecting_model():
    model = GamePolicy(ModelConfig(16, 2, 1))
    with torch.no_grad():
        model.action_head.weight.zero_()
        model.action_head.bias.fill_(-1000)
        model.action_head.bias[0] = 1000
    return model


def test_uncertain_rejections_are_recorded_with_original_likelihood_and_bounded():
    model = selecting_model()
    episode = collect_episode(model, RejectionEnv(), 0, stochastic=False)
    assert episode.end_reason == "rejection_limit"
    assert episode.accepted_commands == 0
    assert len(episode.transitions) == 3
    assert episode.transitions[-1].truncated
    assert all(
        not step.accepted and not step.trace.source_known and step.reward == -0.1
        for step in episode.transitions
    )
    output = model([step.observation for step in episode.transitions])
    logs, _ = evaluate_actions(
        model, output, [step.trace for step in episode.transitions]
    )
    torch.testing.assert_close(
        logs, torch.tensor([step.old_log_prob for step in episode.transitions])
    )
    assert all(
        step.trace.command == Command(0, source=0) for step in episode.transitions
    )
    assert episode.bootstrap_value == pytest.approx(
        float(model([episode.final_observation]).value[0].detach())
    )


def test_guaranteed_legal_rejection_is_reported_as_a_contract_error():
    with pytest.raises(RuntimeError, match="guaranteed-legal"):
        collect_episode(
            selecting_model(), RejectionEnv(known=True), 0, stochastic=False
        )


@pytest.mark.parametrize("end_reason", ["loss", "win"])
def test_gae_bootstraps_truncations_but_not_terminal_outcomes(end_reason):
    model = selecting_model()
    truncated = collect_episode(model, GameEnv(horizon=1), 0, stochastic=False)
    transition = replace(truncated.transitions[0], old_value=2, reward=3)
    truncated = replace(truncated, transitions=(transition,), bootstrap_value=4)
    terminal = replace(
        truncated,
        transitions=(replace(transition, terminated=True, truncated=False),),
        bootstrap_value=999,
        end_reason=end_reason,
    )
    _, returns, _ = compute_gae([truncated, terminal], gamma=0.5, gae_lambda=1)
    torch.testing.assert_close(returns, torch.tensor([5.0, 3.0]))


def test_small_training_update_has_finite_losses_gradients_and_changes_weights():
    torch.manual_seed(11)
    model = GamePolicy(ModelConfig(16, 2, 1))
    episodes = [
        collect_episode(model, GameEnv(horizon=12, deck=deck), seed)
        for seed, deck in [(0, engine.Deck.Magic), (3, engine.Deck.Painted)]
    ]
    before = model.attention[0].self_attn.in_proj_weight.detach().clone()
    metrics = update_policy(
        model,
        torch.optim.Adam(model.parameters(), lr=1e-3),
        episodes,
        TrainingConfig(ppo_epochs=1, minibatch_size=4),
    )
    assert metrics.update_count > 0
    assert np.isfinite([metrics.policy_loss, metrics.value_loss, metrics.entropy]).all()
    assert not torch.equal(before, model.attention[0].self_attn.in_proj_weight)
    assert all(
        torch.isfinite(parameter.grad).all()
        for parameter in model.parameters()
        if parameter.grad is not None
    )
    assert any(
        step.trace.command.source is not None
        for episode in episodes
        for step in episode.transitions
    )


def test_rewards_use_public_round_success_and_loss_facts_only():
    before = engine.new(0).observe()
    after = {key: value.copy() for key, value in before.items()}
    after["run.counts"][1] += 2
    after["run.flags"][1] = True
    assert transition_reward(before, after, True) == pytest.approx(27.0)
    assert transition_reward(after, after, True) == 0
    assert transition_reward(before, after, False) == -0.1
    assert transition_reward(
        before, after, True, RewardConfig(cleared_blind=3)
    ) == pytest.approx(31.0)


class WinningGame:
    """Exercise the rollout boundary with a synthetic public success signal."""

    def __init__(self, game):
        self.game = game
        self.won = False

    def __getattr__(self, name):
        return getattr(self.game, name)

    def apply_action(self, *args, **kwargs):
        result = self.game.apply_action(*args, **kwargs)
        self.won = True
        return result

    def observe(self):
        observation = self.game.observe()
        observation["run.flags"][1] = self.won
        return observation


class WinningEnv(GameEnv):
    def reset(self, seed):
        observation = super().reset(seed)
        self.game = WinningGame(self.game)
        return observation


def test_success_stops_single_rollout_without_bootstrapping_or_advancing_endless():
    env = WinningEnv(horizon=10)
    episode = collect_episode(selecting_model(), env, 0, stochastic=False)
    assert episode.end_reason == "win"
    assert episode.accepted_commands == 1 and len(episode.transitions) == 1
    assert episode.bootstrap_value == 0
    assert episode.transitions[-1].terminated and not episode.transitions[-1].truncated
    assert episode.transitions[-1].reward == pytest.approx(25.0)
    assert episode.final_observation["run.flags"][1]
    assert not episode.final_observation["run.flags"][0]
    assert env.game.decision_point != 5  # Python success does not terminate Rust.
    with pytest.raises(RuntimeError, match="reset"):
        env.step(Command(16, order=tuple(range(8))))


def test_same_policy_decoder_traverses_play_shop_pack_and_terminal_boundaries():
    # Deterministic head initialization provides a simple legal-action baseline.
    # Selection still goes through the production conditional decoder and Rust.
    model = GamePolicy(ModelConfig(16, 2, 1))
    with torch.no_grad():
        model.action_head.weight.zero_()
        model.action_head.bias.fill_(-1000)
        for priority, kind in enumerate([0, 2, 7, 13, 12]):
            model.action_head.bias[kind] = 1000 - priority * 100
        model.source_query.weight.zero_()
        model.source_query.bias.zero_()
        model.target_query.weight.zero_()
        model.target_query.bias.zero_()
        model.stop_head.weight.zero_()
        model.stop_head.bias.fill_(-1000)
    episode = collect_episode(model, GameEnv(horizon=100), 0, stochastic=False)
    phases = {
        int(transition.observation["run.cat"][0]) for transition in episode.transitions
    }
    assert phases == {0, 1, 2, 3}
    assert episode.end_reason == "loss"
    assert episode.final_observation["run.flags"][0]
    assert all(transition.accepted for transition in episode.transitions)
    scores = [
        step.completed_blind_score
        for step in episode.transitions
        if step.completed_blind_score is not None
    ]
    assert scores[0] == 472
    assert len(scores) == sum(
        step.observation["run.cat"][0] == 1 and step.next_observation["run.cat"][0] != 1
        for step in episode.transitions
    )
