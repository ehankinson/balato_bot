import gc

import numpy as np
import pytest
import torch

import balatro_engine as engine

TABLES = {
    "jokers": (3, 1, 5),
    "joker_values": (3, 1, 0),
    "consumables": (2, 1, 1),
    "last_consumable": (2, 1, 1),
    "deck_catalogue": (5, 3, 0),
    "poker_hands": (1, 2, 0),
    "vouchers": (1, 0, 0),
    "tags": (2, 1, 0),
    "blind_offers": (3, 2, 2),
    "hand": (5, 3, 3),
    "shop_items": (10, 5, 5),
    "shop_vouchers": (1, 1, 0),
    "shop_packs": (2, 1, 0),
    "pack_options": (10, 5, 5),
}
VECTORS = {
    "schema_version": (np.int32, 1),
    "sections": (np.bool_, 5),
    "run.cat": (np.int32, 5),
    "run.counts": (np.int64, 14),
    "run.num": (np.float32, 3),
    "run.extra": (np.int64, 3),
    "run.used_planets": (np.bool_, 12),
    "run.flags": (np.bool_, 3),
    "run.visible": (np.bool_, 28),
    "blind.cat": (np.int32, 4),
    "blind.counts": (np.int64, 5),
    "blind.num": (np.float32, 2),
    "blind.flags": (np.bool_, 3),
    "blind.visible": (np.bool_, 14),
    "blind.played_types": (np.bool_, 12),
    "shop.num": (np.float32, 1),
    "shop.visible": (np.bool_, 1),
    "pack.cat": (np.int32, 3),
    "pack.counts": (np.int64, 1),
    "pack.visible": (np.bool_, 4),
    "action_kinds": (np.bool_, 18),
}


def indices(values=()):
    return np.array(values, dtype=np.int32)


def playing_game(seed=0, deck=engine.Deck.Red):
    game = engine.new(seed, deck=deck)
    game.apply_action(engine.Action.SelectBlind, source=0)
    return game


def finished_blind(seed=0, deck=engine.Deck.Red):
    game = playing_game(seed, deck=deck)
    for _ in range(4):
        phase = game.apply_action(engine.Action.PlayCards, targets=indices(range(5)))
        if phase != 1:
            break
    return game


def assert_equal(left, right):
    assert left.keys() == right.keys()
    for key in left:
        np.testing.assert_array_equal(left[key], right[key], err_msg=key)


def assert_schema(observation):
    expected = {}
    for name, (cat, num, flags) in TABLES.items():
        rows = len(observation[f"{name}.present"])
        expected.update({
            f"{name}.cat": (np.int32, (rows, cat)),
            f"{name}.num": (np.float32, (rows, num)),
            f"{name}.flags": (np.bool_, (rows, flags)),
            f"{name}.present": (np.bool_, (rows,)),
            f"{name}.visible": (np.bool_, (rows, cat + num + flags)),
        })
    expected.update({key: (dtype, (length,)) for key, (dtype, length) in VECTORS.items()})
    expected["deck_multiplicity"] = (np.int64, (len(observation["deck_catalogue.present"]),))
    expected["poker_hand_counts"] = (np.int64, (12, 2))
    assert observation.keys() == expected.keys()
    for key, (dtype, shape) in expected.items():
        array = observation[key]
        assert isinstance(array, np.ndarray), key
        assert array.dtype == dtype, key
        assert array.shape == shape, key
        assert array.flags.c_contiguous and array.flags.aligned, key
        assert np.isfinite(array).all(), key
    assert observation["schema_version"].tolist() == [1]


@pytest.mark.parametrize("phase", [0, 1, 2, 3, 5])
def test_complete_schema_at_every_supported_decision_boundary(phase):
    if phase == 0:
        game = engine.new(0, deck=engine.Deck.Magic)
    elif phase == 1:
        game = playing_game()
    elif phase == 5:
        game = finished_blind(1)
    else:
        game = finished_blind()
        if phase == 3:
            game.apply_action(engine.Action.BuyPack, source=0)
    assert game.decision_point == phase
    observation = game.observe()
    assert_schema(observation)
    assert observation["run.cat"][0] == phase
    assert observation["run.flags"][0] == (phase == 5)
    assert observation["sections"][:4].tolist() == [phase == n for n in range(4)]
    assert len(observation["hand.present"]) == (8 if phase == 1 else 0)
    assert len(observation["shop_items.present"]) == (2 if phase == 2 else 0)
    if phase != 0:
        assert not observation["blind_offers.present"].any()
    if phase != 1:
        assert not observation["blind.visible"].any()
        assert not observation["blind.num"].any()
    if phase != 2:
        assert not observation["shop.visible"].any()
        assert not observation["shop.num"].any()
    if phase != 3:
        assert not observation["pack.visible"].any()
        assert not len(observation["pack_options.present"])


# Fixed seeded public trajectories, with no private state construction.
@pytest.mark.parametrize("seed,slot,kind,has_hand", [
    (0, 0, 2, False),   # Celestial
    (0, 1, 0, False),   # Standard
    (4, 0, 3, False),   # Buffoon
    (79, 1, 1, True),   # Arcana
    (270, 0, 4, True),  # Spectral
])
def test_all_pack_kinds_have_separate_offers_and_clear_temporary_sections(seed, slot, kind, has_hand):
    game = finished_blind(seed)
    assert game.apply_action(engine.Action.BuyPack, source=slot) == 3
    observation = game.observe()
    assert_schema(observation)
    assert observation["pack.cat"][0] == kind
    assert observation["sections"][4] == has_hand
    assert bool(len(observation["hand.present"])) == has_hand
    assert len(observation["pack_options.present"]) > 0
    assert game.next_step() == 3
    assert_equal(observation, game.observe())
    assert game.apply_action(engine.Action.SkipPack) == 2
    assert_schema(game.observe())
    assert not len(game.observe()["hand.present"])
    assert not len(game.observe()["pack_options.present"])


def test_observations_survive_steps_new_observations_and_game_destruction():
    game = playing_game()
    old = game.observe()
    expected = {key: array.copy() for key, array in old.items()}
    game.apply_action(engine.Action.ReorderHand, order=indices(range(7, -1, -1)))
    reordered = game.observe()
    np.testing.assert_array_equal(reordered["hand.cat"], old["hand.cat"][::-1])
    game.apply_action(engine.Action.DiscardCards, targets=indices([0]))
    game.observe()
    del game
    gc.collect()
    assert_equal(old, expected)


def test_python_writes_and_tensor_views_cannot_mutate_game_or_other_observations():
    game = playing_game()
    original = game.observe()
    retained = game.observe()
    for key, array in original.items():
        if array.size:
            assert not np.shares_memory(array, retained[key]), key
        tensor = torch.from_numpy(array)
        assert tensor.shape == array.shape
        if array.size:
            assert tensor.data_ptr() == array.ctypes.data, key
        np.testing.assert_array_equal(tensor.numpy(), array)
        tensor.zero_()
    assert_equal(retained, game.observe())
    # The tensor retains its storage after the observation and game go away.
    tensor = torch.from_numpy(retained["hand.num"])
    expected = tensor.clone()
    del retained, original, game
    gc.collect()
    torch.testing.assert_close(tensor, expected)


def test_repeated_reads_and_constraint_queries_do_not_change_seeded_trajectory():
    left, right = playing_game(), playing_game()
    for _ in range(3):
        for _ in range(3):
            assert_equal(left.observe(), right.observe())
            left.action_constraints(engine.Action.PlayCards)
            left.selection_constraints(engine.Action.PlayCards, indices([0]))
        left.apply_action(engine.Action.DiscardCards, targets=indices([0, 2]))
        right.apply_action(engine.Action.DiscardCards, targets=indices([0, 2]))
    assert_equal(left.observe(), right.observe())


def test_constraints_have_typed_owned_arrays_and_refine_selection_prefixes():
    game = playing_game()
    action = engine.Action.PlayCards
    constraints = game.action_constraints(action)
    assert set(constraints) == {
        "source_mask", "source_known", "target_collection", "target_mask", "required_targets",
        "count_mask", "ordered", "permutation_collection", "can_submit",
    }
    for key, array in constraints.items():
        assert array.dtype == (np.int32 if key in {"target_collection", "permutation_collection"} else np.bool_)
        assert array.ndim == 1 and array.flags.c_contiguous
    assert constraints["target_collection"].tolist() == [1]
    assert constraints["source_mask"].shape == (0,)
    assert constraints["count_mask"].tolist() == [False] + [True] * 5 + [False] * 3
    assert not constraints["can_submit"][0]
    partial = game.selection_constraints(action, indices([0, 2]))
    assert partial["can_submit"][0]
    assert not partial["target_mask"][[0, 2]].any()
    assert not partial["count_mask"][:2].any()
    final = game.selection_constraints(action, indices(range(5)))
    assert final["can_submit"][0] and not final["target_mask"].any()
    constraints["target_mask"][:] = False
    assert game.action_constraints(action)["target_mask"].all()
    assert game.apply_action(action, targets=indices([0, 2])) == 1
    assert not partial["target_mask"][[0, 2]].any()


def test_permutation_constraints_require_every_old_slot_once():
    game = playing_game()
    action = engine.Action.ReorderHand
    first = game.action_constraints(action)
    assert first["permutation_collection"].tolist() == [1]
    assert first["ordered"].tolist() == [True]
    assert np.flatnonzero(first["count_mask"]).tolist() == [8]
    prefix = indices([2, 0])
    partial = game.selection_constraints(action, prefix)
    assert not partial["can_submit"][0]
    assert not partial["target_mask"][[0, 2]].any()
    order = indices([2, 0, 1, 3, 4, 5, 6, 7])
    assert game.selection_constraints(action, order)["can_submit"][0]
    game.apply_action(action, order=order)
    game.apply_action(engine.Action.ReorderJokers, order=indices())


@pytest.mark.parametrize("prefix", [[0, 0], [8], list(range(6))])
def test_invalid_conditional_prefixes_fail_without_mutation(prefix):
    game = playing_game()
    before = game.observe()
    with pytest.raises(ValueError):
        game.selection_constraints(engine.Action.PlayCards, indices(prefix))
    assert_equal(before, game.observe())


def test_source_queries_and_empty_unavailable_queries():
    game = engine.new(0)
    sources = game.action_constraints(engine.Action.SelectBlind)
    assert sources["source_mask"].tolist() == [True, False, False]
    assert sources["source_known"].all()
    assert not sources["can_submit"][0]
    assert game.action_constraints(engine.Action.SelectBlind, source=0)["can_submit"][0]
    unavailable = game.selection_constraints(engine.Action.PlayCards, indices())
    assert not unavailable["count_mask"].any()
    for source in [True, 0.0, -1, 2**40, 3]:
        with pytest.raises((TypeError, ValueError, OverflowError)):
            game.action_constraints(engine.Action.SelectBlind, source=source)
    with pytest.raises(ValueError):
        game.action_constraints(engine.Action.ReorderHand, source=0)
    assert game.decision_point == 0


def test_pack_consumable_sources_and_targets_use_separate_collections():
    game = finished_blind(79)
    game.apply_action(engine.Action.BuyPack, source=1)
    action = engine.Action.UsePackConsumable
    before = game.observe()
    sources = game.action_constraints(action)["source_mask"]
    source = next(int(i) for i in np.flatnonzero(sources)
                  if game.action_constraints(action, source=int(i))["target_mask"].size)
    prefix = []
    while True:
        constraints = game.selection_constraints(action, indices(prefix), source=source)
        if constraints["can_submit"][0]:
            break
        prefix.append(int(np.flatnonzero(constraints["target_mask"])[0]))
    game.apply_action(action, source=source, targets=indices(prefix))
    after = game.observe()
    assert not np.array_equal(before["hand.cat"], after["hand.cat"])
    assert after["pack.counts"][0] < before["pack.counts"][0]


def test_owned_consumable_source_masks_and_no_target_commands():
    game = finished_blind(deck=engine.Deck.Magic)
    assert len(game.observe()["consumables.present"]) == 2
    assert not game.action_constraints(engine.Action.UseConsumable)["source_mask"].any()
    game.apply_action(engine.Action.BuyPack, source=0)
    sources = game.action_constraints(engine.Action.UsePackConsumable)["source_mask"]
    game.apply_action(engine.Action.UsePackConsumable, source=int(np.flatnonzero(sources)[0]))
    constraints = game.action_constraints(engine.Action.UseConsumable, source=0)
    assert constraints["source_mask"].tolist() == [True, True]
    assert constraints["can_submit"][0]
    assert not len(constraints["target_mask"])
    before = game.observe()
    with pytest.raises(ValueError):
        game.apply_action(engine.Action.UseConsumable, source=0, targets=indices())
    assert_equal(before, game.observe())
    game.apply_action(engine.Action.UseConsumable, source=0)
    # The Fool consumes its slot and creates a copy of the last used Planet.
    np.testing.assert_array_equal(
        game.observe()["consumables.cat"],
        np.vstack([before["consumables.cat"][1:], before["last_consumable.cat"]]),
    )


def test_skip_tag_pack_automatic_work_finishes_before_returned_phase():
    game = engine.new(3)
    assert game.apply_action(engine.Action.SkipBlind, source=0) == 3
    opening = game.observe()
    assert opening["sections"][3]
    assert opening["pack.cat"][2] == 0  # saved return boundary is blind selection
    assert game.next_step() == 3
    assert_equal(opening, game.observe())
    assert game.apply_action(engine.Action.SkipPack) == 0
    assert game.observe()["run.cat"][3] == 1


@pytest.mark.parametrize("order", [[0], [0, 0, 2, 3, 4, 5, 6, 7], list(range(9))])
def test_invalid_permutations_reach_rules_and_preserve_observation(order):
    game = playing_game()
    before = game.observe()
    with pytest.raises(ValueError):
        game.apply_action(engine.Action.ReorderHand, order=indices(order))
    assert_equal(before, game.observe())


@pytest.mark.parametrize("action", [True, 0, 0.0, "SelectBlind", np.int32(0)])
def test_action_kind_requires_semantic_enum_without_coercion(action):
    game = engine.new(0)
    before = game.observe()
    with pytest.raises(TypeError):
        game.apply_action(action, source=0)
    with pytest.raises(TypeError):
        game.action_constraints(action)
    with pytest.raises(TypeError):
        game.selection_constraints(action, indices())
    assert_equal(before, game.observe())


def bad_payloads():
    return [
        [0], (0,), True, 0, "0", np.int32(0),
        np.array([0], dtype=np.int64), np.array([0], dtype=np.uint32),
        np.array([0.0], dtype=np.float32), np.array([False]),
        np.array([0], dtype=object), np.array([0], dtype=">i4"),
        np.array(0, dtype=np.int32), np.array([[0]], dtype=np.int32),
        np.arange(8, dtype=np.int32)[::2], np.arange(4, dtype=np.int32)[::-1],
        np.ndarray((1,), dtype=np.int32, buffer=bytearray(5), offset=1),
        indices([-1]),
    ]


@pytest.mark.parametrize("payload", bad_payloads())
@pytest.mark.parametrize("route", ["targets", "order", "prefix"])
def test_array_inputs_reject_wrong_types_layout_and_negative_indices_atomically(payload, route):
    game = playing_game()
    before = game.observe()
    with pytest.raises((TypeError, ValueError)):
        if route == "targets":
            game.apply_action(engine.Action.PlayCards, targets=payload)
        elif route == "order":
            game.apply_action(engine.Action.ReorderHand, order=payload)
        else:
            game.selection_constraints(engine.Action.PlayCards, payload)
    assert_equal(before, game.observe())


def test_readonly_input_arrays_are_accepted_without_modification():
    game = playing_game()
    targets = indices([0])
    targets.setflags(write=False)
    game.selection_constraints(engine.Action.DiscardCards, targets)
    assert game.apply_action(engine.Action.DiscardCards, targets=targets) == 1
    np.testing.assert_array_equal(targets, indices([0]))


def test_terminal_observations_are_stable_and_have_no_available_actions():
    game = finished_blind(1)
    final = game.observe()
    assert game.next_step() == 5
    assert final["run.flags"][0]
    assert final["run.cat"][4] == 1
    assert not final["action_kinds"].any()
    with pytest.raises(ValueError):
        game.apply_action(engine.Action.PlayCards, targets=indices([0]))
    assert_equal(final, game.observe())
