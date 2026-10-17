import numpy as np
import pytest
import torch

import balatro_engine as engine
from simulation.decoder import decode_action, evaluate_actions
from simulation.policy import GamePolicy, ModelConfig
from simulation.policy_actions import action_kind_mask, policy_observation


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


def two_joker_game():
    game = engine.new(79, deck=engine.Deck.Yellow)
    game.apply_action(engine.Action.SelectBlind, source=0)
    for _ in range(4):
        if game.decision_point != 1:
            break
        game.apply_action(engine.Action.PlayCards, targets=np.arange(5, dtype=np.int32))
    assert game.decision_point == 2
    for _ in range(2):
        game.apply_action(engine.Action.BuyShopItem, source=0)
    assert len(game.observe()["jokers.present"]) == 2
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
    game = two_joker_game() if kind == 15 else playing_game()
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
        length = len(observation[f"{'hand' if kind == 16 else 'jokers'}.present"])
        assert sorted(selected) == list(range(length))
        assert selected != tuple(range(length))
    logs, entropies = evaluate_actions(policy, output, [decoded.trace])
    torch.testing.assert_close(logs[0], decoded.log_prob)
    torch.testing.assert_close(entropies[0], decoded.entropy)
    assert decoded.trace.choices[-1].selected == (2 if kind == 15 else 8)
    command.apply(game)


@pytest.mark.parametrize("kind, collection", [(15, "jokers"), (16, "hand")])
@pytest.mark.parametrize("length", [0, 1, 2])
def test_policy_reorder_kind_masks_require_two_rows_and_preserve_raw_legality(
    kind, collection, length
):
    observation = two_joker_game().observe() if kind == 15 else playing_game().observe()
    # Exercise the policy boundary even when Rust permits an identity-only reorder.
    observation[f"{collection}.present"] = observation[f"{collection}.present"][:length]
    assert observation["action_kinds"][kind]
    before = {key: value.copy() for key, value in observation.items()}
    allowed = policy_observation(observation, ())
    blocked = policy_observation(observation, {kind})
    other_collection = policy_observation(observation, {31 - kind})
    assert bool(allowed["action_kinds"][kind]) == (length >= 2)
    assert not blocked["action_kinds"][kind]
    np.testing.assert_array_equal(
        other_collection["action_kinds"], allowed["action_kinds"]
    )
    np.testing.assert_array_equal(action_kind_mask(blocked), blocked["action_kinds"])
    unchanged = [index for index in range(18) if index not in (15, 16)]
    np.testing.assert_array_equal(
        allowed["action_kinds"][unchanged], observation["action_kinds"][unchanged]
    )
    assert not np.shares_memory(allowed["action_kinds"], observation["action_kinds"])
    for key, value in before.items():
        np.testing.assert_array_equal(observation[key], value, err_msg=key)
    observation["action_kinds"][kind] = False
    assert not action_kind_mask(observation)[kind]


@pytest.mark.parametrize("length", [0, 1])
def test_decoder_masks_identity_only_joker_reorders_despite_preferred_logits(length):
    torch.manual_seed(23)
    game = two_joker_game() if length else engine.new(0)
    if length:
        game.apply_action(engine.Action.SellJoker, source=0)
    observation = game.observe()
    assert len(observation["jokers.present"]) == length
    assert observation["action_kinds"][15]
    before = {key: value.copy() for key, value in observation.items()}
    policy = GamePolicy(ModelConfig(16, 2, 1)).eval()
    output = forced_output(policy, observation, 15)
    decoded = decode_action(policy, output, game, observation, stochastic=False)
    assert decoded.trace.command.kind != 15
    assert not decoded.trace.choices[0].mask[15]
    assert torch.isfinite(decoded.log_prob) and torch.isfinite(decoded.entropy)
    for key, value in before.items():
        np.testing.assert_array_equal(observation[key], value, err_msg=key)
    # This restriction belongs to the policy, not the engine's legal commands.
    game.apply_action(
        engine.Action.ReorderJokers, order=np.arange(length, dtype=np.int32)
    )


@pytest.mark.parametrize(
    "kind, deck, length",
    [
        (16, engine.Deck.Red, 8),
        (16, engine.Deck.Painted, 10),
        (15, engine.Deck.Yellow, 2),
    ],
)
def test_identity_prefix_forces_penultimate_deviation_and_replays_saved_masks(
    kind, deck, length
):
    torch.manual_seed(31)
    policy = GamePolicy(ModelConfig(16, 2, 1)).eval()
    # Equal pointer scores greedily select the lowest remaining index, forcing
    # the identity prefix rather than relying on a random nonidentity sample.
    with torch.no_grad():
        policy.target_query.weight.zero_()
        policy.target_query.bias.zero_()
        policy.stop_head.weight.zero_()
        policy.stop_head.bias.zero_()
    game = two_joker_game() if kind == 15 else playing_game(deck)
    observation = game.observe()
    before = {key: value.copy() for key, value in observation.items()}
    output = forced_output(policy, observation, kind)
    decoded = decode_action(policy, output, game, observation, stochastic=False)
    expected_order = (*range(length - 2), length - 1, length - 2)
    assert decoded.trace.command.order == expected_order
    selections = decoded.trace.choices[1:]
    assert len(selections) == length + 1
    action = engine.Action.ReorderJokers if kind == 15 else engine.Action.ReorderHand
    for index, choice in enumerate(selections):
        assert choice.prefix == expected_order[:index]
        raw = game.selection_constraints(
            action, np.array(choice.prefix, dtype=np.int32)
        )
        expected_mask = np.concatenate((raw["target_mask"], raw["can_submit"]))
        if index == length - 2:
            assert expected_mask[length - 2 : length].all()
            expected_mask[length - 2] = False
        np.testing.assert_array_equal(choice.mask, expected_mask)
        assert choice.mask[choice.selected] and not choice.mask.flags.writeable
    assert selections[-1].selected == length
    # Uniform pointer distributions give an independent likelihood/entropy oracle.
    expected_entropy = sum(np.log(choice.mask.sum()) for choice in selections)
    assert decoded.log_prob.item() == pytest.approx(-expected_entropy, abs=2e-6)
    assert decoded.entropy.item() == pytest.approx(expected_entropy, abs=2e-6)
    decoded.trace.command.apply(game)
    collection = "jokers" if kind == 15 else "hand"
    np.testing.assert_array_equal(
        game.observe()[f"{collection}.cat"],
        observation[f"{collection}.cat"][list(expected_order)],
    )
    # Replay still uses the retained conditional masks after the engine advances.
    logs, entropies = evaluate_actions(policy, output, [decoded.trace])
    torch.testing.assert_close(logs[0], decoded.log_prob, atol=0, rtol=0)
    torch.testing.assert_close(entropies[0], decoded.entropy, atol=0, rtol=0)
    for key, value in before.items():
        np.testing.assert_array_equal(observation[key], value, err_msg=key)
    identity = np.arange(length, dtype=np.int32)
    assert game.selection_constraints(action, identity)["can_submit"][0]
    game.apply_action(action, order=identity)


def test_nonidentity_prefix_keeps_both_penultimate_choices(monkeypatch):
    torch.manual_seed(32)
    policy = GamePolicy(ModelConfig(16, 2, 1)).eval()
    game = playing_game()
    observation = game.observe()

    def prefer_second_slot(rows, query):
        logits = -torch.arange(len(rows) + 1, device=query.device, dtype=query.dtype)
        logits[1] = 1
        return logits

    monkeypatch.setattr(policy, "selection_logits", prefer_second_slot)
    output = forced_output(policy, observation, 16)
    decoded = decode_action(policy, output, game, observation, stochastic=False)
    assert decoded.trace.command.order == (1, 0, 2, 3, 4, 5, 6, 7)
    penultimate = decoded.trace.choices[-3]
    assert penultimate.prefix == (1, 0, 2, 3, 4, 5)
    assert penultimate.mask[6:8].all()
    logs, entropy = evaluate_actions(policy, output, [decoded.trace])
    torch.testing.assert_close(logs[0], decoded.log_prob)
    torch.testing.assert_close(entropy[0], decoded.entropy)
    decoded.trace.command.apply(game)


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
