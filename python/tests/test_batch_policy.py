import numpy as np
import pytest
import torch

import balatro_engine as engine
from simulation.decoder import (
    Command,
    decode_action_batch,
    evaluate_actions,
    pack_commands,
)
from simulation.encoder import unbatch_observations
from simulation.env import BatchEnv, GameEnv
from simulation.policy import GamePolicy, ModelConfig
from simulation.ppo import update_policy
from simulation.rollout import collect_episode, collect_episodes, collect_runs
from simulation.training_config import TrainingConfig


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


def reorder_preferring_policy():
    model = baseline()
    with torch.no_grad():
        model.action_head.bias[15] = 2000
        model.action_head.bias[16] = 1900
    return model


def playing_batch_with_jokers():
    env = BatchEnv(
        [79, 0, 1], decks=[engine.Deck.Yellow, engine.Deck.Red, engine.Deck.Painted]
    )
    assert (
        env.step(pack_commands([Command(0, source=0), None, None])).results[0].accepted
    )
    for _ in range(4):
        if env.snapshots[0]["run.cat"][0] != 1:
            break
        result = env.step(
            pack_commands([Command(2, targets=(0, 1, 2, 3, 4)), None, None])
        )
        assert result.results[0].accepted
    assert env.snapshots[0]["run.cat"][0] == 2
    for _ in range(2):
        assert (
            env.step(pack_commands([Command(4, source=0), None, None]))
            .results[0]
            .accepted
        )
    assert len(env.snapshots[0]["jokers.present"]) == 2
    assert env.step(pack_commands([Command(12), None, None])).results[0].accepted
    results = env.step(
        pack_commands(
            [Command(0, source=1), Command(0, source=0), Command(0, source=0)]
        )
    ).results
    assert all(result.accepted for result in results)
    return env


def test_mixed_length_identity_prefixes_deviate_and_replay_local_saved_masks():
    torch.manual_seed(51)
    model = reorder_preferring_policy()
    env = playing_batch_with_jokers()
    observations = env.snapshots
    before = [{key: value.copy() for key, value in obs.items()} for obs in observations]
    batch_observations = env.observations
    batch_before = {key: value.copy() for key, value in batch_observations.items()}
    output = model(observations)
    decoded = decode_action_batch(
        model, output, env.games, observations, env.active, stochastic=False
    )
    for slot, (kind, length) in enumerate([(15, 2), (16, 8), (16, 10)]):
        trace = decoded.traces[slot]
        order = (*range(length - 2), length - 1, length - 2)
        assert trace.command.kind == kind and trace.command.order == order
        selections = trace.choices[1:]
        assert len(selections) == length + 1
        for index, choice in enumerate(selections):
            assert choice.prefix == order[:index]
            expected_mask = np.ones(length + 1, dtype=np.bool_)
            expected_mask[list(choice.prefix)] = False
            expected_mask[-1] = index == length
            if index == length - 2:
                expected_mask[length - 2] = False
            np.testing.assert_array_equal(choice.mask, expected_mask)
            assert choice.mask[choice.selected] and not choice.mask.flags.writeable
        assert selections[-1].selected == length
        assert not trace.choices[0].mask.flags.writeable
        assert not np.shares_memory(
            trace.choices[0].mask, observations[slot]["action_kinds"]
        )
        expected_entropy = sum(np.log(choice.mask.sum()) for choice in selections)
        assert decoded.log_probs[slot].item() == pytest.approx(
            -expected_entropy, abs=2e-6
        )
        assert decoded.entropies[slot].item() == pytest.approx(
            expected_entropy, abs=2e-6
        )
    for key, value in batch_before.items():
        np.testing.assert_array_equal(env.observations[key], value, err_msg=key)
    assert all(result.accepted for result in env.step(decoded.commands).results)
    raw_observations = env.games.observe()
    for slot, trace in enumerate(decoded.traces):
        assert not env.snapshots[slot]["action_kinds"][trace.command.kind]
        assert not env.observations["action_kinds"][slot, trace.command.kind]
        assert raw_observations["action_kinds"][slot, trace.command.kind]
    for key, value in batch_before.items():
        np.testing.assert_array_equal(batch_observations[key], value, err_msg=key)
    # Current masks forbid the actions, but replay must use the original masks.
    logs, entropy = evaluate_actions(model, model(observations), decoded.traces)
    torch.testing.assert_close(logs, decoded.log_probs)
    torch.testing.assert_close(entropy, decoded.entropies)
    for obs, expected in zip(observations, before):
        for key, value in expected.items():
            np.testing.assert_array_equal(obs[key], value, err_msg=key)


def test_batch_policy_tracks_each_reorder_collection_through_rejection_and_reset():
    torch.manual_seed(52)
    model = reorder_preferring_policy()
    env = playing_batch_with_jokers()
    retained = []

    def step_first(command):
        retained.extend(
            (obs, {key: value.copy() for key, value in obs.items()})
            for obs in env.snapshots
        )
        return env.step(pack_commands([command, None, None])).results[0]

    def greedy_first():
        return (
            decode_action_batch(
                model,
                model(env.snapshots),
                env.games,
                env.snapshots,
                np.array([True, False, False]),
                stochastic=False,
            )
            .traces[0]
            .command
        )

    first = greedy_first()
    assert first.kind == 15 and first.order == (1, 0)
    assert step_first(first).accepted
    assert not env.snapshots[0]["action_kinds"][15]
    assert env.snapshots[0]["action_kinds"][16]
    second = greedy_first()
    assert second.kind == 16
    assert step_first(second).accepted
    assert not env.snapshots[0]["action_kinds"][[15, 16]].any()
    rejected = step_first(Command(2, targets=(99,)))
    assert not rejected.accepted and rejected.error
    assert not rejected.observation["action_kinds"][[15, 16]].any()
    assert greedy_first().kind == 2
    assert env.games.observe()["action_kinds"][0, [15, 16]].all()
    assert not env.observations["action_kinds"][0, [15, 16]].any()
    # Resetting another slot must preserve restrictions in the returned batch too.
    reset_observations = env.reset([1], [8])
    assert not reset_observations["action_kinds"][0, [15, 16]].any()
    assert not env.snapshots[0]["action_kinds"][[15, 16]].any()
    for returned, snapshot in zip(
        unbatch_observations(reset_observations), env.snapshots
    ):
        for key in snapshot:
            np.testing.assert_array_equal(returned[key], snapshot[key], err_msg=key)
    assert step_first(Command(3, targets=(0,))).accepted
    assert env.snapshots[0]["action_kinds"][[15, 16]].all()
    assert greedy_first().kind == 15
    assert step_first(greedy_first()).accepted
    env.reset([0], [79])
    assert not env.snapshots[0]["action_kinds"][[15, 16]].any()
    assert step_first(Command(0, source=0)).accepted
    assert env.snapshots[0]["action_kinds"][16]
    for observation, expected in retained:
        for key, value in expected.items():
            np.testing.assert_array_equal(observation[key], value, err_msg=key)


@pytest.mark.parametrize("joker_count", [0, 1])
def test_batch_decoder_masks_identity_only_joker_reorders(joker_count):
    torch.manual_seed(53)
    model = reorder_preferring_policy()
    env = playing_batch_with_jokers()
    for _ in range(2 - joker_count):
        assert (
            env.step(pack_commands([Command(10, source=0), None, None]))
            .results[0]
            .accepted
        )
    assert len(env.snapshots[0]["jokers.present"]) == joker_count
    raw_observations = env.games.observe()
    assert raw_observations["action_kinds"][0, 15]
    assert not env.observations["action_kinds"][0, 15]
    # Pass raw engine snapshots to exercise the decoder's own row-count guard.
    observations = unbatch_observations(raw_observations)
    decoded = decode_action_batch(
        model,
        model(observations),
        env.games,
        observations,
        env.active,
        stochastic=False,
    )
    assert all(trace.command.kind == 16 for trace in decoded.traces)
    assert all(not trace.choices[0].mask[15] for trace in decoded.traces)
    assert observations[0]["action_kinds"][15]
    assert all(result.accepted for result in env.step(decoded.commands).results)


def test_reorder_preferring_greedy_batch_rollout_progresses_and_stops():
    torch.manual_seed(54)
    model = reorder_preferring_policy()
    env = BatchEnv(
        [0, 1, 3],
        decks=[engine.Deck.Red, engine.Deck.Painted, engine.Deck.Magic],
        horizon=60,
    )
    episodes = collect_episodes(model, env, stochastic=False)
    assert not env.active.any()
    for episode in episodes:
        assert episode.end_reason in {"loss", "win", "horizon"}
        assert 1 < episode.accepted_commands <= 60
        kinds = [step.trace.command.kind for step in episode.transitions]
        assert kinds[0] == 0 and 16 in kinds and kinds.count(2) >= 2
        reordered = set()
        for step in episode.transitions:
            assert step.accepted
            for kind, collection in [(15, "jokers"), (16, "hand")]:
                if (
                    kind in reordered
                    or len(step.observation[f"{collection}.present"]) < 2
                ):
                    assert not step.observation["action_kinds"][kind]
                    assert not step.trace.choices[0].mask[kind]
            kind = step.trace.command.kind
            if kind in (15, 16):
                assert kind not in reordered
                assert step.trace.command.order != tuple(
                    range(len(step.trace.command.order))
                )
                reordered.add(kind)
                assert not step.next_observation["action_kinds"][kind]
            else:
                reordered.clear()


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


def test_batch_completed_blind_scores_include_exit_hand_but_not_idle_or_cutoffs():
    env = BatchEnv([0, 1, 2], horizon=5)
    env.step(pack_commands([Command(0, source=0)] * 3))
    for _ in range(3):
        result = env.step(
            pack_commands(
                [
                    Command(2, targets=tuple(range(5))),
                    Command(2, targets=tuple(range(5))),
                    Command(16, order=tuple(range(8))),
                ]
            )
        )
        assert all(step.completed_blind_score is None for step in result.results)
    result = env.step(
        pack_commands(
            [
                Command(2, targets=tuple(range(5))),
                Command(2, targets=tuple(range(5))),
                Command(2, targets=tuple(range(5))),
            ]
        )
    )
    assert [step.completed_blind_score for step in result.results] == [472, 252, None]
    assert result.results[
        0
    ].truncated  # A completed blind at the episode cutoff counts.
    assert result.results[1].terminated
    assert result.results[2].truncated  # An unfinished blind at the cutoff does not.
    assert env.step(pack_commands([None] * 3)).results == (None,) * 3
    env.reset([0], [0])
    result = env.step(pack_commands([Command(0, source=0), None, None]))
    assert result.results[0].completed_blind_score is None


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
                and a.completed_blind_score == b.completed_blind_score
            )
            for key in a.observation:
                np.testing.assert_array_equal(
                    a.observation[key], b.observation[key], err_msg=key
                )
    phases = {int(step.observation["run.cat"][0]) for step in actual[0].transitions}
    assert {0, 1, 2, 3}.issubset(phases)


def test_finished_slots_are_idle_until_explicit_reset_and_snapshots_survive():
    torch.manual_seed(55)
    model = reorder_preferring_policy()
    env = BatchEnv([0, 1], horizon=2)
    episodes = collect_episodes(model, env, stochastic=False)
    assert all(
        [step.trace.command.kind for step in episode.transitions] == [0, 16]
        for episode in episodes
    )
    assert env.games.observe()["action_kinds"][:, 16].all()
    assert not env.observations["action_kinds"][:, [15, 16]].any()
    assert all(not obs["action_kinds"][[15, 16]].any() for obs in env.snapshots)
    final = episodes[0].final_observation
    expected = {key: array.copy() for key, array in final.items()}
    idle = env.step(pack_commands([None, None]))
    assert idle.results == (None, None)
    assert not env.observations["action_kinds"][:, [15, 16]].any()
    assert all(not obs["action_kinds"][[15, 16]].any() for obs in env.snapshots)
    with pytest.raises(RuntimeError, match="reset"):
        env.step(pack_commands([Command(15, order=()), None]))
    reset_observations = env.reset([0], [8])
    assert not reset_observations["action_kinds"][:, [15, 16]].any()
    for returned, snapshot in zip(
        unbatch_observations(reset_observations), env.snapshots
    ):
        for key in snapshot:
            np.testing.assert_array_equal(returned[key], snapshot[key], err_msg=key)
    assert env.active.tolist() == [True, False]
    assert all(not obs["action_kinds"][[15, 16]].any() for obs in env.snapshots)
    output = model(env.snapshots)
    decoded = decode_action_batch(
        model, output, env.games, env.snapshots, env.active, stochastic=False
    )
    assert decoded.traces[1] is None and (decoded.commands[1] == -1).all()
    assert decoded.traces[0].command.kind == 0
    torch.testing.assert_close(decoded.log_probs[1], torch.tensor(0.0))
    torch.testing.assert_close(decoded.entropies[1], torch.tensor(0.0))
    env.step(decoded.commands)
    assert env.snapshots[0]["action_kinds"][16]
    assert not env.snapshots[1]["action_kinds"][[15, 16]].any()
    for key, value in expected.items():
        np.testing.assert_array_equal(final[key], value, err_msg=key)


class WinningBatch:
    """Expose a synthetic win for slot zero after its first accepted command."""

    def __init__(self, games):
        self.games = games
        self.won = False

    def __getattr__(self, name):
        return getattr(self.games, name)

    def step(self, commands, profile=False):
        result = self.games.step(commands, profile=profile)
        self.won |= bool(result["accepted"][0])
        result["observations"]["run.flags"][0, 1] = self.won
        return result


def test_winning_batch_slot_stops_and_zero_bootstraps_while_other_slots_continue():
    env = BatchEnv([0, 1], horizon=3)
    env.games = WinningBatch(env.games)
    episodes = collect_episodes(baseline(), env, stochastic=False)
    won, cutoff = episodes
    assert won.end_reason == "win" and won.accepted_commands == 1
    assert len(won.transitions) == 1 and won.bootstrap_value == 0
    assert won.transitions[-1].terminated and not won.transitions[-1].truncated
    assert won.transitions[-1].reward == pytest.approx(25.0)
    assert won.final_observation["run.flags"][1]
    assert not won.final_observation["run.flags"][0]
    assert cutoff.end_reason == "horizon" and cutoff.accepted_commands == 3
    assert not env.active.any()
    with pytest.raises(RuntimeError, match="reset"):
        env.step(pack_commands([Command(15, order=()), None]))


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
def test_batch_rollouts_feed_a_finite_training_update_and_have_bounded_chunks(device):
    torch.manual_seed(41)
    model = GamePolicy(ModelConfig(16, 2, 1)).to(device)
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
