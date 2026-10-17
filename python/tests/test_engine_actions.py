import numpy as np
import pytest

import balatro_engine as engine


def indices(values):
    return np.array(values, dtype=np.int32)


def playing_game():
    game = engine.new(0)
    assert game.apply_action(engine.Action.SelectBlind, source=0) == 1
    return game


def shop_game():
    game = playing_game()
    for _ in range(4):
        phase = game.apply_action(engine.Action.PlayCards, targets=indices([0, 1, 2, 3, 4]))
        if phase != 1:
            break
    assert phase == 2
    return game


def test_engine_action_codes_match_schema_v1():
    names = (
        "SelectBlind", "SkipBlind", "PlayCards", "DiscardCards", "BuyShopItem",
        "BuyVoucher", "RerollShop", "BuyPack", "SelectPackOption", "UseConsumable",
        "SellJoker", "SellConsumable", "LeaveShop", "SkipPack", "UsePackConsumable",
        "ReorderJokers", "ReorderHand", "RerollBoss",
    )
    assert [int(getattr(engine.Action, name)) for name in names] == list(range(18))
    assert not hasattr(engine.Action, "SelectPackOptions")


def test_seeded_control_loop_traverses_shop_and_pack_then_next_blind():
    game = shop_game()
    assert game.apply_action(engine.Action.BuyPack, source=0) == 3
    assert game.next_step() == 3
    assert game.apply_action(engine.Action.SkipPack) == 2
    assert game.apply_action(engine.Action.LeaveShop) == 0
    assert game.blind_index == 1
    assert game.apply_action(engine.Action.SelectBlind, source=1) == 1


@pytest.mark.parametrize("value", [True, 0.0, -1, 2**40])
def test_source_input_rejects_bools_floats_negative_and_overflow(value):
    game = engine.new(0)
    with pytest.raises((TypeError, ValueError, OverflowError)):
        game.apply_action(engine.Action.SelectBlind, source=value)
    assert game.decision_point == 0
    assert game.blind_index == 0
    assert game.apply_action(engine.Action.SelectBlind, source=0) == 1


@pytest.mark.parametrize("value", [True, 0.0, -1, 2**40])
def test_target_input_rejects_bools_floats_negative_and_overflow(value):
    game = playing_game()
    with pytest.raises((TypeError, ValueError, OverflowError)):
        game.apply_action(engine.Action.PlayCards, targets=np.array([value]))
    assert game.decision_point == 1
    assert game.apply_action(engine.Action.DiscardCards, targets=indices([0])) == 1


@pytest.mark.parametrize(
    "action, fields",
    [
        (engine.Action.SelectBlind, {}),
        (engine.Action.SelectBlind, {"source": 0, "targets": indices([0])}),
        (engine.Action.SelectBlind, {"source": 0, "order": indices([])}),
        (engine.Action.PlayCards, {}),
        (engine.Action.PlayCards, {"source": 0, "targets": indices([0])}),
        (engine.Action.ReorderHand, {}),
        (engine.Action.ReorderJokers, {"source": 0, "order": indices([])}),
        (engine.Action.SkipPack, {"targets": indices([])}),
        (engine.Action.RerollBoss, {"source": 0}),
    ],
)
def test_command_fields_are_required_and_exclusive(action, fields):
    game = engine.new(0)
    with pytest.raises(ValueError):
        game.apply_action(action, **fields)
    assert game.decision_point == 0


def test_card_and_permutation_targets_are_strict_arrays():
    game = playing_game()
    for value in [[0, 1], (0, 1), 0, "0"]:
        with pytest.raises(TypeError):
            game.apply_action(engine.Action.DiscardCards, targets=value)
        with pytest.raises(TypeError):
            game.apply_action(engine.Action.ReorderHand, order=value)
    assert game.apply_action(engine.Action.ReorderHand, order=indices(range(7, -1, -1))) == 1


@pytest.mark.parametrize("targets", [[], [0, 0], [8], [0, 1, 2, 3, 4, 5]])
def test_invalid_hand_selections_reach_rust_validation(targets):
    game = playing_game()
    with pytest.raises(ValueError):
        game.apply_action(engine.Action.PlayCards, targets=indices(targets))
    assert game.decision_point == 1
    assert game.apply_action(engine.Action.PlayCards, targets=indices([0])) == 1


def test_inventory_sales_work_at_initial_blind_selection():
    game = engine.new(0, deck=engine.Deck.Magic, stake=engine.Stake.White)
    assert game.apply_action(engine.Action.SellConsumable, source=0) == 0
    assert game.apply_action(engine.Action.SellConsumable, source=0) == 0
    with pytest.raises(ValueError):
        game.apply_action(engine.Action.SellConsumable, source=0)
    assert game.blind_index == 0


def test_unavailable_boss_reroll_and_pack_purchase_preserve_phase():
    game = engine.new(0)
    with pytest.raises(ValueError):
        game.apply_action(engine.Action.RerollBoss)
    assert game.decision_point == 0
    game = shop_game()
    with pytest.raises(ValueError):
        game.apply_action(engine.Action.BuyPack, source=1000)
    assert game.decision_point == 2
    assert game.apply_action(engine.Action.BuyPack, source=0) == 3


@pytest.mark.parametrize(
    "seed, scores, final_phase",
    [(0, [44, 76, 136, 472], 2), (1, [16, 112, 144, 252], 5)],
)
def test_blind_score_includes_final_winning_or_losing_hand(seed, scores, final_phase):
    game = engine.new(seed)
    assert type(game.blind_score) is int
    assert game.blind_score == 0
    assert game.apply_action(engine.Action.SelectBlind, source=0) == 1
    assert game.blind_score == 0
    for index, score in enumerate(scores):
        phase = game.apply_action(engine.Action.PlayCards, targets=indices(range(5)))
        assert phase == (final_phase if index == len(scores) - 1 else 1)
        assert type(game.blind_score) is int
        assert game.blind_score == score

    # Observations hide the blind on exit, but the final hand must still count.
    observation = game.observe()
    assert not observation["blind.visible"].any()
    assert not observation["blind.num"].any()
    assert game.next_step() == final_phase
    with pytest.raises(ValueError):
        game.apply_action(engine.Action.PlayCards, targets=indices([0]))
    assert game.blind_score == scores[-1]


def test_blind_score_survives_shop_pack_and_selection_until_next_blind_begins():
    game = shop_game()
    assert game.blind_score == 472
    assert game.apply_action(engine.Action.BuyPack, source=0) == 3
    assert game.blind_score == 472
    assert game.apply_action(engine.Action.SkipPack) == 2
    assert game.blind_score == 472
    assert game.apply_action(engine.Action.LeaveShop) == 0
    assert game.blind_score == 472
    assert game.next_step() == 0
    assert game.blind_score == 472
    assert game.apply_action(engine.Action.SelectBlind, source=1) == 1
    assert game.blind_score == 0
    assert engine.new(0).blind_score == 0


def test_blind_score_is_read_only_and_reads_do_not_change_state_or_rng():
    game = playing_game()
    control = playing_game()
    before = game.observe()
    for _ in range(3):
        assert game.blind_score == 0
    with pytest.raises(AttributeError):
        game.blind_score = 2**64 - 1
    for key, value in before.items():
        np.testing.assert_array_equal(game.observe()[key], value, err_msg=key)
    for _ in range(4):
        for current in (game, control):
            current.apply_action(engine.Action.PlayCards, targets=indices(range(5)))
        assert game.blind_score == control.blind_score
        actual, expected = game.observe(), control.observe()
        for key in actual:
            np.testing.assert_array_equal(actual[key], expected[key], err_msg=key)


@pytest.mark.parametrize(
    "action, fields, error",
    [
        (engine.Action.PlayCards, {}, ValueError),
        (engine.Action.PlayCards, {"targets": indices([0, 0])}, ValueError),
        (engine.Action.PlayCards, {"targets": indices([1000])}, ValueError),
        (engine.Action.PlayCards, {"targets": [0]}, TypeError),
        (engine.Action.SelectBlind, {"source": 0}, ValueError),
    ],
)
def test_rejected_commands_preserve_nonzero_blind_score(action, fields, error):
    game = playing_game()
    assert game.apply_action(engine.Action.PlayCards, targets=indices(range(5))) == 1
    assert game.blind_score == 44
    with pytest.raises(error):
        game.apply_action(action, **fields)
    assert game.blind_score == 44
    assert game.apply_action(engine.Action.DiscardCards, targets=indices([0])) == 1
    assert game.blind_score == 44


def test_old_action_keyword_arguments_are_removed():
    game = engine.new(0)
    with pytest.raises(TypeError):
        game.apply_action(engine.Action.SelectBlind, index=0)
    assert game.decision_point == 0
