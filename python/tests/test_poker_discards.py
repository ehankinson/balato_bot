import json
import random
from pathlib import Path

import pytest

from calculation.poker_discards import generate_discard_table
from core.enums import HandAction, PokerHand
from core.models import Card, Deck, GameState


FIXTURE_PATH = (
    Path(__file__).parent / "fixtures" / "poker_discard_rollouts.json"
)
# This fixture was recorded from _generate_discard_table_python_reference
# before the public generate_discard_table function was switched to Rust.
SEEDS = tuple(range(1000, 1008))
DISCARD_HANDS = tuple(
    poker_hand
    for poker_hand in PokerHand
    if poker_hand != PokerHand.HIGH_CARD
)


def _card_data(card: Card) -> list[int]:
    return [int(card.rank), int(card.suit), card.score]


def _discard_mask(hand: list[Card], discard: list[Card]) -> int:
    return sum(
        1 << index for index, card in enumerate(hand) if card in discard
    )


def _normalize_table(
    table: dict[PokerHand, tuple[int, float, list[Card]]],
    hand: list[Card],
) -> list[list[int | float]]:
    return [
        [
            table[poker_hand][0],
            table[poker_hand][1],
            _discard_mask(hand, table[poker_hand][2]),
        ]
        for poker_hand in DISCARD_HANDS
    ]


def _assert_table(
    actual: list[list[int | float]],
    expected: list[list[int | float]],
) -> None:
    assert len(actual) == len(expected) == len(DISCARD_HANDS)

    for poker_hand, actual_row, expected_row in zip(
        DISCARD_HANDS, actual, expected, strict=True
    ):
        actual_value, actual_probability, actual_discard_mask = actual_row
        expected_value, expected_probability, expected_discard_mask = (
            expected_row
        )

        assert actual_value == expected_value, poker_hand.name
        assert actual_probability == pytest.approx(
            expected_probability, abs=1e-12
        ), poker_hand.name
        assert actual_discard_mask == expected_discard_mask, poker_hand.name


@pytest.mark.parametrize("seed", SEEDS, ids=lambda seed: f"seed_{seed}")
def test_seeded_discard_rollout_matches_python_reference(seed: int) -> None:
    expected_rollouts = json.loads(FIXTURE_PATH.read_text())
    expected_steps = expected_rollouts[str(seed)]

    # Deck uses Python's global random module for shuffling. The separate RNG
    # makes the action/card-selection sequence deterministic and independent.
    random.seed(seed)
    action_rng = random.Random(seed)
    deck = Deck()
    game_state = GameState(score_to_beat=1_000_000)
    hand = deck.draw(game_state.hand_size)

    for step_index, expected in enumerate(expected_steps):
        assert [_card_data(card) for card in hand] == expected["hand"]
        assert game_state.hands == expected["hands"]
        assert game_state.discards == expected["discards"]

        table = generate_discard_table(deck, hand)
        _assert_table(_normalize_table(table, hand), expected["table"])

        valid_actions = [HandAction.PLAY_HAND]
        if game_state.discards > 0:
            valid_actions.append(HandAction.DISCARD)
        action = action_rng.choice(valid_actions)
        selected_indices = sorted(
            action_rng.sample(range(len(hand)), k=min(5, len(hand)))
        )

        assert int(action) == expected["action"], f"step {step_index}"
        assert selected_indices == expected["selected_indices"]

        selected_cards = [hand[index] for index in selected_indices]
        for index in reversed(selected_indices):
            hand.pop(index)

        game_state.execute_hand_action(
            action,
            selected_cards,
            hand,
            deck,
        )

    assert game_state.hands == 0
    assert len(hand) == game_state.hand_size
