import numpy as np
import pytest
import torch
from simulation.decoder import (
    Command,
    decode_action_batch,
    evaluate_actions,
    pack_commands,
)
from simulation.env import BatchEnv, GameEnv
from simulation.policy import GamePolicy, ModelConfig
from simulation.ppo import update_policy
from simulation.rollout import collect_episode, collect_episodes, collect_runs
from simulation.training_config import TrainingConfig

import balatro_engine as engine


@pytest.fixture(scope="module", autouse=True)
def single_threaded():
    previous = torch.get_num_threads()
    torch.set_num_threads(1)
    yield
    torch.set_num_threads(previous)


def baseline():
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
    return model.eval()


def test_mixed_source_targets_and_permutation_likelihoods_replay_exactly():
    torch.manual_seed(13)
    model = GamePolicy(ModelConfig(16, 2, 1)).eval()
    env = BatchEnv(
        [0, 1, 3], [engine.Deck.Red, engine.Deck.Painted, engine.Deck.Magic], horizon=20
    )
    env.step(pack_commands([Command(0, source=0), Command(0, source=0), None]))
    output = model(env.snapshots)
    output.action_logits = torch.full_like(output.action_logits, -1000)
    for slot, kind in enumerate([2, 16, 11]):
        output.action_logits[slot, kind] = 1000
    decoded = decode_action_batch(model, output, env.games, env.snapshots, env.active)
    assert decoded.traces[0].command.kind == 2
    assert 1 <= len(decoded.traces[0].command.targets) <= 5
    assert sorted(decoded.traces[1].command.order) == list(range(10))
    assert decoded.traces[2].command.source in (0, 1)
    logs, entropy = evaluate_actions(model, output, decoded.traces)
    torch.testing.assert_close(logs, decoded.log_probs)
    torch.testing.assert_close(entropy, decoded.entropies)
    assert all(result.accepted for result in env.step(decoded.commands).results)


def test_batch_decoding_never_reads_an_individual_tensor_scalar(monkeypatch):
    model = baseline()
    env = BatchEnv([0, 1], horizon=20)
    env.step(pack_commands([Command(0, source=0)] * 2))
    output = model(env.snapshots)

    def forbidden(*args, **kwargs):
        raise AssertionError("individual tensor scalar read")

    with monkeypatch.context() as patch:
        patch.setattr(torch.Tensor, "item", forbidden)
        patch.setattr(torch.Tensor, "__int__", forbidden)
        patch.setattr(torch.Tensor, "__float__", forbidden)
        decoded = decode_action_batch(
            model, output, env.games, env.snapshots, env.active, stochastic=False
        )
    assert decoded.commands.shape == (2, 9)
    assert all(result.accepted for result in env.step(decoded.commands).results)


def test_batch_and_single_greedy_rollouts_match_across_full_game_phases():
    model = baseline()
    decks = [engine.Deck.Red, engine.Deck.Magic, engine.Deck.Painted]
    env = BatchEnv([0, 1, 3], decks=decks, horizon=40)
    actual = collect_episodes(model, env, stochastic=False)
    singles = [
        collect_episode(model, GameEnv(horizon=40, deck=deck), seed, stochastic=False)
        for seed, deck in zip([0, 1, 3], decks)
    ]
    for left, right in zip(actual, singles):
        assert left.end_reason == right.end_reason
        assert left.accepted_commands == right.accepted_commands
        assert len(left.transitions) == len(right.transitions)
        assert left.bootstrap_value == pytest.approx(right.bootstrap_value, abs=1e-6)
        for a, b in zip(left.transitions, right.transitions):
            assert a.trace.command == b.trace.command
            assert a.old_log_prob == pytest.approx(b.old_log_prob, abs=2e-5)
            assert (
                a.reward == b.reward
                and a.terminated == b.terminated
                and a.truncated == b.truncated
            )
            for key in a.observation:
                np.testing.assert_array_equal(
                    a.observation[key], b.observation[key], err_msg=key
                )
    phases = {int(step.observation["run.cat"][0]) for step in actual[0].transitions}
    assert {0, 1, 2, 3}.issubset(phases)


def test_finished_slots_are_idle_until_explicit_reset_and_snapshots_survive():
    model = baseline()
    env = BatchEnv([0, 1], horizon=3)
    episodes = collect_episodes(model, env, stochastic=False)
    final = episodes[0].final_observation
    expected = {key: array.copy() for key, array in final.items()}
    idle = env.step(pack_commands([None, None]))
    assert idle.results == (None, None)
    with pytest.raises(RuntimeError, match="reset"):
        env.step(pack_commands([Command(15, order=()), None]))
    env.reset([0], [8])
    assert env.active.tolist() == [True, False]
    output = model(env.snapshots)
    decoded = decode_action_batch(
        model, output, env.games, env.snapshots, env.active, stochastic=False
    )
    assert decoded.traces[1] is None and (decoded.commands[1] == -1).all()
    env.step(decoded.commands)
    for key, value in expected.items():
        np.testing.assert_array_equal(final[key], value, err_msg=key)


class UncertainRejection:
    def __init__(self, games):
        self.games = games

    def constraints(self, kinds, sources, prefixes):
        result = self.games.constraints(kinds, sources, prefixes)
        result["source_known"][0] = False
        return result

    def step(self, commands, profile=False):
        attempted = commands[0, 0] != -1
        commands = commands.copy()
        commands[0] = -1
        result = self.games.step(commands, profile=profile)
        if attempted:
            result["attempted"][0] = True
            result["errors"][0] = "unavailable action"
        return result


def test_uncertain_rejections_keep_traces_while_other_slots_continue():
    model = baseline()
    env = BatchEnv([0, 1], horizon=3, max_consecutive_rejections=2)
    env.games = UncertainRejection(env.games)
    episodes = collect_episodes(model, env, stochastic=False)
    assert episodes[0].end_reason == "rejection_limit"
    assert episodes[0].accepted_commands == 0
    assert len(episodes[0].transitions) == 2
    assert all(
        not step.accepted and not step.trace.source_known
        for step in episodes[0].transitions
    )
    assert episodes[1].accepted_commands == 3
    assert episodes[1].end_reason == "horizon"
    steps = [episodes[0].transitions[0], episodes[1].transitions[0]]
    logs, _ = evaluate_actions(
        model,
        model([step.observation for step in steps]),
        [step.trace for step in steps],
    )
    torch.testing.assert_close(
        logs, torch.tensor([step.old_log_prob for step in steps])
    )


def test_batch_rollouts_feed_a_finite_training_update_and_have_bounded_chunks():
    torch.manual_seed(41)
    model = GamePolicy(ModelConfig(16, 2, 1))
    episodes = collect_runs(
        model,
        [0, 1, 2],
        [engine.Deck.Red, engine.Deck.Magic, engine.Deck.Painted],
        [engine.Stake.White] * 3,
        horizon=8,
        batch_size=2,
    )
    assert len(episodes) == 3
    metrics = update_policy(
        model,
        torch.optim.Adam(model.parameters()),
        episodes,
        TrainingConfig(ppo_epochs=1, minibatch_size=2),
    )
    assert np.isfinite([metrics.policy_loss, metrics.value_loss, metrics.entropy]).all()
    assert metrics.update_count > 0
    assert all(
        torch.isfinite(parameter.grad).all()
        for parameter in model.parameters()
        if parameter.grad is not None
    )


def test_explicit_prepare_transfer_forward_matches_the_public_model_path():
    model = baseline()
    env = BatchEnv([0, 1], decks=[engine.Deck.Magic, engine.Deck.Painted])
    prepared = model.prepare(env.snapshots)
    output = model.forward_batch(prepared.to("cpu"))
    expected = model(env.snapshots)
    torch.testing.assert_close(output.action_logits, expected.action_logits)
    torch.testing.assert_close(output.value, expected.value)
    assert prepared.features["hand"].values.device.type == "cpu"
