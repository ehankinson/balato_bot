import numpy as np
import pytest
from simulation.decoder import Command, pack_commands
from simulation.encoder import unbatch_observations

import balatro_engine as engine


def batch(seeds=(0, 1), decks=None):
    return engine.new_batch(np.array(seeds, dtype=np.uint64), decks=decks)


def same(left, right):
    assert left.keys() == right.keys()
    for key in left:
        np.testing.assert_array_equal(left[key], right[key], err_msg=key)


def test_batch_observations_match_single_games_with_typed_padding_and_zero_width_tables():
    decks = [engine.Deck.Magic, engine.Deck.Painted, engine.Deck.Red]
    games = batch((0, 3, 1), decks)
    singles = [engine.new(seed, deck=deck) for seed, deck in zip((0, 3, 1), decks)]
    packed = games.observe()
    assert len(games) == 3
    for actual, single in zip(unbatch_observations(packed), singles):
        same(actual, single.observe())
    for name, array in packed.items():
        assert array.flags.c_contiguous and array.flags.aligned, name
        assert array.dtype in (np.int32, np.int64, np.float32, np.bool_), name
    assert packed["consumables.cat"].shape == (3, 2, 2)
    assert packed["vouchers.num"].shape == (3, 1, 0)
    assert not packed["consumables.present"][1:].any()
    assert not packed["consumables.visible"][1:].any()
    assert packed["lengths.consumables"].tolist() == [2, 0, 0]


def test_seeded_batch_actions_match_single_trajectories_and_are_independent():
    decks = [engine.Deck.Red, engine.Deck.Painted, engine.Deck.Magic]
    games = batch((0, 0, 3), decks)
    singles = [engine.new(seed, deck=deck) for seed, deck in zip((0, 0, 3), decks)]
    rounds = [
        [Command(0, source=0)] * 3,
        [
            Command(3, targets=(0, 1)),
            Command(16, order=tuple(range(9, -1, -1))),
            Command(11, source=0),
        ],
        [Command(2, targets=(0, 1, 2)), None, Command(3, targets=(0,))],
    ]
    for commands in rounds:
        result = games.step(pack_commands(commands))
        for index, command in enumerate(commands):
            if command is not None:
                command.apply(singles[index])
                assert result["accepted"][index]
            else:
                assert not result["accepted"][index] and not result["attempted"][index]
        assert result["blind_scores"].dtype == np.uint64
        assert result["blind_scores"].shape == (len(singles),)
        assert result["blind_scores"].tolist() == [game.blind_score for game in singles]
        for actual, single in zip(
            unbatch_observations(result["observations"]), singles
        ):
            same(actual, single.observe())


def test_rule_rejections_are_per_slot_and_structurally_valid_siblings_can_advance():
    games = batch((0, 1, 2))
    before = unbatch_observations(games.observe())
    result = games.step(
        pack_commands([Command(0, source=0), Command(2, targets=(0,)), None])
    )
    assert result["accepted"].tolist() == [True, False, False]
    assert result["attempted"].tolist() == [True, True, False]
    assert (
        result["errors"][0] is None
        and result["errors"][1]
        and result["errors"][2] is None
    )
    snapshots = unbatch_observations(result["observations"])
    same(before[1], snapshots[1])
    same(before[2], snapshots[2])
    assert snapshots[0]["run.cat"][0] == 1


def test_command_packing_cannot_drop_a_surplus_order_field():
    with pytest.raises(ValueError, match="both targets and order"):
        pack_commands([Command(2, targets=(0,), order=(0,))])


@pytest.mark.parametrize(
    "commands",
    [
        np.array([[0, 0, -1, -1]], dtype=np.int32),
        np.array([[0, 0, -1, -1]] * 2, dtype=np.int64),
        np.array([[0, 0, -1, -1]] * 2, dtype=np.float32),
        np.array([0, 0, -1, -1] * 2, dtype=np.int32),
        np.array([[0, 0, -1]] * 2, dtype=np.int32),
        np.array([[0, 0, -1, -1], [-2, -1, -1, -1]], dtype=np.int32),
        np.array([[0, 0, -1, -1], [-1, 0, -1, -1]], dtype=np.int32),
        np.array([[0, 0, -1, -1, -1], [2, -1, 1, 1, 0]], dtype=np.int32),
        np.array([[0, 0, -1, -1, -1], [2, -1, 2, -1, 0]], dtype=np.int32),
        np.array([[0, 0, -1, -1, -1], [2, -1, 1, -1, -1]], dtype=np.int32),
        np.array([[0, 0, -1, -1, 1], [-1, -1, -1, -1, -1]], dtype=np.int32),
        np.zeros((2, 8), dtype=np.int32)[:, ::2],
    ],
)
def test_malformed_batch_commands_fail_before_mutating_any_slot(commands):
    games = batch()
    before = games.observe()
    with pytest.raises((ValueError, TypeError)):
        games.step(commands)
    after = games.observe()
    same(before, after)


@pytest.mark.parametrize(
    "seeds",
    [
        [],
        [0],
        np.array([0], dtype=np.int64),
        np.array([True]),
        np.array([0.0]),
        np.array(0, dtype=np.uint64),
        np.array([], dtype=np.uint64),
        np.arange(4, dtype=np.uint64)[::2],
    ],
)
def test_seed_boundary_rejects_wrong_types_shapes_empty_and_strided_arrays(seeds):
    with pytest.raises((TypeError, ValueError)):
        engine.new_batch(seeds)


def test_rule_lengths_and_reset_arguments_are_validated_atomically():
    with pytest.raises(ValueError):
        engine.new_batch(np.array([0, 1], dtype=np.uint64), decks=[engine.Deck.Red])
    games = batch((0, 1), [engine.Deck.Magic, engine.Deck.Painted])
    before = games.observe()
    for slots, seeds in [
        ([0, 0], [3, 4]),
        ([0, 2], [3, 4]),
        ([-1], [3]),
        ([0], [3, 4]),
    ]:
        with pytest.raises(ValueError):
            games.reset(
                np.array(slots, dtype=np.int32), np.array(seeds, dtype=np.uint64)
            )
        same(before, games.observe())
    with pytest.raises(TypeError):
        games.reset(np.array([0], dtype=np.int64), np.array([3], dtype=np.uint64))
    games.step(pack_commands([Command(11, source=0), Command(0, source=0)]))
    retained = games.observe()
    expected = {key: value.copy() for key, value in retained.items()}
    games.reset(np.array([0], dtype=np.int32), np.array([7], dtype=np.uint64))
    snapshots = unbatch_observations(games.observe())
    same(snapshots[0], engine.new(7, deck=engine.Deck.Magic).observe())
    same(snapshots[1], unbatch_observations(retained)[1])
    same(retained, expected)


def test_batched_conditional_constraints_match_single_masks_and_prefix_indices():
    decks = [engine.Deck.Red, engine.Deck.Painted]
    games = batch((0, 1), decks)
    singles = [engine.new(seed, deck=deck) for seed, deck in zip((0, 1), decks)]
    commands = [Command(0, source=0)] * 2
    games.step(pack_commands(commands))
    for single in singles:
        commands[0].apply(single)
    result = games.constraints(
        np.array([2, 16], dtype=np.int32),
        np.array([-1, -1], dtype=np.int32),
        np.array([[2, 0, 2], [2, 1, 0]], dtype=np.int32),
    )
    for index, (action, prefix) in enumerate(
        [(engine.Action.PlayCards, [0, 2]), (engine.Action.ReorderHand, [1, 0])]
    ):
        expected = singles[index].selection_constraints(
            action, np.array(prefix, dtype=np.int32)
        )
        for key, array in expected.items():
            length = (
                int(result[f"lengths.{key}"][index])
                if f"lengths.{key}" in result
                else len(array)
            )
            np.testing.assert_array_equal(
                result[key][index, :length], array, err_msg=key
            )
    before = games.observe()
    with pytest.raises(ValueError):
        games.constraints(
            np.array([2, 16], dtype=np.int32),
            np.array([-1, -1], dtype=np.int32),
            np.array([[2, 0, 0], [0, -1, -1]], dtype=np.int32),
        )
    same(before, games.observe())


@pytest.mark.parametrize("observe", [True, False])
@pytest.mark.parametrize("profile", [True, False])
def test_blind_scores_include_final_hands_and_idle_rejected_and_reset_slots(
    observe, profile
):
    games = batch((0, 1, 0, 3))

    def step(commands):
        result = games.step(pack_commands(commands), observe=observe, profile=profile)
        scores = result["blind_scores"]
        assert scores.dtype == np.uint64
        assert scores.shape == (4,)
        assert scores.flags.c_contiguous and scores.flags.aligned
        assert (result["observations"] is not None) == observe
        assert ("timings" in result) == profile
        return result

    assert step([None] * 4)["blind_scores"].tolist() == [0, 0, 0, 0]
    selected = step([Command(0, source=0)] * 3 + [None])
    assert selected["blind_scores"].tolist() == [0, 0, 0, 0]
    play = Command(2, targets=(0, 1, 2, 3, 4))
    first = step([play] * 3 + [None])
    assert first["blind_scores"].tolist() == [44, 16, 44, 0]
    for winning_score, losing_score in [(76, 112), (136, 144), (472, 252)]:
        result = step([play, play, None, None])
        assert result["blind_scores"].tolist() == [winning_score, losing_score, 44, 0]
        assert result["accepted"].tolist() == [True, True, False, False]
    assert result["phases"].tolist() == [2, 5, 1, 0]
    if observe:
        assert not result["observations"]["blind.visible"][:2].any()
        assert not result["observations"]["blind.num"][:2].any()

    rejected = step([play, play, Command(2, targets=(0, 0)), None])
    assert not rejected["accepted"].any()
    assert rejected["attempted"].tolist() == [True, True, True, False]
    assert all(rejected["errors"][:3]) and rejected["errors"][3] is None
    assert rejected["blind_scores"].tolist() == [472, 252, 44, 0]
    idle = step([None] * 4)
    assert not idle["accepted"].any() and not idle["attempted"].any()
    assert idle["blind_scores"].tolist() == [472, 252, 44, 0]

    # Each result owns its scores; subsequent play/reset cannot rewrite a snapshot.
    assert first["blind_scores"].tolist() == [44, 16, 44, 0]
    games.reset(np.array([0, 1], dtype=np.int32), np.array([0, 1], dtype=np.uint64))
    assert step([None] * 4)["blind_scores"].tolist() == [0, 0, 44, 0]
    assert result["blind_scores"].tolist() == [472, 252, 44, 0]
    selected = step([Command(0, source=0), None, None, None])
    assert selected["blind_scores"].tolist() == [0, 0, 44, 0]
    assert step([play, None, None, None])["blind_scores"].tolist() == [44, 0, 44, 0]


def test_blind_scores_persist_in_shop_pack_and_selection_then_clear_on_next_blind():
    games = batch((0,))
    games.step(pack_commands([Command(0, source=0)]))
    for _ in range(4):
        result = games.step(pack_commands([Command(2, targets=(0, 1, 2, 3, 4))]))
    assert result["phases"].tolist() == [2]
    assert result["blind_scores"].tolist() == [472]
    for command, phase in [
        (Command(7, source=0), 3),
        (Command(13), 2),
        (Command(12), 0),
    ]:
        result = games.step(pack_commands([command]), observe=False)
        assert result["accepted"].tolist() == [True]
        assert result["phases"].tolist() == [phase]
        assert result["blind_scores"].tolist() == [472]
    result = games.step(pack_commands([Command(0, source=1)]), observe=False)
    assert result["phases"].tolist() == [1]
    assert result["blind_scores"].tolist() == [0]


def test_blind_scores_are_detached_and_malformed_commands_preserve_nonzero_scores():
    games = batch((0, 1))
    games.step(pack_commands([Command(0, source=0)] * 2))
    result = games.step(pack_commands([Command(2, targets=(0, 1, 2, 3, 4))] * 2))
    assert result["blind_scores"].tolist() == [44, 16]
    result["blind_scores"][:] = np.iinfo(np.uint64).max
    idle = pack_commands([None, None])
    assert games.step(idle)["blind_scores"].tolist() == [44, 16]

    malformed = pack_commands([Command(2, targets=(0,)), None])
    malformed[1, 0] = -2
    with pytest.raises(ValueError):
        games.step(malformed)
    assert games.step(idle)["blind_scores"].tolist() == [44, 16]

    # Field decoding fails per slot, like game-rule rejections, without losing metrics.
    result = games.step(pack_commands([Command(2), None]))
    assert result["errors"][0] and result["errors"][1] is None
    assert result["blind_scores"].tolist() == [44, 16]


def test_terminal_shop_pack_and_selection_coexist_without_automatic_resets():
    games = batch((1, 0, 0, 3))
    games.step(pack_commands([Command(0, source=0)] * 3 + [None]))
    for _ in range(4):
        snapshots = unbatch_observations(games.observe())
        games.step(
            pack_commands(
                [
                    Command(2, targets=(0, 1, 2, 3, 4))
                    if snapshot["run.cat"][0] == 1
                    else None
                    for snapshot in snapshots
                ]
            )
        )
    games.step(pack_commands([None, None, Command(7, source=0), None]))
    old = games.observe()
    assert old["run.cat"][:, 0].tolist() == [5, 2, 3, 0]
    assert not old["action_kinds"][0].any()
    result = games.step(
        pack_commands([Command(2, targets=(0,)), None, None, None]), profile=True
    )
    assert not result["accepted"].any() and result["errors"][0]
    same(old, result["observations"])
    assert all(value >= 0 for value in result["timings"].values())
    assert games.step(pack_commands([None] * 4), observe=False)["observations"] is None
    games.reset(np.array([0], dtype=np.int32), np.array([5], dtype=np.uint64))
    assert games.observe()["run.cat"][0, 0] == 0
    assert old["run.cat"][0, 0] == 5
