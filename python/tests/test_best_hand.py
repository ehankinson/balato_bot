from collections import Counter

import pytest
from _test_util import build_card, build_jokers

from best_hand import get_best_scoring_hand
from config.poker_hands import HAND_STATS
from core.enums import Edition, Enhancement, JokersName, PokerHand, Rank, Seal, Suit
from core.hand_stats import HandStats
from core.models import (
    BestHand,
    FinalScoringResults,
    HandScoring,
    Joker,
    JokerPlan,
    JokerReq,
)


def build_joker(
    joker_name: JokersName,
    *,
    req_rank: Rank = Rank.NONE,
    req_suit: Suit = Suit.NONE,
) -> Joker:
    joker = Joker.build(joker_name)
    joker.req = JokerReq(rank=req_rank, suit=req_suit)
    return joker


def red_poly_glass_king(suit: Suit = Suit.HEARTS):
    return build_card(Rank.KING, suit, Enhancement.GLASS, Seal.RED, Edition.POLYCHROME)


def red_poly_steel_king(suit: Suit = Suit.HEARTS):
    return build_card(Rank.KING, suit, Enhancement.STEEL, Seal.RED, Edition.POLYCHROME)


def _check_value(value: str, best_score: int | float, expected: int | float):
    if best_score != expected:
        pytest.fail(
            f"The value {value}: {best_score} does not matche the expected: {expected}"
        )


def _check_list(value: str, best_score: list, expected: list):
    actual_counts = Counter(best_score)
    expected_counts = Counter(expected)

    missing = list((expected_counts - actual_counts).elements())
    not_touched = list((actual_counts - expected_counts).elements())
    if len(missing) > 0:
        pytest.fail(
            f"For the value {value}, these calculated items {not_touched} were not matched. "
            f"The expected was expecting: {missing}"
        )


def _assert_numbers(best_score: HandStats | BestHand, expected: HandStats | BestHand):
    for attribute in type(best_score).__dataclass_fields__.keys():
        _check_value(
            attribute,
            best_score.__getattribute__(attribute),
            expected.__getattribute__(attribute),
        )


def _assert_list(
    best_score: HandScoring | JokerPlan, expected: HandScoring | JokerPlan
):
    if isinstance(best_score, HandScoring) and isinstance(expected, HandScoring):
        _assert_numbers(best_score.hand_stats, expected.hand_stats)

    for attribute in type(best_score).__dataclass_fields__.keys():
        value = best_score.__getattribute__(attribute)
        if isinstance(value, list):
            _check_list(
                attribute,
                best_score.__getattribute__(attribute),
                expected.__getattribute__(attribute),
            )


def assert_final_scoring_results(
    best_score: FinalScoringResults, expected: FinalScoringResults
):
    _assert_numbers(best_score.best_hand, expected.best_hand)
    _assert_list(best_score.hand_scoring, expected.hand_scoring)
    _assert_list(best_score.joker_plan, expected.joker_plan)


def test_0001_high_card_scores_best_ace():
    cards = [build_card(Rank.ACE, Suit.HEARTS), build_card(Rank.KING, Suit.CLUBS)]
    jokers = []

    best_score = get_best_scoring_hand(cards, jokers)

    expected = FinalScoringResults(
        best_hand=BestHand(
            chips=16, worst_case_mult=1, avg_case_mult=1, best_case_mult=1
        ),
        hand_scoring=HandScoring(
            hand_stats=HAND_STATS[PokerHand.HIGH_CARD],
            scored_played=[build_card(Rank.ACE, Suit.HEARTS)],
            unscored_held=[build_card(Rank.KING, Suit.CLUBS)],
        ),
        joker_plan=JokerPlan(),
    )

    assert_final_scoring_results(best_score, expected)


def test_0002_pair_scores_best_pair():
    cards = [
        build_card(Rank.ACE, Suit.HEARTS),
        build_card(Rank.ACE, Suit.CLUBS),
        build_card(Rank.KING, Suit.SPADES),
    ]
    jokers = []

    best_score = get_best_scoring_hand(cards, jokers)

    expected = FinalScoringResults(
        best_hand=BestHand(
            chips=32, worst_case_mult=2, avg_case_mult=2, best_case_mult=2
        ),
        hand_scoring=HandScoring(
            hand_stats=HAND_STATS[PokerHand.PAIR],
            scored_played=[
                build_card(Rank.ACE, Suit.HEARTS),
                build_card(Rank.ACE, Suit.CLUBS),
            ],
            unscored_held=[build_card(Rank.KING, Suit.SPADES)],
        ),
        joker_plan=JokerPlan(),
    )

    assert_final_scoring_results(best_score, expected)


# def test_0003_two_pair_scores_best_two_pair():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS),
#         build_card(Rank.ACE, Suit.CLUBS),
#         build_card(Rank.KING, Suit.SPADES),
#         build_card(Rank.KING, Suit.DIAMONDS),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0004_three_kind_scores_best_three_kind():
#     cards = [
#         build_card(Rank.QUEEN, Suit.HEARTS),
#         build_card(Rank.QUEEN, Suit.CLUBS),
#         build_card(Rank.QUEEN, Suit.SPADES),
#         build_card(Rank.ACE, Suit.DIAMONDS),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0005_four_kind_scores_best_four_kind():
#     cards = [
#         build_card(Rank.KING, Suit.HEARTS),
#         build_card(Rank.KING, Suit.CLUBS),
#         build_card(Rank.KING, Suit.SPADES),
#         build_card(Rank.KING, Suit.DIAMONDS),
#         build_card(Rank.QUEEN, Suit.HEARTS),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0006_straight_scores_best_straight():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS),
#         build_card(Rank.KING, Suit.CLUBS),
#         build_card(Rank.QUEEN, Suit.SPADES),
#         build_card(Rank.JACK, Suit.DIAMONDS),
#         build_card(Rank.TEN, Suit.CLUBS),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0007_flush_scores_best_flush():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS),
#         build_card(Rank.KING, Suit.HEARTS),
#         build_card(Rank.QUEEN, Suit.HEARTS),
#         build_card(Rank.JACK, Suit.HEARTS),
#         build_card(Rank.NINE, Suit.HEARTS),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0008_full_house_scores_best_full_house():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS),
#         build_card(Rank.ACE, Suit.CLUBS),
#         build_card(Rank.ACE, Suit.SPADES),
#         build_card(Rank.KING, Suit.HEARTS),
#         build_card(Rank.KING, Suit.CLUBS),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0009_straight_flush_scores_best_straight_flush():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS),
#         build_card(Rank.KING, Suit.HEARTS),
#         build_card(Rank.QUEEN, Suit.HEARTS),
#         build_card(Rank.JACK, Suit.HEARTS),
#         build_card(Rank.TEN, Suit.HEARTS),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0010_five_kind_scores_best_five_kind():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS),
#         build_card(Rank.ACE, Suit.CLUBS),
#         build_card(Rank.ACE, Suit.SPADES),
#         build_card(Rank.ACE, Suit.DIAMONDS),
#         build_card(Rank.ACE, Suit.HEARTS),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0011_bonus_card_increases_chips():
#     cards = [build_card(Rank.ACE, Suit.HEARTS, Enhancement.BONUS)]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0012_foil_card_increases_chips():
#     cards = [build_card(Rank.ACE, Suit.HEARTS, edition=Edition.FOIL)]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0013_mult_card_increases_mult():
#     cards = [build_card(Rank.ACE, Suit.HEARTS, Enhancement.MULT)]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0014_holographic_card_increases_mult():
#     cards = [build_card(Rank.ACE, Suit.HEARTS, edition=Edition.HOLOGRAPHIC)]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0015_glass_card_doubles_mult():
#     cards = [build_card(Rank.ACE, Suit.HEARTS, Enhancement.GLASS)]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0016_polychrome_card_multiplies_mult():
#     cards = [build_card(Rank.ACE, Suit.HEARTS, edition=Edition.POLYCHROME)]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0017_red_seal_retriggers_played_card_chips():
#     cards = [build_card(Rank.ACE, Suit.HEARTS, seal=Seal.RED)]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0018_red_seal_glass_retriggers_played_x_mult():
#     cards = [build_card(Rank.ACE, Suit.HEARTS, Enhancement.GLASS, Seal.RED)]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0019_red_seal_polychrome_glass_retriggers_played_x_mult():
#     cards = [
#         build_card(
#             Rank.ACE, Suit.HEARTS, Enhancement.GLASS, Seal.RED, Edition.POLYCHROME
#         )
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0020_steel_card_scores_when_held():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS),
#         build_card(Rank.KING, Suit.CLUBS, Enhancement.STEEL),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0021_red_seal_steel_card_scores_when_held():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS),
#         build_card(Rank.KING, Suit.CLUBS, Enhancement.STEEL, Seal.RED),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0022_multiple_steel_cards_score_when_held():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS),
#         build_card(Rank.KING, Suit.CLUBS, Enhancement.STEEL),
#         build_card(Rank.QUEEN, Suit.SPADES, Enhancement.STEEL),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0023_stone_card_adds_to_played_chips():
#     cards = [
#         build_card(Rank.KING, Suit.CLUBS),
#         build_card(Rank.ACE, Suit.HEARTS, Enhancement.STONE),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0024_bonus_pair_scores_bonus_chips():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS, Enhancement.BONUS),
#         build_card(Rank.ACE, Suit.CLUBS),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0025_mult_pair_scores_added_mult():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS, Enhancement.MULT),
#         build_card(Rank.ACE, Suit.CLUBS),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0026_glass_pair_scores_x_mult():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS, Enhancement.GLASS),
#         build_card(Rank.ACE, Suit.CLUBS),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0027_glass_and_mult_pair_orders_add_before_x_mult():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS, Enhancement.GLASS),
#         build_card(Rank.ACE, Suit.CLUBS, Enhancement.MULT),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0028_lucky_card_tracks_best_avg_and_worst_mult_cases():
#     cards = [build_card(Rank.ACE, Suit.HEARTS, Enhancement.LUCKY)]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0029_best_scoring_type_currently_treats_lucky_card_as_neutral_mult():
#     cards = [
#         build_card(Rank.KING, Suit.CLUBS),
#         build_card(Rank.ACE, Suit.HEARTS, Enhancement.LUCKY),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0030_worst_scoring_type_chooses_plain_ace_over_lucky_king():
#     cards = [
#         build_card(Rank.KING, Suit.HEARTS, Enhancement.LUCKY),
#         build_card(Rank.ACE, Suit.CLUBS),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers, scoring_type="worst")

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0031_avg_scoring_type_currently_treats_lucky_card_as_neutral_mult():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS, Enhancement.LUCKY),
#         build_card(Rank.KING, Suit.CLUBS),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers, scoring_type="avg")

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0032_flush_with_bonus_cards_scores_best_bonus_flush():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS, Enhancement.BONUS),
#         build_card(Rank.KING, Suit.HEARTS),
#         build_card(Rank.QUEEN, Suit.HEARTS),
#         build_card(Rank.JACK, Suit.HEARTS),
#         build_card(Rank.NINE, Suit.HEARTS),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0033_flush_with_mult_card_scores_best_mult_flush():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS, Enhancement.MULT),
#         build_card(Rank.KING, Suit.HEARTS),
#         build_card(Rank.QUEEN, Suit.HEARTS),
#         build_card(Rank.JACK, Suit.HEARTS),
#         build_card(Rank.NINE, Suit.HEARTS),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0034_flush_with_glass_card_scores_best_glass_flush():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS, Enhancement.GLASS),
#         build_card(Rank.KING, Suit.HEARTS),
#         build_card(Rank.QUEEN, Suit.HEARTS),
#         build_card(Rank.JACK, Suit.HEARTS),
#         build_card(Rank.NINE, Suit.HEARTS),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0035_flush_with_polychrome_glass_card_scores_best_flush():
#     cards = [
#         build_card(
#             Rank.ACE, Suit.HEARTS, Enhancement.GLASS, edition=Edition.POLYCHROME
#         ),
#         build_card(Rank.KING, Suit.HEARTS),
#         build_card(Rank.QUEEN, Suit.HEARTS),
#         build_card(Rank.JACK, Suit.HEARTS),
#         build_card(Rank.NINE, Suit.HEARTS),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0036_full_house_with_steel_held_card_scores_steel_multiplier():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS),
#         build_card(Rank.ACE, Suit.CLUBS),
#         build_card(Rank.ACE, Suit.SPADES),
#         build_card(Rank.KING, Suit.HEARTS),
#         build_card(Rank.KING, Suit.CLUBS),
#         build_card(Rank.QUEEN, Suit.SPADES, Enhancement.STEEL),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0037_straight_flush_with_stone_extra_card_adds_stone():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS),
#         build_card(Rank.KING, Suit.HEARTS),
#         build_card(Rank.QUEEN, Suit.HEARTS),
#         build_card(Rank.JACK, Suit.HEARTS),
#         build_card(Rank.TEN, Suit.HEARTS),
#         build_card(Rank.THREE, Suit.CLUBS, Enhancement.STONE),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0038_high_card_returns_expected_final_scoring_results():
#     cards = [build_card(Rank.ACE, Suit.HEARTS)]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0039_empty_joker_list_uses_no_joker_path():
#     cards = [build_card(Rank.ACE, Suit.HEARTS)]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0040_high_card_can_choose_foil_over_plain_ace():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS),
#         build_card(Rank.KING, Suit.CLUBS, edition=Edition.FOIL),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0041_high_card_can_choose_holographic_king_over_plain_ace():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS),
#         build_card(Rank.KING, Suit.CLUBS, edition=Edition.HOLOGRAPHIC),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0042_high_card_can_choose_polychrome_ace_over_plain_king():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS, edition=Edition.POLYCHROME),
#         build_card(Rank.KING, Suit.CLUBS),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0043_pair_with_red_seal_card_retriggers_pair_card():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS, seal=Seal.RED),
#         build_card(Rank.ACE, Suit.CLUBS),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0044_pair_with_red_seal_mult_card_retriggers_add_mult():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS, Enhancement.MULT, Seal.RED),
#         build_card(Rank.ACE, Suit.CLUBS),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0045_pair_with_red_seal_glass_card_retriggers_x_mult():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS, Enhancement.GLASS, Seal.RED),
#         build_card(Rank.ACE, Suit.CLUBS),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0046_stone_card_does_not_create_high_card_by_itself():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS, Enhancement.STONE),
#         build_card(Rank.KING, Suit.CLUBS),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0047_multiple_stone_cards_can_fill_played_hand_slots():
#     cards = [
#         build_card(Rank.KING, Suit.CLUBS),
#         build_card(Rank.ACE, Suit.HEARTS, Enhancement.STONE),
#         build_card(Rank.QUEEN, Suit.SPADES, Enhancement.STONE),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0048_straight_ace_low_currently_raises_value_error():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS),
#         build_card(Rank.FIVE, Suit.CLUBS),
#         build_card(Rank.FOUR, Suit.SPADES),
#         build_card(Rank.THREE, Suit.DIAMONDS),
#         build_card(Rank.TWO, Suit.CLUBS),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0049_best_hand_with_more_than_five_cards_chooses_best_available_hand():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS),
#         build_card(Rank.ACE, Suit.CLUBS),
#         build_card(Rank.ACE, Suit.SPADES),
#         build_card(Rank.KING, Suit.DIAMONDS),
#         build_card(Rank.KING, Suit.HEARTS),
#         build_card(Rank.QUEEN, Suit.CLUBS),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0050_best_hand_with_many_cards_keeps_best_straight_flush_score():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS),
#         build_card(Rank.KING, Suit.HEARTS),
#         build_card(Rank.QUEEN, Suit.HEARTS),
#         build_card(Rank.JACK, Suit.HEARTS),
#         build_card(Rank.TEN, Suit.HEARTS),
#         build_card(Rank.ACE, Suit.CLUBS),
#         build_card(Rank.ACE, Suit.SPADES),
#         build_card(Rank.KING, Suit.CLUBS),
#     ]
#     jokers = []

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0051_joker_add_mult_public_best_hand_currently_raises_name_error():
#     cards = [build_card(Rank.ACE, Suit.HEARTS)]
#     jokers = build_jokers((JokersName.JOKER,))

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0052_jolly_joker_pair_public_best_hand_currently_raises_name_error():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS),
#         build_card(Rank.ACE, Suit.CLUBS),
#     ]
#     jokers = build_jokers((JokersName.JOLLY_JOKER,))

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0053_zany_joker_three_kind_public_best_hand_currently_raises_name_error():
#     cards = [
#         build_card(Rank.QUEEN, Suit.HEARTS),
#         build_card(Rank.QUEEN, Suit.CLUBS),
#         build_card(Rank.QUEEN, Suit.SPADES),
#     ]
#     jokers = build_jokers((JokersName.ZANY_JOKER,))

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0054_blackboard_baron_mime_public_best_hand_currently_raises_name_error():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS),
#         build_card(Rank.KING, Suit.DIAMONDS),
#         build_card(Rank.KING, Suit.CLUBS, Enhancement.STEEL, Seal.RED),
#         build_card(Rank.QUEEN, Suit.SPADES),
#         build_card(Rank.JACK, Suit.CLUBS),
#         build_card(Rank.NINE, Suit.SPADES),
#     ]
#     jokers = build_jokers((JokersName.BLACKBOARD, JokersName.BARON, JokersName.MIME))

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0055_raised_fist_blackboard_baron_mime_public_best_hand_currently_raises_name_error():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS),
#         build_card(Rank.KING, Suit.HEARTS),
#         build_card(Rank.KING, Suit.CLUBS, Enhancement.STEEL, Seal.RED),
#         build_card(Rank.QUEEN, Suit.SPADES),
#         build_card(Rank.JACK, Suit.CLUBS),
#         build_card(Rank.NINE, Suit.SPADES),
#     ]
#     jokers = build_jokers(
#         (
#             JokersName.RAISED_FIST,
#             JokersName.BLACKBOARD,
#             JokersName.BARON,
#             JokersName.MIME,
#         )
#     )

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0056_triboulet_photograph_hanging_chad_public_best_hand_currently_raises_name_error():
#     cards = [
#         build_card(
#             Rank.KING, Suit.HEARTS, Enhancement.GLASS, Seal.RED, Edition.POLYCHROME
#         ),
#         build_card(Rank.QUEEN, Suit.CLUBS, Enhancement.GLASS),
#         build_card(Rank.ACE, Suit.SPADES),
#     ]
#     jokers = build_jokers(
#         (
#             JokersName.TRIBOULET_BACKGROUND,
#             JokersName.PHOTOGRAPH,
#             JokersName.HANGING_CHAD,
#         )
#     )

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0057_two_blueprints_baron_mime_public_best_hand_complex_case():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS),
#         build_card(Rank.KING, Suit.CLUBS, Enhancement.STEEL, Seal.RED),
#         build_card(Rank.KING, Suit.SPADES, Enhancement.STEEL, Seal.RED),
#         build_card(Rank.QUEEN, Suit.DIAMONDS),
#     ]
#     jokers = build_jokers(
#         (JokersName.BLUEPRINT, JokersName.BLUEPRINT, JokersName.BARON, JokersName.MIME)
#     )

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0058_high_card_vs_flush_five_with_idol_baron_mime_triboulet():
#     cards = [
#         red_poly_glass_king(Suit.HEARTS),
#         red_poly_glass_king(Suit.HEARTS),
#         red_poly_glass_king(Suit.HEARTS),
#         red_poly_glass_king(Suit.HEARTS),
#         red_poly_glass_king(Suit.HEARTS),
#         red_poly_steel_king(Suit.HEARTS),
#         red_poly_steel_king(Suit.HEARTS),
#         red_poly_steel_king(Suit.HEARTS),
#         red_poly_steel_king(Suit.HEARTS),
#         red_poly_steel_king(Suit.HEARTS),
#         red_poly_steel_king(Suit.HEARTS),
#         red_poly_steel_king(Suit.HEARTS),
#         red_poly_steel_king(Suit.HEARTS),
#         red_poly_steel_king(Suit.HEARTS),
#         red_poly_steel_king(Suit.HEARTS),
#     ]
#     jokers = [
#         build_joker(JokersName.BLUEPRINT),
#         build_joker(JokersName.SOCK_AND_BUSKIN),
#         build_joker(JokersName.THE_IDOL, req_rank=Rank.KING, req_suit=Suit.HEARTS),
#         build_joker(JokersName.BARON),
#         build_joker(JokersName.MIME),
#         build_joker(JokersName.TRIBOULET_BACKGROUND),
#     ]

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0059_blackboard_can_make_playing_diamond_king_better_than_holding_baron():
#     cards = [
#         build_card(Rank.KING, Suit.DIAMONDS, Enhancement.STEEL, Seal.RED),
#         build_card(Rank.ACE, Suit.HEARTS),
#         build_card(Rank.QUEEN, Suit.CLUBS, Enhancement.STEEL, Seal.RED),
#         build_card(Rank.JACK, Suit.SPADES, Enhancement.STEEL, Seal.RED),
#         build_card(Rank.NINE, Suit.CLUBS, Enhancement.STEEL),
#         build_card(Rank.SEVEN, Suit.SPADES, Enhancement.STEEL),
#         build_card(Rank.FIVE, Suit.CLUBS),
#         build_card(Rank.THREE, Suit.SPADES),
#     ]
#     jokers = [
#         build_joker(JokersName.BLACKBOARD),
#         build_joker(JokersName.BARON),
#         build_joker(JokersName.MIME),
#     ]

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0060_blackboard_baron_mime_with_club_king_can_keep_blackboard_and_baron():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS),
#         build_card(Rank.KING, Suit.CLUBS, Enhancement.STEEL, Seal.RED),
#         build_card(Rank.KING, Suit.SPADES, Enhancement.STEEL, Seal.RED),
#         build_card(Rank.QUEEN, Suit.CLUBS, Enhancement.STEEL),
#         build_card(Rank.JACK, Suit.SPADES, Enhancement.STEEL),
#         build_card(Rank.NINE, Suit.CLUBS),
#         build_card(Rank.SEVEN, Suit.SPADES),
#     ]
#     jokers = [
#         build_joker(JokersName.BLACKBOARD),
#         build_joker(JokersName.BARON),
#         build_joker(JokersName.MIME),
#     ]

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0061_raised_fist_blackboard_baron_mime_large_held_hand():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS),
#         build_card(Rank.KING, Suit.HEARTS),
#         build_card(Rank.QUEEN, Suit.HEARTS),
#         build_card(Rank.JACK, Suit.HEARTS),
#         build_card(Rank.TEN, Suit.HEARTS),
#         build_card(Rank.KING, Suit.CLUBS, Enhancement.STEEL, Seal.RED),
#         build_card(Rank.KING, Suit.SPADES, Enhancement.STEEL, Seal.RED),
#         build_card(Rank.QUEEN, Suit.CLUBS),
#         build_card(Rank.JACK, Suit.SPADES),
#         build_card(Rank.NINE, Suit.CLUBS),
#         build_card(Rank.SEVEN, Suit.SPADES),
#         build_card(Rank.FIVE, Suit.CLUBS),
#     ]
#     jokers = [
#         build_joker(JokersName.RAISED_FIST),
#         build_joker(JokersName.BLACKBOARD),
#         build_joker(JokersName.BARON),
#         build_joker(JokersName.MIME),
#     ]

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0062_photograph_hanging_chad_triboulet_red_seal_poly_glass_faces():
#     cards = [
#         build_card(
#             Rank.KING, Suit.HEARTS, Enhancement.GLASS, Seal.RED, Edition.POLYCHROME
#         ),
#         build_card(
#             Rank.QUEEN, Suit.HEARTS, Enhancement.GLASS, Seal.RED, Edition.POLYCHROME
#         ),
#         build_card(Rank.KING, Suit.CLUBS, Enhancement.GLASS, Seal.RED),
#         build_card(Rank.QUEEN, Suit.SPADES, Enhancement.GLASS),
#         build_card(Rank.ACE, Suit.DIAMONDS),
#         build_card(Rank.TEN, Suit.CLUBS),
#     ]
#     jokers = [
#         build_joker(JokersName.PHOTOGRAPH),
#         build_joker(JokersName.HANGING_CHAD),
#         build_joker(JokersName.TRIBOULET_BACKGROUND),
#         build_joker(JokersName.SOCK_AND_BUSKIN),
#     ]

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0063_blueprint_baron_mime_multiple_red_seal_steel_kings():
#     cards = [
#         build_card(Rank.ACE, Suit.SPADES),
#         red_poly_steel_king(Suit.HEARTS),
#         red_poly_steel_king(Suit.DIAMONDS),
#         red_poly_steel_king(Suit.CLUBS),
#         red_poly_steel_king(Suit.SPADES),
#         red_poly_steel_king(Suit.HEARTS),
#         red_poly_steel_king(Suit.DIAMONDS),
#         red_poly_steel_king(Suit.CLUBS),
#         red_poly_steel_king(Suit.SPADES),
#     ]
#     jokers = [
#         build_joker(JokersName.BLUEPRINT),
#         build_joker(JokersName.BARON),
#         build_joker(JokersName.MIME),
#     ]

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0064_blueprint_brainstorm_baron_mime_triboulet_photograph_sock_example():
#     cards = [
#         build_card(Rank.ACE, Suit.SPADES),
#         red_poly_steel_king(Suit.HEARTS),
#         red_poly_steel_king(Suit.DIAMONDS),
#         red_poly_steel_king(Suit.CLUBS),
#         red_poly_steel_king(Suit.SPADES),
#         red_poly_steel_king(Suit.HEARTS),
#         red_poly_steel_king(Suit.DIAMONDS),
#         red_poly_steel_king(Suit.CLUBS),
#         red_poly_steel_king(Suit.HEARTS),
#         red_poly_steel_king(Suit.DIAMONDS),
#         red_poly_steel_king(Suit.CLUBS),
#         red_poly_steel_king(Suit.SPADES),
#         red_poly_steel_king(Suit.HEARTS),
#         red_poly_steel_king(Suit.DIAMONDS),
#         red_poly_steel_king(Suit.CLUBS),
#     ]
#     jokers = [
#         build_joker(JokersName.BLUEPRINT),
#         build_joker(JokersName.BARON),
#         build_joker(JokersName.BRAINSTORM),
#         build_joker(JokersName.MIME),
#         build_joker(JokersName.TRIBOULET_BACKGROUND),
#         build_joker(JokersName.PHOTOGRAPH),
#         build_joker(JokersName.SOCK_AND_BUSKIN),
#     ]

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0065_flush_five_vs_held_steel_kings_with_idol_and_sock():
#     cards = [
#         red_poly_glass_king(Suit.HEARTS),
#         red_poly_glass_king(Suit.HEARTS),
#         red_poly_glass_king(Suit.HEARTS),
#         red_poly_glass_king(Suit.HEARTS),
#         red_poly_glass_king(Suit.HEARTS),
#         red_poly_steel_king(Suit.SPADES),
#         red_poly_steel_king(Suit.CLUBS),
#         red_poly_steel_king(Suit.DIAMONDS),
#         red_poly_steel_king(Suit.HEARTS),
#     ]
#     jokers = [
#         build_joker(JokersName.THE_IDOL, req_rank=Rank.KING, req_suit=Suit.HEARTS),
#         build_joker(JokersName.SOCK_AND_BUSKIN),
#         build_joker(JokersName.BARON),
#         build_joker(JokersName.MIME),
#         build_joker(JokersName.TRIBOULET_BACKGROUND),
#     ]

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0066_hack_low_red_seal_glass_cards_vs_high_face_synergy():
#     cards = [
#         build_card(
#             Rank.FIVE, Suit.HEARTS, Enhancement.GLASS, Seal.RED, Edition.POLYCHROME
#         ),
#         build_card(Rank.FIVE, Suit.CLUBS, Enhancement.GLASS, Seal.RED),
#         build_card(Rank.FOUR, Suit.SPADES, Enhancement.GLASS, Seal.RED),
#         build_card(Rank.KING, Suit.HEARTS, Enhancement.GLASS, Seal.RED),
#         build_card(Rank.QUEEN, Suit.HEARTS, Enhancement.GLASS, Seal.RED),
#         build_card(Rank.ACE, Suit.DIAMONDS),
#     ]
#     jokers = [
#         build_joker(JokersName.HACK),
#         build_joker(JokersName.PHOTOGRAPH),
#         build_joker(JokersName.HANGING_CHAD),
#         build_joker(JokersName.SOCK_AND_BUSKIN),
#     ]

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0067_blackboard_vs_full_house_face_synergy():
#     cards = [
#         build_card(Rank.KING, Suit.HEARTS, Enhancement.GLASS, Seal.RED),
#         build_card(Rank.KING, Suit.DIAMONDS, Enhancement.GLASS, Seal.RED),
#         build_card(Rank.KING, Suit.CLUBS, Enhancement.STEEL, Seal.RED),
#         build_card(Rank.QUEEN, Suit.HEARTS, Enhancement.GLASS, Seal.RED),
#         build_card(Rank.QUEEN, Suit.SPADES, Enhancement.STEEL, Seal.RED),
#         build_card(Rank.JACK, Suit.CLUBS, Enhancement.STEEL),
#         build_card(Rank.NINE, Suit.SPADES, Enhancement.STEEL),
#     ]
#     jokers = [
#         build_joker(JokersName.BLACKBOARD),
#         build_joker(JokersName.TRIBOULET_BACKGROUND),
#         build_joker(JokersName.SOCK_AND_BUSKIN),
#         build_joker(JokersName.BARON),
#         build_joker(JokersName.MIME),
#     ]

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)
