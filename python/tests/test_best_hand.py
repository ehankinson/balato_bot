import os
import platform
import time
from collections import Counter

import pytest
from _test_util import build_card

from best_hand import build_joker_plan, get_best_scoring_hand
from config.poker_hands import HAND_STATS
from core.enums import Edition, Enhancement, JokersName, PokerHand, Rank, Seal, Suit
from core.hand_stats import HandStats
from core.models import (
    BestHand,
    Card,
    FinalScoringResults,
    GameState,
    HandScoring,
    Joker,
    JokerGameModifier,
    JokerPlan,
    JokerReq,
)
from game.game_state import build_game_state
from utils.files import load_json, write_json

CWD = os.getcwd()
VERSION = 0.5
PERFORMANCE_FILE = os.path.join(
    CWD, "tests", "best_hand_performance", f"results_{VERSION}_{platform.system()}.json"
)


def build_test_game_state(jokers: list[Joker], cards: list[Card]) -> GameState:
    game_state = GameState()
    build_game_state(
        game_state,
        [joker for joker in jokers if isinstance(joker, JokerGameModifier)],
        cards,
    )
    return game_state


def build_joker(
    joker_name: JokersName,
    *,
    req_rank: Rank = Rank.NONE,
    req_suit: Suit = Suit.NONE,
) -> Joker:
    joker = Joker.build(joker_name)
    joker.req = JokerReq(rank=req_rank, suit=req_suit)
    return joker


def _check_value(value: str, best_score: int | float, expected: int | float):
    if best_score != pytest.approx(expected):
        pytest.fail(
            f"The value {value}: {best_score} does not matche the expected: {expected}"
        )


def _check_list(value: str, best_score: list, expected: list):
    actual_counts = Counter(best_score)
    expected_counts = Counter(expected)

    missing_values = list((expected_counts - actual_counts).elements())
    incorrect_values = list((actual_counts - expected_counts).elements())
    if len(incorrect_values) > 0:
        pytest.fail(
            f"For the value {value}, these calculated items {incorrect_values} were not matched. "
            f"The expected was expecting: {missing_values}"
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
    _assert_list(best_score.hand_scoring, expected.hand_scoring)
    _assert_numbers(best_score.best_hand, expected.best_hand)
    _assert_list(best_score.joker_plan, expected.joker_plan)


def run_assert(
    test_number: int,
    cards: list[Card],
    jokers: list[Joker],
    game_state: GameState,
    expected: FinalScoringResults,
):
    start_time = time.perf_counter()
    best_score, iterations = get_best_scoring_hand(cards, jokers, game_state, test=True)
    end_time = time.perf_counter()
    time_spent = end_time - start_time

    assert_final_scoring_results(best_score, expected)
    data = load_json(PERFORMANCE_FILE)

    if "total" not in data or test_number == 1:
        data["total"] = {
            "test_count": 0,
            "total_time_spend": 0.0,
            "total_combinations": 0,
        }

    data["total"]["test_count"] += 1
    data["total"]["total_time_spend"] += time_spent
    data["total"]["total_combinations"] += iterations

    data[f"test_{test_number}"] = {"time_spend": time_spent, "combinations": iterations}

    write_json(PERFORMANCE_FILE, data)


def test_0001_high_card_scores_best_ace():
    cards = [
        build_card(
            Rank.ACE, Suit.HEARTS, Enhancement.GLASS, Seal.RED, Edition.POLYCHROME
        ),
        build_card(Rank.KING, Suit.CLUBS, Enhancement.MULT),
        build_card(Rank.QUEEN, Suit.DIAMONDS, Enhancement.GLASS),
        build_card(Rank.NINE, Suit.SPADES, Enhancement.LUCKY),
        build_card(Rank.SEVEN, Suit.HEARTS, Enhancement.BONUS),
        build_card(Rank.FIVE, Suit.CLUBS, Enhancement.GLASS, Seal.RED),
        build_card(Rank.THREE, Suit.DIAMONDS, Enhancement.STONE),
        build_card(Rank.TWO, Suit.SPADES, edition=Edition.HOLOGRAPHIC),
    ]
    jokers = []

    expected = FinalScoringResults(
        best_hand=BestHand(
            chips=77, worst_case_mult=9, avg_case_mult=9, best_case_mult=9
        ),
        hand_scoring=HandScoring(
            hand_stats=HAND_STATS[PokerHand.HIGH_CARD],
            scored_played=[
                build_card(
                    Rank.ACE,
                    Suit.HEARTS,
                    Enhancement.GLASS,
                    Seal.RED,
                    Edition.POLYCHROME,
                ),
                build_card(Rank.THREE, Suit.DIAMONDS, Enhancement.STONE),
            ],
            unscored_held=[
                build_card(Rank.KING, Suit.CLUBS, Enhancement.MULT),
                build_card(Rank.QUEEN, Suit.DIAMONDS, Enhancement.GLASS),
                build_card(Rank.NINE, Suit.SPADES, Enhancement.LUCKY),
                build_card(Rank.SEVEN, Suit.HEARTS, Enhancement.BONUS),
                build_card(Rank.FIVE, Suit.CLUBS, Enhancement.GLASS, Seal.RED),
                build_card(Rank.TWO, Suit.SPADES, edition=Edition.HOLOGRAPHIC),
            ],
        ),
        joker_plan=JokerPlan(),
    )

    run_assert(1, cards, jokers, GameState(), expected)


def test_0002_pair_scores_best_pair():
    cards = [
        build_card(Rank.ACE, Suit.HEARTS, edition=Edition.HOLOGRAPHIC),
        build_card(
            Rank.ACE, Suit.CLUBS, Enhancement.GLASS, Seal.RED, Edition.POLYCHROME
        ),
        build_card(Rank.KING, Suit.HEARTS),
        build_card(Rank.KING, Suit.HEARTS, Enhancement.STEEL, Seal.RED),
        build_card(Rank.FIVE, Suit.DIAMONDS, Enhancement.GLASS, Seal.RED),
        build_card(Rank.THREE, Suit.SPADES, Enhancement.STONE),
        build_card(Rank.TWO, Suit.HEARTS),
        build_card(Rank.NINE, Suit.HEARTS),
    ]
    jokers = []

    expected = FinalScoringResults(
        best_hand=BestHand(
            chips=93, worst_case_mult=243, avg_case_mult=243, best_case_mult=243
        ),
        hand_scoring=HandScoring(
            hand_stats=HAND_STATS[PokerHand.PAIR],
            scored_played=[
                build_card(Rank.ACE, Suit.HEARTS, edition=Edition.HOLOGRAPHIC),
                build_card(
                    Rank.ACE,
                    Suit.CLUBS,
                    Enhancement.GLASS,
                    Seal.RED,
                    Edition.POLYCHROME,
                ),
                build_card(Rank.THREE, Suit.SPADES, Enhancement.STONE),
            ],
            scored_held=[
                build_card(Rank.KING, Suit.HEARTS, Enhancement.STEEL, Seal.RED),
            ],
            unscored_held=[
                build_card(Rank.KING, Suit.HEARTS),
                build_card(Rank.FIVE, Suit.DIAMONDS, Enhancement.GLASS, Seal.RED),
                build_card(Rank.THREE, Suit.SPADES, Enhancement.STONE),
                build_card(Rank.TWO, Suit.HEARTS),
                build_card(Rank.NINE, Suit.HEARTS),
            ],
        ),
        joker_plan=JokerPlan(),
    )

    run_assert(2, cards, jokers, GameState(), expected)


def test_0003_two_pair_scores_best_two_pair():
    cards = [
        build_card(Rank.ACE, Suit.HEARTS, Enhancement.GLASS, Seal.RED),
        build_card(Rank.ACE, Suit.CLUBS, edition=Edition.POLYCHROME),
        build_card(Rank.KING, Suit.SPADES, Enhancement.MULT),
        build_card(Rank.KING, Suit.DIAMONDS, Enhancement.MULT),
        build_card(Rank.QUEEN, Suit.HEARTS, Enhancement.GLASS),
        build_card(Rank.QUEEN, Suit.CLUBS, Enhancement.BONUS),
        build_card(Rank.FIVE, Suit.DIAMONDS, Enhancement.GLASS, Seal.RED),
        build_card(Rank.THREE, Suit.SPADES, Enhancement.STONE),
    ]
    jokers = []

    expected = FinalScoringResults(
        best_hand=BestHand(
            chips=123, worst_case_mult=60, avg_case_mult=60, best_case_mult=60
        ),
        hand_scoring=HandScoring(
            hand_stats=HAND_STATS[PokerHand.TWO_PAIR],
            scored_played=[
                build_card(Rank.KING, Suit.SPADES, Enhancement.MULT),
                build_card(Rank.KING, Suit.DIAMONDS, Enhancement.MULT),
                build_card(Rank.ACE, Suit.HEARTS, Enhancement.GLASS, Seal.RED),
                build_card(Rank.ACE, Suit.CLUBS, edition=Edition.POLYCHROME),
                build_card(Rank.THREE, Suit.SPADES, Enhancement.STONE),
            ],
            unscored_held=[
                build_card(Rank.QUEEN, Suit.HEARTS, Enhancement.GLASS),
                build_card(Rank.QUEEN, Suit.CLUBS, Enhancement.BONUS),
                build_card(Rank.FIVE, Suit.DIAMONDS, Enhancement.GLASS, Seal.RED),
            ],
        ),
        joker_plan=JokerPlan(),
    )

    run_assert(3, cards, jokers, GameState(), expected)


def test_0004_three_kind_scores_best_three_kind():
    cards = [
        build_card(Rank.QUEEN, Suit.HEARTS, Enhancement.GLASS, Seal.RED),
        build_card(Rank.QUEEN, Suit.CLUBS, Enhancement.MULT),
        build_card(Rank.QUEEN, Suit.SPADES, edition=Edition.POLYCHROME),
        build_card(Rank.ACE, Suit.DIAMONDS, Enhancement.BONUS),
        build_card(Rank.JACK, Suit.CLUBS, Enhancement.GLASS),
        build_card(Rank.KING, Suit.HEARTS, Enhancement.MULT),
        build_card(Rank.FIVE, Suit.DIAMONDS, Enhancement.GLASS, Seal.RED),
        build_card(Rank.THREE, Suit.SPADES, Enhancement.STONE),
    ]
    jokers = []

    expected = FinalScoringResults(
        best_hand=BestHand(
            chips=120, worst_case_mult=42, avg_case_mult=42, best_case_mult=42
        ),
        hand_scoring=HandScoring(
            hand_stats=HAND_STATS[PokerHand.THREE_OF_A_KIND],
            scored_played=[
                build_card(Rank.QUEEN, Suit.CLUBS, Enhancement.MULT),
                build_card(Rank.QUEEN, Suit.HEARTS, Enhancement.GLASS, Seal.RED),
                build_card(Rank.QUEEN, Suit.SPADES, edition=Edition.POLYCHROME),
                build_card(Rank.THREE, Suit.SPADES, Enhancement.STONE),
            ],
            unscored_held=[
                build_card(Rank.ACE, Suit.DIAMONDS, Enhancement.BONUS),
                build_card(Rank.JACK, Suit.CLUBS, Enhancement.GLASS),
                build_card(Rank.KING, Suit.HEARTS, Enhancement.MULT),
                build_card(Rank.FIVE, Suit.DIAMONDS, Enhancement.GLASS, Seal.RED),
            ],
        ),
        joker_plan=JokerPlan(),
    )

    run_assert(4, cards, jokers, GameState(), expected)


def test_0005_four_kind_scores_best_four_kind():
    cards = [
        build_card(Rank.KING, Suit.HEARTS, Enhancement.GLASS, Seal.RED),
        build_card(Rank.KING, Suit.CLUBS, Enhancement.MULT),
        build_card(Rank.KING, Suit.SPADES, edition=Edition.POLYCHROME),
        build_card(Rank.KING, Suit.DIAMONDS, Enhancement.BONUS),
        build_card(Rank.QUEEN, Suit.HEARTS, Enhancement.GLASS),
        build_card(Rank.JACK, Suit.CLUBS, Enhancement.MULT),
        build_card(Rank.ACE, Suit.DIAMONDS, Enhancement.GLASS, Seal.RED),
        build_card(Rank.THREE, Suit.SPADES, Enhancement.STONE),
    ]
    jokers = []

    expected = FinalScoringResults(
        best_hand=BestHand(
            chips=190, worst_case_mult=66, avg_case_mult=66, best_case_mult=66
        ),
        hand_scoring=HandScoring(
            hand_stats=HAND_STATS[PokerHand.FOUR_OF_A_KIND],
            scored_played=[
                build_card(Rank.KING, Suit.CLUBS, Enhancement.MULT),
                build_card(Rank.KING, Suit.HEARTS, Enhancement.GLASS, Seal.RED),
                build_card(Rank.KING, Suit.SPADES, edition=Edition.POLYCHROME),
                build_card(Rank.KING, Suit.DIAMONDS, Enhancement.BONUS),
                build_card(Rank.THREE, Suit.SPADES, Enhancement.STONE),
            ],
            unscored_held=[
                build_card(Rank.QUEEN, Suit.HEARTS, Enhancement.GLASS),
                build_card(Rank.JACK, Suit.CLUBS, Enhancement.MULT),
                build_card(Rank.ACE, Suit.DIAMONDS, Enhancement.GLASS, Seal.RED),
            ],
        ),
        joker_plan=JokerPlan(),
    )

    run_assert(5, cards, jokers, GameState(), expected)


def test_0006_straight_scores_best_straight():
    cards = [
        build_card(Rank.ACE, Suit.HEARTS, Enhancement.GLASS, Seal.RED),
        build_card(Rank.KING, Suit.CLUBS, Enhancement.MULT),
        build_card(Rank.QUEEN, Suit.SPADES, edition=Edition.POLYCHROME),
        build_card(Rank.JACK, Suit.DIAMONDS, Enhancement.BONUS),
        build_card(Rank.TEN, Suit.CLUBS, Enhancement.GLASS),
        build_card(Rank.SIX, Suit.HEARTS, Enhancement.LUCKY),
        build_card(Rank.FOUR, Suit.DIAMONDS, Enhancement.MULT),
        build_card(Rank.TWO, Suit.SPADES, Enhancement.STONE),
    ]
    jokers = []

    expected = FinalScoringResults(
        best_hand=BestHand(
            chips=122, worst_case_mult=96, avg_case_mult=96, best_case_mult=96
        ),
        hand_scoring=HandScoring(
            hand_stats=HAND_STATS[PokerHand.STRAIGHT],
            scored_played=[
                build_card(Rank.KING, Suit.CLUBS, Enhancement.MULT),
                build_card(Rank.ACE, Suit.HEARTS, Enhancement.GLASS, Seal.RED),
                build_card(Rank.QUEEN, Suit.SPADES, edition=Edition.POLYCHROME),
                build_card(Rank.TEN, Suit.CLUBS, Enhancement.GLASS),
                build_card(Rank.JACK, Suit.DIAMONDS, Enhancement.BONUS),
            ],
            unscored_held=[
                build_card(Rank.SIX, Suit.HEARTS, Enhancement.LUCKY),
                build_card(Rank.FOUR, Suit.DIAMONDS, Enhancement.MULT),
                build_card(Rank.TWO, Suit.SPADES, Enhancement.STONE),
            ],
        ),
        joker_plan=JokerPlan(),
    )

    run_assert(6, cards, jokers, GameState(), expected)


def test_0007_flush_scores_best_flush():
    cards = [
        build_card(Rank.ACE, Suit.HEARTS, Enhancement.GLASS, Seal.RED),
        build_card(Rank.KING, Suit.HEARTS, Enhancement.MULT),
        build_card(Rank.QUEEN, Suit.HEARTS, edition=Edition.HOLOGRAPHIC),
        build_card(Rank.JACK, Suit.HEARTS, Enhancement.BONUS),
        build_card(Rank.NINE, Suit.HEARTS, Enhancement.LUCKY),
        build_card(Rank.EIGHT, Suit.CLUBS, Enhancement.GLASS),
        build_card(Rank.FIVE, Suit.DIAMONDS, Enhancement.GLASS, Seal.RED),
        build_card(Rank.TWO, Suit.SPADES, Enhancement.STONE),
    ]
    jokers = []

    expected = FinalScoringResults(
        best_hand=BestHand(
            chips=126, worst_case_mult=72, avg_case_mult=88, best_case_mult=152
        ),
        hand_scoring=HandScoring(
            hand_stats=HAND_STATS[PokerHand.FLUSH],
            scored_played=[
                build_card(Rank.NINE, Suit.HEARTS, Enhancement.LUCKY),
                build_card(Rank.KING, Suit.HEARTS, Enhancement.MULT),
                build_card(Rank.QUEEN, Suit.HEARTS, edition=Edition.HOLOGRAPHIC),
                build_card(Rank.ACE, Suit.HEARTS, Enhancement.GLASS, Seal.RED),
                build_card(Rank.JACK, Suit.HEARTS, Enhancement.BONUS),
            ],
            unscored_held=[
                build_card(Rank.EIGHT, Suit.CLUBS, Enhancement.GLASS),
                build_card(Rank.FIVE, Suit.DIAMONDS, Enhancement.GLASS, Seal.RED),
                build_card(Rank.TWO, Suit.SPADES, Enhancement.STONE),
            ],
        ),
        joker_plan=JokerPlan(),
    )

    run_assert(7, cards, jokers, GameState(), expected)


def test_0008_full_house_scores_best_full_house():
    cards = [
        build_card(Rank.ACE, Suit.HEARTS, Enhancement.GLASS, Seal.RED),
        build_card(Rank.ACE, Suit.CLUBS, Enhancement.MULT),
        build_card(Rank.ACE, Suit.SPADES, edition=Edition.HOLOGRAPHIC),
        build_card(Rank.KING, Suit.HEARTS, Enhancement.GLASS),
        build_card(Rank.KING, Suit.CLUBS, Enhancement.BONUS),
        build_card(Rank.QUEEN, Suit.DIAMONDS, Enhancement.MULT),
        build_card(Rank.EIGHT, Suit.HEARTS, Enhancement.GLASS, Seal.RED),
        build_card(Rank.THREE, Suit.SPADES, Enhancement.STONE),
    ]
    jokers = []

    expected = FinalScoringResults(
        best_hand=BestHand(
            chips=134, worst_case_mult=144, avg_case_mult=144, best_case_mult=144
        ),
        hand_scoring=HandScoring(
            hand_stats=HAND_STATS[PokerHand.FULL_HOUSE],
            scored_played=[
                build_card(Rank.ACE, Suit.CLUBS, Enhancement.MULT),
                build_card(Rank.ACE, Suit.SPADES, edition=Edition.HOLOGRAPHIC),
                build_card(Rank.ACE, Suit.HEARTS, Enhancement.GLASS, Seal.RED),
                build_card(Rank.KING, Suit.HEARTS, Enhancement.GLASS),
                build_card(Rank.KING, Suit.CLUBS, Enhancement.BONUS),
            ],
            unscored_held=[
                build_card(Rank.QUEEN, Suit.DIAMONDS, Enhancement.MULT),
                build_card(Rank.EIGHT, Suit.HEARTS, Enhancement.GLASS, Seal.RED),
                build_card(Rank.THREE, Suit.SPADES, Enhancement.STONE),
            ],
        ),
        joker_plan=JokerPlan(),
    )

    run_assert(8, cards, jokers, GameState(), expected)


def test_0009_straight_flush_scores_best_straight_flush():
    cards = [
        build_card(Rank.ACE, Suit.HEARTS, Enhancement.GLASS, Seal.RED),
        build_card(Rank.KING, Suit.HEARTS, Enhancement.MULT),
        build_card(Rank.QUEEN, Suit.HEARTS, edition=Edition.HOLOGRAPHIC),
        build_card(Rank.JACK, Suit.HEARTS, Enhancement.BONUS),
        build_card(Rank.TEN, Suit.HEARTS, Enhancement.GLASS),
        build_card(Rank.NINE, Suit.HEARTS, Enhancement.LUCKY),
        build_card(Rank.EIGHT, Suit.CLUBS, Enhancement.BONUS),
        build_card(Rank.FIVE, Suit.DIAMONDS, Enhancement.GLASS, Seal.RED),
    ]
    jokers = []

    expected = FinalScoringResults(
        best_hand=BestHand(
            chips=192, worst_case_mult=176, avg_case_mult=176, best_case_mult=176
        ),
        hand_scoring=HandScoring(
            hand_stats=HAND_STATS[PokerHand.STRAIGHT_FLUSH],
            scored_played=[
                build_card(Rank.KING, Suit.HEARTS, Enhancement.MULT),
                build_card(Rank.QUEEN, Suit.HEARTS, edition=Edition.HOLOGRAPHIC),
                build_card(Rank.ACE, Suit.HEARTS, Enhancement.GLASS, Seal.RED),
                build_card(Rank.TEN, Suit.HEARTS, Enhancement.GLASS),
                build_card(Rank.JACK, Suit.HEARTS, Enhancement.BONUS),
            ],
            unscored_held=[
                build_card(Rank.NINE, Suit.HEARTS, Enhancement.LUCKY),
                build_card(Rank.EIGHT, Suit.CLUBS, Enhancement.BONUS),
                build_card(Rank.FIVE, Suit.DIAMONDS, Enhancement.GLASS, Seal.RED),
            ],
        ),
        joker_plan=JokerPlan(),
    )

    run_assert(9, cards, jokers, GameState(), expected)


def test_0010_five_kind_scores_best_five_kind():
    cards = [
        build_card(Rank.ACE, Suit.HEARTS, Enhancement.GLASS, Seal.RED),
        build_card(Rank.ACE, Suit.CLUBS, Enhancement.MULT),
        build_card(Rank.ACE, Suit.SPADES, edition=Edition.HOLOGRAPHIC),
        build_card(Rank.ACE, Suit.DIAMONDS, Enhancement.BONUS),
        build_card(Rank.ACE, Suit.HEARTS, edition=Edition.POLYCHROME),
        build_card(Rank.KING, Suit.CLUBS, Enhancement.MULT),
        build_card(Rank.KING, Suit.SPADES),
        build_card(Rank.FOUR, Suit.DIAMONDS, Enhancement.MULT),
    ]
    jokers = []

    expected = FinalScoringResults(
        best_hand=BestHand(
            chips=216, worst_case_mult=156, avg_case_mult=156, best_case_mult=156
        ),
        hand_scoring=HandScoring(
            hand_stats=HAND_STATS[PokerHand.FIVE_OF_A_KIND],
            scored_played=[
                build_card(Rank.ACE, Suit.CLUBS, Enhancement.MULT),
                build_card(Rank.ACE, Suit.SPADES, edition=Edition.HOLOGRAPHIC),
                build_card(Rank.ACE, Suit.HEARTS, Enhancement.GLASS, Seal.RED),
                build_card(Rank.ACE, Suit.HEARTS, edition=Edition.POLYCHROME),
                build_card(Rank.ACE, Suit.DIAMONDS, Enhancement.BONUS),
            ],
            unscored_held=[
                build_card(Rank.KING, Suit.CLUBS, Enhancement.MULT),
                build_card(Rank.KING, Suit.SPADES),
                build_card(Rank.FOUR, Suit.DIAMONDS, Enhancement.MULT),
            ],
        ),
        joker_plan=JokerPlan(),
    )

    run_assert(10, cards, jokers, GameState(), expected)


def test_0011_photograph_hanging_chad_vs_polychrome_lucky_king():
    cards = [
        build_card(
            Rank.KING, Suit.HEARTS, Enhancement.LUCKY, Seal.RED, Edition.POLYCHROME
        ),
        build_card(Rank.ACE, Suit.SPADES, edition=Edition.FOIL),
        build_card(Rank.QUEEN, Suit.DIAMONDS, Enhancement.GLASS),
        build_card(Rank.NINE, Suit.CLUBS, Enhancement.LUCKY),
        build_card(Rank.FIVE, Suit.HEARTS, Enhancement.MULT),
        build_card(Rank.TWO, Suit.SPADES, Enhancement.BONUS),
        build_card(
            Rank.JACK, Suit.CLUBS, Enhancement.WILD, edition=Edition.HOLOGRAPHIC
        ),
        build_card(Rank.THREE, Suit.DIAMONDS),
    ]
    jokers = [
        Joker.build(JokersName.PHOTOGRAPH),
        Joker.build(JokersName.HANGING_CHAD),
        Joker.build(JokersName.OOPS_ALL_6S),
    ]

    game_state = build_test_game_state(jokers, cards)

    expected = FinalScoringResults(
        best_hand=BestHand(
            chips=45, worst_case_mult=81, avg_case_mult=1041, best_case_mult=2481
        ),
        hand_scoring=HandScoring(
            hand_stats=HAND_STATS[PokerHand.HIGH_CARD],
            scored_played=[
                build_card(
                    Rank.KING,
                    Suit.HEARTS,
                    Enhancement.LUCKY,
                    Seal.RED,
                    Edition.POLYCHROME,
                ),
            ],
            unscored_held=[
                build_card(
                    Rank.JACK, Suit.CLUBS, Enhancement.WILD, edition=Edition.HOLOGRAPHIC
                ),
                build_card(Rank.ACE, Suit.SPADES, edition=Edition.FOIL),
                build_card(Rank.QUEEN, Suit.DIAMONDS, Enhancement.GLASS),
                build_card(Rank.NINE, Suit.CLUBS, Enhancement.LUCKY),
                build_card(Rank.FIVE, Suit.HEARTS, Enhancement.MULT),
                build_card(Rank.TWO, Suit.SPADES, Enhancement.BONUS),
                build_card(Rank.THREE, Suit.DIAMONDS),
            ],
        ),
        joker_plan=build_joker_plan(jokers),
    )

    run_assert(11, cards, jokers, game_state, expected)


def test_0012_glass_queen_vs_lucky_polychrome_king_as_first_card():
    cards = [
        build_card(Rank.QUEEN, Suit.SPADES, Enhancement.GLASS, Seal.RED),
        build_card(
            Rank.KING, Suit.DIAMONDS, Enhancement.LUCKY, edition=Edition.POLYCHROME
        ),
        build_card(Rank.ACE, Suit.HEARTS, edition=Edition.FOIL),
        build_card(Rank.TEN, Suit.SPADES, Enhancement.MULT),
        build_card(Rank.SEVEN, Suit.CLUBS, Enhancement.LUCKY),
        build_card(Rank.FOUR, Suit.DIAMONDS, Enhancement.BONUS),
        build_card(Rank.TWO, Suit.HEARTS),
        build_card(Rank.JACK, Suit.HEARTS, Enhancement.GOLD),
    ]
    jokers = [
        Joker.build(JokersName.PHOTOGRAPH),
        Joker.build(JokersName.HANGING_CHAD),
        Joker.build(JokersName.SOCK_AND_BUSKIN),
    ]

    game_state = build_test_game_state(jokers, cards)

    expected = FinalScoringResults(
        best_hand=BestHand(
            chips=191, worst_case_mult=9225, avg_case_mult=9240, best_case_mult=9300
        ),
        hand_scoring=HandScoring(
            hand_stats=HAND_STATS[PokerHand.STRAIGHT],
            scored_played=[
                build_card(Rank.QUEEN, Suit.SPADES, Enhancement.GLASS, Seal.RED),
                build_card(Rank.ACE, Suit.HEARTS, edition=Edition.FOIL),
                build_card(Rank.JACK, Suit.HEARTS, Enhancement.GOLD),
                build_card(Rank.TEN, Suit.SPADES, Enhancement.MULT),
                build_card(
                    Rank.KING,
                    Suit.DIAMONDS,
                    Enhancement.LUCKY,
                    edition=Edition.POLYCHROME,
                ),
            ],
            unscored_held=[
                build_card(Rank.SEVEN, Suit.CLUBS, Enhancement.LUCKY),
                build_card(Rank.FOUR, Suit.DIAMONDS, Enhancement.BONUS),
                build_card(Rank.TWO, Suit.HEARTS),
            ],
        ),
        joker_plan=build_joker_plan(jokers),
    )

    run_assert(12, cards, jokers, game_state, expected)


def test_0013_baron_mime_wants_kings_held_not_played():
    cards = [
        build_card(Rank.KING, Suit.SPADES, Enhancement.STEEL, Seal.RED),
        build_card(
            Rank.KING, Suit.HEARTS, Enhancement.STEEL, edition=Edition.POLYCHROME
        ),
        build_card(Rank.KING, Suit.DIAMONDS, Enhancement.GOLD),
        build_card(Rank.ACE, Suit.CLUBS, edition=Edition.FOIL),
        build_card(Rank.ACE, Suit.DIAMONDS, Enhancement.GLASS),
        build_card(Rank.ACE, Suit.HEARTS, Enhancement.MULT),
        build_card(Rank.NINE, Suit.SPADES),
        build_card(Rank.THREE, Suit.CLUBS),
    ]
    jokers = [
        Joker.build(JokersName.BARON),
        Joker.build(JokersName.MIME),
        Joker.build(JokersName.RAISED_FIST),
    ]

    game_state = build_test_game_state(jokers)

    expected = FinalScoringResults(
        best_hand=BestHand(
            chips=113,
            worst_case_mult=6141.326,
            avg_case_mult=6141.326,
            best_case_mult=6141.326,
        ),
        hand_scoring=HandScoring(
            hand_stats=HAND_STATS[PokerHand.THREE_OF_A_KIND],
            scored_played=[
                build_card(Rank.ACE, Suit.CLUBS, edition=Edition.FOIL),
                build_card(Rank.ACE, Suit.HEARTS, Enhancement.MULT),
                build_card(Rank.ACE, Suit.DIAMONDS, Enhancement.GLASS),
            ],
            unscored_played=[
                build_card(Rank.NINE, Suit.SPADES),
                build_card(Rank.THREE, Suit.CLUBS),
            ],
            scored_held=[
                build_card(Rank.KING, Suit.SPADES, Enhancement.STEEL, Seal.RED),
                build_card(
                    Rank.KING,
                    Suit.HEARTS,
                    Enhancement.STEEL,
                    edition=Edition.POLYCHROME,
                ),
                build_card(Rank.KING, Suit.DIAMONDS, Enhancement.GOLD),
            ],
        ),
        joker_plan=build_joker_plan(jokers),
    )

    run_assert(13, cards, jokers, game_state, expected)


def test_0014_full_house_available_but_held_baron_kings_may_dominate():
    cards = [
        build_card(Rank.KING, Suit.SPADES, Enhancement.STEEL, Seal.RED),
        build_card(Rank.KING, Suit.HEARTS, Enhancement.STEEL),
        build_card(Rank.KING, Suit.DIAMONDS, Enhancement.STEEL, edition=Edition.FOIL),
        build_card(Rank.NINE, Suit.SPADES, Enhancement.GLASS),
        build_card(Rank.NINE, Suit.HEARTS, Enhancement.MULT),
        build_card(Rank.NINE, Suit.DIAMONDS, Enhancement.LUCKY),
        build_card(Rank.TWO, Suit.CLUBS, Enhancement.BONUS),
        build_card(Rank.FIVE, Suit.DIAMONDS),
    ]
    jokers = [
        Joker.build(JokersName.BARON),
        Joker.build(JokersName.MIME),
        Joker.build(JokersName.THE_TRIO),
    ]

    game_state = build_test_game_state(jokers, cards)

    expected = FinalScoringResults(
        best_hand=BestHand(
            chips=57,
            worst_case_mult=12261.028,
            avg_case_mult=19267.331,
            best_case_mult=47292.540,
        ),
        hand_scoring=HandScoring(
            hand_stats=HAND_STATS[PokerHand.THREE_OF_A_KIND],
            scored_played=[
                build_card(Rank.NINE, Suit.SPADES, Enhancement.GLASS),
                build_card(Rank.NINE, Suit.HEARTS, Enhancement.MULT),
                build_card(Rank.NINE, Suit.DIAMONDS, Enhancement.LUCKY),
            ],
            scored_held=[
                build_card(Rank.KING, Suit.SPADES, Enhancement.STEEL, Seal.RED),
                build_card(Rank.KING, Suit.HEARTS, Enhancement.STEEL),
                build_card(
                    Rank.KING, Suit.DIAMONDS, Enhancement.STEEL, edition=Edition.FOIL
                ),
            ],
            unscored_held=[
                build_card(Rank.TWO, Suit.CLUBS, Enhancement.BONUS),
                build_card(Rank.FIVE, Suit.DIAMONDS),
            ],
        ),
        joker_plan=build_joker_plan(jokers),
    )

    run_assert(14, cards, jokers, game_state, expected)


def test_0015_mime_steel_red_seal_ace_vs_baron_king():
    cards = [
        build_card(Rank.KING, Suit.CLUBS, Enhancement.STEEL),
        build_card(
            Rank.ACE, Suit.SPADES, Enhancement.STEEL, Seal.RED, Edition.POLYCHROME
        ),
        build_card(Rank.QUEEN, Suit.HEARTS, Enhancement.STEEL),
        build_card(Rank.TEN, Suit.DIAMONDS, Enhancement.GLASS),
        build_card(Rank.TEN, Suit.SPADES, Enhancement.MULT),
        build_card(Rank.TEN, Suit.CLUBS, Enhancement.LUCKY),
        build_card(Rank.FOUR, Suit.HEARTS),
        build_card(Rank.TWO, Suit.DIAMONDS),
    ]
    jokers = [
        Joker.build(JokersName.MIME),
        Joker.build(JokersName.BARON),
        Joker.build(JokersName.RAISED_FIST),
    ]

    game_state = build_test_game_state(jokers, cards)

    expected = FinalScoringResults(
        best_hand=BestHand(
            chips=60,
            worst_case_mult=1392.504,
            avg_case_mult=1700.051,
            best_case_mult=2930.238,
        ),
        hand_scoring=HandScoring(
            hand_stats=HAND_STATS[PokerHand.THREE_OF_A_KIND],
            scored_played=[
                build_card(Rank.TEN, Suit.DIAMONDS, Enhancement.GLASS),
                build_card(Rank.TEN, Suit.SPADES, Enhancement.MULT),
                build_card(Rank.TEN, Suit.CLUBS, Enhancement.LUCKY),
            ],
            scored_held=[
                build_card(Rank.KING, Suit.CLUBS, Enhancement.STEEL),
                build_card(
                    Rank.ACE,
                    Suit.SPADES,
                    Enhancement.STEEL,
                    Seal.RED,
                    Edition.POLYCHROME,
                ),
                build_card(Rank.QUEEN, Suit.HEARTS, Enhancement.STEEL),
            ],
            unscored_played=[
                build_card(Rank.FOUR, Suit.HEARTS),
                build_card(Rank.TWO, Suit.DIAMONDS),
            ],
        ),
        joker_plan=build_joker_plan(jokers),
    )

    run_assert(15, cards, jokers, game_state, expected)


def test_0016_pareidolia_photograph_midas_vampire():
    cards = [
        build_card(Rank.TWO, Suit.HEARTS, Enhancement.LUCKY, Seal.RED),
        build_card(Rank.FIVE, Suit.DIAMONDS, Enhancement.GLASS, edition=Edition.FOIL),
        build_card(Rank.EIGHT, Suit.CLUBS, Enhancement.MULT),
        build_card(Rank.ACE, Suit.SPADES, edition=Edition.POLYCHROME),
        build_card(Rank.KING, Suit.HEARTS, Enhancement.GOLD),
        build_card(Rank.JACK, Suit.DIAMONDS, Enhancement.WILD),
        build_card(Rank.THREE, Suit.SPADES, Enhancement.BONUS),
        build_card(Rank.TEN, Suit.CLUBS),
    ]
    jokers = [
        Joker.build(JokersName.PAREIDOLIA),
        Joker.build(JokersName.PHOTOGRAPH),
        Joker.build(JokersName.HANGING_CHAD),
        Joker.build(JokersName.MIDAS_MASK),
        Joker.build(JokersName.VAMPIRE),
    ]

    game_state = build_test_game_state(jokers, cards)

    expected = FinalScoringResults(
        best_hand=BestHand(
            chips=60,
            worst_case_mult=1392.504,
            avg_case_mult=1700.051,
            best_case_mult=2930.238,
        ),
        hand_scoring=HandScoring(
            hand_stats=HAND_STATS[PokerHand.THREE_OF_A_KIND],
            scored_played=[
                build_card(Rank.TEN, Suit.DIAMONDS, Enhancement.GLASS),
                build_card(Rank.TEN, Suit.SPADES, Enhancement.MULT),
                build_card(Rank.TEN, Suit.CLUBS, Enhancement.LUCKY),
            ],
            scored_held=[
                build_card(Rank.KING, Suit.CLUBS, Enhancement.STEEL),
                build_card(
                    Rank.ACE,
                    Suit.SPADES,
                    Enhancement.STEEL,
                    Seal.RED,
                    Edition.POLYCHROME,
                ),
                build_card(Rank.QUEEN, Suit.HEARTS, Enhancement.STEEL),
            ],
            unscored_played=[
                build_card(Rank.FOUR, Suit.HEARTS),
                build_card(Rank.TWO, Suit.DIAMONDS),
            ],
        ),
        joker_plan=build_joker_plan(jokers),
    )

    assert_final_scoring_results(best_score, expected)


# def test_0017_wild_polychrome_jack_creates_flush_and_retrigger_bait():
#     cards = [
#         build_card(
#             Rank.JACK, Suit.CLUBS, Enhancement.WILD, Seal.RED, Edition.POLYCHROME
#         ),
#         build_card(Rank.ACE, Suit.HEARTS, Enhancement.GLASS),
#         build_card(Rank.TEN, Suit.HEARTS, Enhancement.LUCKY),
#         build_card(Rank.SEVEN, Suit.HEARTS, Enhancement.MULT),
#         build_card(Rank.FOUR, Suit.HEARTS, Enhancement.BONUS),
#         build_card(Rank.KING, Suit.SPADES, Enhancement.STEEL),
#         build_card(Rank.QUEEN, Suit.DIAMONDS, Enhancement.GOLD),
#         build_card(Rank.TWO, Suit.CLUBS),
#     ]
#     jokers = [
#         Joker.build(JokersName.BLOODSTONE),
#         Joker.build(JokersName.PHOTOGRAPH),
#         Joker.build(JokersName.HANGING_CHAD),
#     ]

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0018_bloodstone_ev_vs_photograph_order():
#     cards = [
#         build_card(Rank.KING, Suit.HEARTS, Enhancement.GLASS, Seal.RED),
#         build_card(
#             Rank.QUEEN, Suit.HEARTS, Enhancement.LUCKY, edition=Edition.POLYCHROME
#         ),
#         build_card(Rank.JACK, Suit.HEARTS, Enhancement.MULT),
#         build_card(Rank.NINE, Suit.HEARTS, Enhancement.BONUS),
#         build_card(Rank.THREE, Suit.HEARTS),
#         build_card(Rank.ACE, Suit.SPADES, Enhancement.STEEL),
#         build_card(Rank.EIGHT, Suit.DIAMONDS, Enhancement.LUCKY),
#         build_card(Rank.TWO, Suit.CLUBS),
#     ]
#     jokers = [
#         Joker.build(JokersName.BLOODSTONE),
#         Joker.build(JokersName.PHOTOGRAPH),
#         Joker.build(JokersName.HANGING_CHAD),
#         Joker.build(JokersName.SOCK_AND_BUSKIN),
#     ]

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0019_oops_lucky_cat_multiple_lucky_candidates():
#     cards = [
#         build_card(Rank.ACE, Suit.DIAMONDS, Enhancement.LUCKY, Seal.RED),
#         build_card(
#             Rank.KING, Suit.CLUBS, Enhancement.LUCKY, edition=Edition.POLYCHROME
#         ),
#         build_card(Rank.QUEEN, Suit.HEARTS, Enhancement.LUCKY),
#         build_card(Rank.JACK, Suit.SPADES, Enhancement.LUCKY, edition=Edition.FOIL),
#         build_card(Rank.TEN, Suit.DIAMONDS, Enhancement.LUCKY),
#         build_card(Rank.SIX, Suit.CLUBS, Enhancement.GLASS),
#         build_card(Rank.SIX, Suit.HEARTS, Enhancement.MULT),
#         build_card(Rank.TWO, Suit.SPADES),
#     ]
#     jokers = [
#         Joker.build(JokersName.LUCKY_CAT),
#         Joker.build(JokersName.OOPS_ALL_6S),
#         Joker.build(JokersName.HANGING_CHAD),
#         Joker.build(JokersName.PHOTOGRAPH),
#     ]

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0020_splash_makes_junk_enhanced_cards_score():
#     cards = [
#         build_card(Rank.ACE, Suit.SPADES),
#         build_card(Rank.ACE, Suit.DIAMONDS),
#         build_card(Rank.SEVEN, Suit.HEARTS, Enhancement.GLASS, Seal.RED),
#         build_card(
#             Rank.FIVE, Suit.CLUBS, Enhancement.LUCKY, edition=Edition.POLYCHROME
#         ),
#         build_card(Rank.THREE, Suit.DIAMONDS, Enhancement.MULT),
#         build_card(Rank.KING, Suit.HEARTS, Enhancement.STEEL),
#         build_card(Rank.QUEEN, Suit.CLUBS, Enhancement.GOLD),
#         build_card(Rank.TWO, Suit.SPADES, Enhancement.BONUS),
#     ]
#     jokers = [
#         Joker.build(JokersName.SPLASH),
#         Joker.build(JokersName.HANGING_CHAD),
#         Joker.build(JokersName.OOPS_ALL_6S),
#     ]

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0021_splash_photograph_first_card_scores_outside_main_pair():
#     cards = [
#         build_card(Rank.KING, Suit.DIAMONDS, Enhancement.GLASS, Seal.RED),
#         build_card(Rank.ACE, Suit.SPADES),
#         build_card(Rank.ACE, Suit.HEARTS),
#         build_card(Rank.EIGHT, Suit.CLUBS, Enhancement.LUCKY),
#         build_card(Rank.FOUR, Suit.SPADES, Enhancement.MULT),
#         build_card(Rank.TWO, Suit.DIAMONDS, Enhancement.BONUS),
#         build_card(Rank.QUEEN, Suit.HEARTS, Enhancement.STEEL),
#         build_card(Rank.JACK, Suit.CLUBS, Enhancement.GOLD),
#     ]
#     jokers = [
#         Joker.build(JokersName.SPLASH),
#         Joker.build(JokersName.PHOTOGRAPH),
#         Joker.build(JokersName.HANGING_CHAD),
#     ]

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0022_sock_and_buskin_red_seal_face_card_pile():
#     cards = [
#         build_card(Rank.KING, Suit.SPADES, Enhancement.GLASS, Seal.RED),
#         build_card(
#             Rank.KING, Suit.HEARTS, Enhancement.LUCKY, edition=Edition.POLYCHROME
#         ),
#         build_card(Rank.QUEEN, Suit.DIAMONDS, Enhancement.MULT, Seal.RED),
#         build_card(
#             Rank.JACK, Suit.CLUBS, Enhancement.WILD, edition=Edition.HOLOGRAPHIC
#         ),
#         build_card(Rank.TEN, Suit.SPADES),
#         build_card(Rank.SIX, Suit.HEARTS, Enhancement.BONUS),
#         build_card(Rank.THREE, Suit.CLUBS),
#         build_card(Rank.TWO, Suit.DIAMONDS),
#     ]
#     jokers = [
#         Joker.build(JokersName.SOCK_AND_BUSKIN),
#         Joker.build(JokersName.PHOTOGRAPH),
#         Joker.build(JokersName.HANGING_CHAD),
#     ]

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0023_hack_retriggers_low_enhanced_cards():
#     cards = [
#         build_card(Rank.FIVE, Suit.HEARTS, Enhancement.GLASS, Seal.RED, Edition.FOIL),
#         build_card(Rank.FIVE, Suit.CLUBS, Enhancement.LUCKY),
#         build_card(Rank.FIVE, Suit.DIAMONDS, Enhancement.MULT),
#         build_card(
#             Rank.FOUR, Suit.SPADES, Enhancement.LUCKY, edition=Edition.POLYCHROME
#         ),
#         build_card(Rank.THREE, Suit.HEARTS, Enhancement.BONUS),
#         build_card(Rank.KING, Suit.SPADES, Enhancement.STEEL),
#         build_card(Rank.QUEEN, Suit.DIAMONDS, Enhancement.GOLD),
#         build_card(Rank.ACE, Suit.CLUBS),
#     ]
#     jokers = [
#         Joker.build(JokersName.HACK),
#         Joker.build(JokersName.HANGING_CHAD),
#         Joker.build(JokersName.OOPS_ALL_6S),
#     ]

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0024_hack_fibonacci_lucky_ev():
#     cards = [
#         build_card(Rank.TWO, Suit.SPADES, Enhancement.LUCKY, Seal.RED),
#         build_card(Rank.THREE, Suit.HEARTS, Enhancement.GLASS),
#         build_card(
#             Rank.FIVE, Suit.DIAMONDS, Enhancement.LUCKY, edition=Edition.POLYCHROME
#         ),
#         build_card(Rank.FIVE, Suit.CLUBS, Enhancement.MULT),
#         build_card(Rank.FIVE, Suit.HEARTS, Enhancement.BONUS),
#         build_card(Rank.EIGHT, Suit.SPADES, Enhancement.LUCKY),
#         build_card(Rank.KING, Suit.DIAMONDS, Enhancement.STEEL),
#         build_card(Rank.QUEEN, Suit.CLUBS),
#     ]
#     jokers = [
#         Joker.build(JokersName.HACK),
#         Joker.build(JokersName.FIBONACCI),
#         Joker.build(JokersName.LUCKY_CAT),
#         Joker.build(JokersName.OOPS_ALL_6S),
#     ]

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0025_idol_picks_one_exact_red_seal_card():
#     cards = [
#         build_card(Rank.QUEEN, Suit.HEARTS, Enhancement.GLASS, Seal.RED),
#         build_card(
#             Rank.QUEEN, Suit.DIAMONDS, Enhancement.LUCKY, edition=Edition.POLYCHROME
#         ),
#         build_card(Rank.QUEEN, Suit.CLUBS, Enhancement.MULT),
#         build_card(Rank.NINE, Suit.SPADES, Enhancement.BONUS),
#         build_card(Rank.SIX, Suit.HEARTS, Enhancement.LUCKY),
#         build_card(Rank.KING, Suit.CLUBS, Enhancement.STEEL),
#         build_card(Rank.ACE, Suit.DIAMONDS),
#         build_card(Rank.TWO, Suit.SPADES),
#     ]
#     jokers = [
#         build_joker(JokersName.THE_IDOL, req_rank=Rank.QUEEN, req_suit=Suit.HEARTS),
#         Joker.build(JokersName.SOCK_AND_BUSKIN),
#         Joker.build(JokersName.HANGING_CHAD),
#     ]

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0026_reserved_parking_held_face_cards():
#     cards = [
#         build_card(Rank.KING, Suit.SPADES, Enhancement.GOLD, Seal.RED),
#         build_card(Rank.QUEEN, Suit.HEARTS, Enhancement.GOLD),
#         build_card(
#             Rank.JACK, Suit.DIAMONDS, Enhancement.GOLD, edition=Edition.POLYCHROME
#         ),
#         build_card(Rank.ACE, Suit.SPADES, Enhancement.GLASS),
#         build_card(Rank.ACE, Suit.HEARTS, Enhancement.MULT),
#         build_card(Rank.ACE, Suit.DIAMONDS, Enhancement.LUCKY),
#         build_card(Rank.SEVEN, Suit.CLUBS, Enhancement.BONUS),
#         build_card(Rank.TWO, Suit.HEARTS),
#     ]
#     jokers = [
#         Joker.build(JokersName.RESERVED_PARKING),
#         Joker.build(JokersName.MIME),
#         Joker.build(JokersName.THE_TRIO),
#     ]

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0027_raised_fist_lowest_held_card_and_steel_kings():
#     cards = [
#         build_card(Rank.TWO, Suit.CLUBS, Enhancement.STEEL, Seal.RED),
#         build_card(Rank.KING, Suit.SPADES, Enhancement.STEEL),
#         build_card(
#             Rank.KING, Suit.HEARTS, Enhancement.STEEL, edition=Edition.POLYCHROME
#         ),
#         build_card(Rank.TEN, Suit.DIAMONDS, Enhancement.GLASS),
#         build_card(Rank.TEN, Suit.SPADES, Enhancement.MULT),
#         build_card(Rank.TEN, Suit.HEARTS, Enhancement.LUCKY),
#         build_card(Rank.SEVEN, Suit.CLUBS, Enhancement.BONUS),
#         build_card(Rank.ACE, Suit.DIAMONDS),
#     ]
#     jokers = [
#         Joker.build(JokersName.RAISED_FIST),
#         Joker.build(JokersName.BARON),
#         Joker.build(JokersName.MIME),
#     ]

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0028_business_card_face_card_economy_with_hanging_chad():
#     cards = [
#         build_card(Rank.JACK, Suit.SPADES, Enhancement.LUCKY, Seal.RED),
#         build_card(Rank.QUEEN, Suit.CLUBS, Enhancement.GLASS, edition=Edition.FOIL),
#         build_card(
#             Rank.KING, Suit.DIAMONDS, Enhancement.MULT, edition=Edition.POLYCHROME
#         ),
#         build_card(Rank.TEN, Suit.HEARTS),
#         build_card(Rank.EIGHT, Suit.DIAMONDS, Enhancement.BONUS),
#         build_card(Rank.FIVE, Suit.CLUBS, Enhancement.LUCKY),
#         build_card(Rank.THREE, Suit.SPADES),
#         build_card(Rank.ACE, Suit.HEARTS, Enhancement.STEEL),
#     ]
#     jokers = [
#         Joker.build(JokersName.BUSINESS_CARD),
#         Joker.build(JokersName.SOCK_AND_BUSKIN),
#         Joker.build(JokersName.HANGING_CHAD),
#         Joker.build(JokersName.PHOTOGRAPH),
#     ]

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0029_ancient_joker_suit_target_with_red_seal_glass_card():
#     cards = [
#         build_card(Rank.ACE, Suit.SPADES, Enhancement.GLASS, Seal.RED),
#         build_card(
#             Rank.KING, Suit.SPADES, Enhancement.LUCKY, edition=Edition.POLYCHROME
#         ),
#         build_card(Rank.QUEEN, Suit.SPADES, Enhancement.MULT),
#         build_card(Rank.JACK, Suit.SPADES, Enhancement.WILD),
#         build_card(Rank.TEN, Suit.SPADES, Enhancement.BONUS),
#         build_card(Rank.NINE, Suit.HEARTS, Enhancement.LUCKY),
#         build_card(Rank.FOUR, Suit.DIAMONDS, Enhancement.STEEL),
#         build_card(Rank.TWO, Suit.CLUBS),
#     ]
#     jokers = [
#         build_joker(JokersName.ANCIENT_JOKER, req_suit=Suit.SPADES),
#         Joker.build(JokersName.PHOTOGRAPH),
#         Joker.build(JokersName.HANGING_CHAD),
#     ]

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)


# def test_0030_full_scoring_chaos_splash_lucky_glass_held_steel():
#     cards = [
#         build_card(Rank.ACE, Suit.HEARTS, Enhancement.STEEL, Seal.RED),
#         build_card(
#             Rank.KING, Suit.CLUBS, Enhancement.STEEL, edition=Edition.POLYCHROME
#         ),
#         build_card(Rank.QUEEN, Suit.DIAMONDS, Enhancement.GOLD),
#         build_card(Rank.NINE, Suit.SPADES, Enhancement.GLASS, Seal.RED),
#         build_card(Rank.NINE, Suit.HEARTS, Enhancement.LUCKY),
#         build_card(Rank.SIX, Suit.CLUBS, Enhancement.MULT, edition=Edition.FOIL),
#         build_card(Rank.FOUR, Suit.DIAMONDS, Enhancement.BONUS),
#         build_card(Rank.TWO, Suit.SPADES, Enhancement.LUCKY),
#     ]
#     jokers = [
#         Joker.build(JokersName.SPLASH),
#         Joker.build(JokersName.HANGING_CHAD),
#         Joker.build(JokersName.MIME),
#         Joker.build(JokersName.BARON),
#         Joker.build(JokersName.OOPS_ALL_6S),
#     ]

#     best_score = get_best_scoring_hand(cards, jokers)

#     expected = FinalScoringResults()

#     assert_final_scoring_results(best_score, expected)
