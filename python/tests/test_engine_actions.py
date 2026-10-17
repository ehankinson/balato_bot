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


def test_old_action_keyword_arguments_are_removed():
    game = engine.new(0)
    with pytest.raises(TypeError):
        game.apply_action(engine.Action.SelectBlind, index=0)
    assert game.decision_point == 0
