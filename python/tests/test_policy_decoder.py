import numpy as np
import pytest
import torch
from simulation.decoder import decode_action, evaluate_actions
from simulation.policy import GamePolicy, ModelConfig

import balatro_engine as engine


@pytest.fixture(scope="module", autouse=True)
def single_threaded():
    previous = torch.get_num_threads()
    torch.set_num_threads(1)
    yield
    torch.set_num_threads(previous)


def playing_game(deck=engine.Deck.Red):
    game = engine.new(0, deck=deck)
    game.apply_action(engine.Action.SelectBlind, source=0)
    return game


def forced_output(model, observation, kind):
    output = model([observation])
    output.action_logits = torch.full_like(output.action_logits, -1000)
    output.action_logits[0, kind] = 1000
    return output


@pytest.mark.parametrize("kind", [2, 3, 15, 16])
def test_selected_targets_counts_and_permutations_follow_rust_constraints(kind):
    torch.manual_seed(10)
    policy = GamePolicy(ModelConfig(16, 2, 1))
    game = playing_game()
    observation = game.observe()
    output = forced_output(policy, observation, kind)
    decoded = decode_action(policy, output, game, observation)
    command = decoded.trace.command
    assert command.kind == kind
    selected = command.order if command.order is not None else command.targets
    assert len(set(selected)) == len(selected)
    if kind in (2, 3):
        assert 1 <= len(selected) <= 5
        assert list(selected) == sorted(selected)
    else:
        assert sorted(selected) == list(range(8 if kind == 16 else 0))
    logs, entropies = evaluate_actions(policy, output, [decoded.trace])
    torch.testing.assert_close(logs[0], decoded.log_prob)
    torch.testing.assert_close(entropies[0], decoded.entropy)
    assert decoded.trace.choices[-1].selected == (8 if kind in (2, 3, 16) else 0)
    command.apply(game)


def test_joint_likelihood_replay_matches_after_variable_length_minibatch_padding():
    torch.manual_seed(99)
    policy = GamePolicy(ModelConfig(16, 2, 1)).eval()
    games = [playing_game(), playing_game(engine.Deck.Painted), engine.new(3)]
    observations = [game.observe() for game in games]
    decoded = []
    for game, observation in zip(games, observations):
        output = policy([observation])
        decoded.append(decode_action(policy, output, game, observation))
    output = policy(observations)
    logs, entropy = evaluate_actions(
        policy, output, [action.trace for action in decoded]
    )
    torch.testing.assert_close(
        logs, torch.stack([action.log_prob for action in decoded]), atol=2e-5, rtol=1e-5
    )
    torch.testing.assert_close(
        entropy,
        torch.stack([action.entropy for action in decoded]),
        atol=2e-5,
        rtol=1e-5,
    )
    loss = -(logs.mean() + entropy.mean() * 0.01)
    loss.backward()
    assert policy.action_head.weight.grad.abs().sum() > 0


def test_source_choice_conditions_later_scores_and_preserves_source_identity():
    policy = GamePolicy(ModelConfig(16, 2, 1))
    game = engine.new(0, deck=engine.Deck.Magic)
    observation = game.observe()
    output = forced_output(policy, observation, 11)
    decoded = decode_action(policy, output, game, observation)
    assert decoded.trace.command.source in (0, 1)
    assert decoded.trace.source_known
    assert decoded.trace.choices[1].collection == "consumables"
    decoded.trace.command.apply(game)
    assert len(game.observe()["consumables.present"]) == 1
    rows = output.objects(0, "consumables")
    query = policy.query(output, 0, 9)
    # Two distinct inventory rows produce different conditional query states.
    assert not torch.allclose(
        policy.history(rows[0], query), policy.history(rows[1], query)
    )


class MandatoryTarget:
    """Public constraint-provider fixture requiring the last hand slot."""

    def __init__(self, game):
        self.game = game

    def action_constraints(self, action, source=None):
        return self.game.action_constraints(action, source=source)

    def selection_constraints(self, action, prefix, source=None):
        result = self.game.selection_constraints(action, prefix, source=source)
        result["required_targets"][-1] = True
        result["can_submit"][0] &= 7 in prefix
        if 7 not in prefix and len(prefix) == 4:
            result["target_mask"][:] = False
            result["target_mask"][7] = True
        return result


def test_mandatory_target_cannot_be_skipped_by_stop_or_maximum_count():
    policy = GamePolicy(ModelConfig(16, 2, 1))
    game = playing_game()
    observation = game.observe()
    for seed in range(10):
        torch.manual_seed(seed)
        output = forced_output(policy, observation, 2)
        decoded = decode_action(policy, output, MandatoryTarget(game), observation)
        assert 7 in decoded.trace.command.targets
        assert len(decoded.trace.command.targets) <= 5
        logs, _ = evaluate_actions(policy, output, [decoded.trace])
        torch.testing.assert_close(logs[0], decoded.log_prob)


def test_ordered_consumable_preserves_sampled_direction_and_replays_prefixes():
    # This fixture provides an authoritative directed pair over a real hand;
    # command field construction must preserve direction rather than sorting.
    class DirectedPair:
        def action_constraints(self, action, source=None):
            return {
                "source_mask": np.array([True]),
                "source_known": np.array([True]),
                "permutation_collection": np.array([0]),
                "target_collection": np.array([1]),
                "can_submit": np.array([False]),
            }

        def selection_constraints(self, action, prefix, source=None):
            mask = np.zeros(8, dtype=np.bool_)
            if len(prefix) < 2:
                mask[5 if not len(prefix) else 1] = True
            return {
                "target_mask": mask,
                "can_submit": np.array([len(prefix) == 2]),
                "ordered": np.array([True]),
            }

    policy = GamePolicy(ModelConfig(16, 2, 1))
    game = playing_game(engine.Deck.Magic)
    obs = game.observe()
    # The directed-source fixture supplies just slot zero.
    for suffix in ("cat", "num", "flags", "present", "visible"):
        obs[f"consumables.{suffix}"] = obs[f"consumables.{suffix}"][:1].copy()
    obs["action_kinds"][9] = True
    output = forced_output(policy, obs, 9)
    decoded = decode_action(policy, output, DirectedPair(), obs)
    assert decoded.trace.command.targets == (5, 1)
    assert [choice.prefix for choice in decoded.trace.choices[2:]] == [(), (5,), (5, 1)]
    logs, _ = evaluate_actions(policy, output, [decoded.trace])
    torch.testing.assert_close(logs[0], decoded.log_prob)


def test_terminal_state_has_no_distribution_instead_of_nan_probabilities():
    policy = GamePolicy(ModelConfig(16, 2, 1))
    game = playing_game()
    observation = game.observe()
    observation["action_kinds"][:] = False
    with pytest.raises(ValueError, match="empty"):
        decode_action(policy, policy([observation]), game, observation)


@pytest.mark.parametrize(
    "fields",
    [
        {"kind": -1},
        {"kind": 18},
        {"kind": True},
        {"kind": 2, "targets": (0.5,)},
        {"kind": 2, "targets": (True,)},
        {"kind": 2, "targets": (-1,)},
        {"kind": 2, "targets": (2**31,)},
        {"kind": 2, "targets": [0]},
        {"kind": 0, "source": np.int32(0)},
    ],
)
def test_command_transport_does_not_coerce_invalid_indices_or_mutable_payloads(fields):
    from simulation.decoder import Command

    with pytest.raises((TypeError, ValueError)):
        Command(**fields)
