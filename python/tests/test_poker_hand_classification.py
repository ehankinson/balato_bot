"""Classification tests for every hand returned by find_best_hand_type.

Each poker hand has one ordinary example and three edge cases.  The final
section contains near-misses that guard against awarding five-card hands to
short or incomplete selections.
"""

from calculation.poker_eval import find_best_hand_type
from config.poker_hands import HAND_STATS
from core.enums import Enhancement, PokerHand, Rank, Suit
from core.models import Card


EXPECTED_SCORING_CARDS = {
    PokerHand.HIGH_CARD: 1,
    PokerHand.PAIR: 2,
    PokerHand.THREE_OF_A_KIND: 3,
    PokerHand.FOUR_OF_A_KIND: 4,
    PokerHand.FIVE_OF_A_KIND: 5,
    PokerHand.TWO_PAIR: 4,
    PokerHand.STRAIGHT: 5,
    PokerHand.FLUSH: 5,
    PokerHand.FULL_HOUSE: 5,
    PokerHand.STRAIGHT_FLUSH: 5,
    PokerHand.FLUSH_HOUSE: 5,
    PokerHand.FLUSH_FIVE: 5,
}


def card(
    rank: Rank,
    suit: Suit,
    enhancement: Enhancement = Enhancement.NONE,
) -> Card:
    return Card(rank=rank, suit=suit, enhancement=enhancement)


def assert_hand(expected: PokerHand, cards: list[Card]) -> None:
    assert 1 <= len(cards) <= 5
    hand_stats, scored_cards = find_best_hand_type(cards)
    assert hand_stats == HAND_STATS[expected]
    assert len(scored_cards) == EXPECTED_SCORING_CARDS[expected]
    assert all(scored_card in cards for scored_card in scored_cards)


# High card


def test_0001_high_card_regular_five_card_hand():
    assert_hand(PokerHand.HIGH_CARD, [
        card(Rank.ACE, Suit.HEARTS),
        card(Rank.JACK, Suit.CLUBS),
        card(Rank.EIGHT, Suit.DIAMONDS),
        card(Rank.FIVE, Suit.SPADES),
        card(Rank.TWO, Suit.HEARTS),
    ])


def test_0002_high_card_from_two_unrelated_cards():
    assert_hand(PokerHand.HIGH_CARD, [
        card(Rank.ACE, Suit.HEARTS),
        card(Rank.SEVEN, Suit.CLUBS),
    ])


def test_0003_high_card_from_four_unpaired_cards():
    assert_hand(PokerHand.HIGH_CARD, [
        card(Rank.KING, Suit.CLUBS),
        card(Rank.TEN, Suit.DIAMONDS),
        card(Rank.SIX, Suit.SPADES),
        card(Rank.THREE, Suit.HEARTS),
    ])


def test_0004_high_card_with_four_cards_sharing_a_suit():
    assert_hand(PokerHand.HIGH_CARD, [
        card(Rank.ACE, Suit.HEARTS),
        card(Rank.QUEEN, Suit.HEARTS),
        card(Rank.NINE, Suit.HEARTS),
        card(Rank.SIX, Suit.HEARTS),
        card(Rank.TWO, Suit.CLUBS),
    ])


# Pair


def test_0005_pair_regular_two_card_hand():
    assert_hand(PokerHand.PAIR, [
        card(Rank.ACE, Suit.HEARTS),
        card(Rank.ACE, Suit.CLUBS),
    ])


def test_0006_pair_with_one_kicker():
    assert_hand(PokerHand.PAIR, [
        card(Rank.KING, Suit.HEARTS),
        card(Rank.KING, Suit.DIAMONDS),
        card(Rank.FOUR, Suit.CLUBS),
    ])


def test_0007_pair_with_three_kickers():
    assert_hand(PokerHand.PAIR, [
        card(Rank.QUEEN, Suit.HEARTS),
        card(Rank.QUEEN, Suit.CLUBS),
        card(Rank.NINE, Suit.DIAMONDS),
        card(Rank.FIVE, Suit.SPADES),
        card(Rank.TWO, Suit.HEARTS),
    ])


def test_0008_pair_with_wild_kicker_does_not_become_flush():
    assert_hand(PokerHand.PAIR, [
        card(Rank.TEN, Suit.HEARTS),
        card(Rank.TEN, Suit.CLUBS),
        card(Rank.SEVEN, Suit.DIAMONDS, Enhancement.WILD),
        card(Rank.FOUR, Suit.SPADES),
        card(Rank.TWO, Suit.DIAMONDS),
    ])


# Three of a kind


def test_0009_three_of_a_kind_regular():
    assert_hand(PokerHand.THREE_OF_A_KIND, [
        card(Rank.NINE, Suit.HEARTS),
        card(Rank.NINE, Suit.CLUBS),
        card(Rank.NINE, Suit.DIAMONDS),
    ])


def test_0010_three_of_a_kind_with_one_kicker():
    assert_hand(PokerHand.THREE_OF_A_KIND, [
        card(Rank.ACE, Suit.HEARTS),
        card(Rank.ACE, Suit.CLUBS),
        card(Rank.ACE, Suit.DIAMONDS),
        card(Rank.FOUR, Suit.SPADES),
    ])


def test_0011_three_of_a_kind_with_two_distinct_kickers():
    assert_hand(PokerHand.THREE_OF_A_KIND, [
        card(Rank.SEVEN, Suit.HEARTS),
        card(Rank.SEVEN, Suit.CLUBS),
        card(Rank.SEVEN, Suit.DIAMONDS),
        card(Rank.KING, Suit.SPADES),
        card(Rank.THREE, Suit.HEARTS),
    ])


def test_0012_three_of_a_kind_with_duplicate_card_instances():
    assert_hand(PokerHand.THREE_OF_A_KIND, [
        card(Rank.FIVE, Suit.HEARTS),
        card(Rank.FIVE, Suit.HEARTS),
        card(Rank.FIVE, Suit.CLUBS),
        card(Rank.JACK, Suit.DIAMONDS),
        card(Rank.TWO, Suit.SPADES),
    ])


# Four of a kind


def test_0013_four_of_a_kind_regular():
    assert_hand(PokerHand.FOUR_OF_A_KIND, [
        card(Rank.JACK, Suit.HEARTS),
        card(Rank.JACK, Suit.CLUBS),
        card(Rank.JACK, Suit.DIAMONDS),
        card(Rank.JACK, Suit.SPADES),
    ])


def test_0014_four_of_a_kind_with_high_kicker():
    assert_hand(PokerHand.FOUR_OF_A_KIND, [
        card(Rank.SIX, Suit.HEARTS),
        card(Rank.SIX, Suit.CLUBS),
        card(Rank.SIX, Suit.DIAMONDS),
        card(Rank.SIX, Suit.SPADES),
        card(Rank.ACE, Suit.HEARTS),
    ])


def test_0015_four_of_a_kind_with_duplicate_suits():
    assert_hand(PokerHand.FOUR_OF_A_KIND, [
        card(Rank.QUEEN, Suit.HEARTS),
        card(Rank.QUEEN, Suit.HEARTS),
        card(Rank.QUEEN, Suit.CLUBS),
        card(Rank.QUEEN, Suit.DIAMONDS),
        card(Rank.THREE, Suit.SPADES),
    ])


def test_0016_four_of_a_kind_with_wild_kicker():
    assert_hand(PokerHand.FOUR_OF_A_KIND, [
        card(Rank.FOUR, Suit.HEARTS),
        card(Rank.FOUR, Suit.CLUBS),
        card(Rank.FOUR, Suit.DIAMONDS),
        card(Rank.FOUR, Suit.SPADES),
        card(Rank.KING, Suit.HEARTS, Enhancement.WILD),
    ])


# Five of a kind (requires duplicate cards in this simplified deck model)


def test_0017_five_of_a_kind_regular_duplicate_rank():
    assert_hand(PokerHand.FIVE_OF_A_KIND, [
        card(Rank.EIGHT, Suit.HEARTS),
        card(Rank.EIGHT, Suit.CLUBS),
        card(Rank.EIGHT, Suit.DIAMONDS),
        card(Rank.EIGHT, Suit.SPADES),
        card(Rank.EIGHT, Suit.HEARTS),
    ])


def test_0018_five_of_a_kind_with_two_duplicate_suits():
    assert_hand(PokerHand.FIVE_OF_A_KIND, [
        card(Rank.ACE, Suit.HEARTS),
        card(Rank.ACE, Suit.HEARTS),
        card(Rank.ACE, Suit.CLUBS),
        card(Rank.ACE, Suit.CLUBS),
        card(Rank.ACE, Suit.DIAMONDS),
    ])


def test_0019_five_of_a_kind_with_enhanced_card():
    assert_hand(PokerHand.FIVE_OF_A_KIND, [
        card(Rank.THREE, Suit.HEARTS),
        card(Rank.THREE, Suit.CLUBS),
        card(Rank.THREE, Suit.DIAMONDS),
        card(Rank.THREE, Suit.SPADES),
        card(Rank.THREE, Suit.HEARTS, Enhancement.BONUS),
    ])


def test_0020_five_of_a_kind_is_not_flush_five_when_suits_differ():
    assert_hand(PokerHand.FIVE_OF_A_KIND, [
        card(Rank.KING, Suit.HEARTS),
        card(Rank.KING, Suit.CLUBS),
        card(Rank.KING, Suit.DIAMONDS),
        card(Rank.KING, Suit.SPADES),
        card(Rank.KING, Suit.CLUBS),
    ])


# Two pair


def test_0021_two_pair_regular_four_card_hand():
    assert_hand(PokerHand.TWO_PAIR, [
        card(Rank.ACE, Suit.HEARTS),
        card(Rank.ACE, Suit.CLUBS),
        card(Rank.KING, Suit.DIAMONDS),
        card(Rank.KING, Suit.SPADES),
    ])


def test_0022_two_pair_with_kicker():
    assert_hand(PokerHand.TWO_PAIR, [
        card(Rank.TEN, Suit.HEARTS),
        card(Rank.TEN, Suit.CLUBS),
        card(Rank.FOUR, Suit.DIAMONDS),
        card(Rank.FOUR, Suit.SPADES),
        card(Rank.ACE, Suit.HEARTS),
    ])


def test_0023_two_pair_with_duplicate_card_instances():
    assert_hand(PokerHand.TWO_PAIR, [
        card(Rank.NINE, Suit.HEARTS),
        card(Rank.NINE, Suit.HEARTS),
        card(Rank.SIX, Suit.CLUBS),
        card(Rank.SIX, Suit.CLUBS),
        card(Rank.TWO, Suit.DIAMONDS),
    ])


def test_0024_two_pair_with_wild_kicker_does_not_become_flush():
    assert_hand(PokerHand.TWO_PAIR, [
        card(Rank.QUEEN, Suit.HEARTS),
        card(Rank.QUEEN, Suit.CLUBS),
        card(Rank.SEVEN, Suit.DIAMONDS),
        card(Rank.SEVEN, Suit.SPADES),
        card(Rank.THREE, Suit.HEARTS, Enhancement.WILD),
    ])


# Straight


def test_0025_straight_regular_mixed_suits():
    assert_hand(PokerHand.STRAIGHT, [
        card(Rank.NINE, Suit.HEARTS),
        card(Rank.EIGHT, Suit.CLUBS),
        card(Rank.SEVEN, Suit.DIAMONDS),
        card(Rank.SIX, Suit.SPADES),
        card(Rank.FIVE, Suit.HEARTS),
    ])


def test_0026_straight_ace_low_wheel():
    assert_hand(PokerHand.STRAIGHT, [
        card(Rank.ACE, Suit.HEARTS),
        card(Rank.FIVE, Suit.CLUBS),
        card(Rank.FOUR, Suit.DIAMONDS),
        card(Rank.THREE, Suit.SPADES),
        card(Rank.TWO, Suit.HEARTS),
    ])


def test_0027_straight_broadway():
    assert_hand(PokerHand.STRAIGHT, [
        card(Rank.ACE, Suit.SPADES),
        card(Rank.KING, Suit.HEARTS),
        card(Rank.QUEEN, Suit.CLUBS),
        card(Rank.JACK, Suit.DIAMONDS),
        card(Rank.TEN, Suit.SPADES),
    ])


def test_0028_straight_with_wild_card_and_mixed_suits():
    assert_hand(PokerHand.STRAIGHT, [
        card(Rank.SEVEN, Suit.HEARTS),
        card(Rank.SIX, Suit.CLUBS),
        card(Rank.FIVE, Suit.DIAMONDS, Enhancement.WILD),
        card(Rank.FOUR, Suit.SPADES),
        card(Rank.THREE, Suit.CLUBS),
    ])


# Flush


def test_0029_flush_regular():
    assert_hand(PokerHand.FLUSH, [
        card(Rank.ACE, Suit.HEARTS),
        card(Rank.JACK, Suit.HEARTS),
        card(Rank.EIGHT, Suit.HEARTS),
        card(Rank.FIVE, Suit.HEARTS),
        card(Rank.TWO, Suit.HEARTS),
    ])


def test_0030_flush_with_one_wild_card():
    assert_hand(PokerHand.FLUSH, [
        card(Rank.KING, Suit.CLUBS),
        card(Rank.TEN, Suit.CLUBS),
        card(Rank.SEVEN, Suit.DIAMONDS, Enhancement.WILD),
        card(Rank.FOUR, Suit.CLUBS),
        card(Rank.TWO, Suit.CLUBS),
    ])


def test_0031_flush_containing_one_pair():
    assert_hand(PokerHand.FLUSH, [
        card(Rank.QUEEN, Suit.SPADES),
        card(Rank.QUEEN, Suit.SPADES),
        card(Rank.NINE, Suit.SPADES),
        card(Rank.FIVE, Suit.SPADES),
        card(Rank.TWO, Suit.SPADES),
    ])


def test_0032_flush_with_multiple_wild_cards():
    assert_hand(PokerHand.FLUSH, [
        card(Rank.ACE, Suit.DIAMONDS),
        card(Rank.JACK, Suit.HEARTS, Enhancement.WILD),
        card(Rank.EIGHT, Suit.CLUBS, Enhancement.WILD),
        card(Rank.FIVE, Suit.DIAMONDS),
        card(Rank.TWO, Suit.DIAMONDS),
    ])


# Full house


def test_0033_full_house_regular():
    assert_hand(PokerHand.FULL_HOUSE, [
        card(Rank.KING, Suit.HEARTS),
        card(Rank.KING, Suit.CLUBS),
        card(Rank.KING, Suit.DIAMONDS),
        card(Rank.FOUR, Suit.HEARTS),
        card(Rank.FOUR, Suit.SPADES),
    ])


def test_0034_full_house_pair_listed_before_triplet():
    assert_hand(PokerHand.FULL_HOUSE, [
        card(Rank.ACE, Suit.HEARTS),
        card(Rank.ACE, Suit.CLUBS),
        card(Rank.SEVEN, Suit.DIAMONDS),
        card(Rank.SEVEN, Suit.SPADES),
        card(Rank.SEVEN, Suit.HEARTS),
    ])


def test_0035_full_house_with_duplicate_card_instances():
    assert_hand(PokerHand.FULL_HOUSE, [
        card(Rank.NINE, Suit.HEARTS),
        card(Rank.NINE, Suit.HEARTS),
        card(Rank.NINE, Suit.CLUBS),
        card(Rank.THREE, Suit.DIAMONDS),
        card(Rank.THREE, Suit.DIAMONDS),
    ])


def test_0036_full_house_with_wild_card_but_mixed_effective_suits():
    assert_hand(PokerHand.FULL_HOUSE, [
        card(Rank.JACK, Suit.HEARTS),
        card(Rank.JACK, Suit.CLUBS),
        card(Rank.JACK, Suit.DIAMONDS, Enhancement.WILD),
        card(Rank.SIX, Suit.SPADES),
        card(Rank.SIX, Suit.CLUBS),
    ])


# Straight flush


def test_0037_straight_flush_regular():
    assert_hand(PokerHand.STRAIGHT_FLUSH, [
        card(Rank.NINE, Suit.HEARTS),
        card(Rank.EIGHT, Suit.HEARTS),
        card(Rank.SEVEN, Suit.HEARTS),
        card(Rank.SIX, Suit.HEARTS),
        card(Rank.FIVE, Suit.HEARTS),
    ])


def test_0038_straight_flush_ace_low_wheel():
    assert_hand(PokerHand.STRAIGHT_FLUSH, [
        card(Rank.ACE, Suit.CLUBS),
        card(Rank.FIVE, Suit.CLUBS),
        card(Rank.FOUR, Suit.CLUBS),
        card(Rank.THREE, Suit.CLUBS),
        card(Rank.TWO, Suit.CLUBS),
    ])


def test_0039_straight_flush_broadway():
    assert_hand(PokerHand.STRAIGHT_FLUSH, [
        card(Rank.ACE, Suit.SPADES),
        card(Rank.KING, Suit.SPADES),
        card(Rank.QUEEN, Suit.SPADES),
        card(Rank.JACK, Suit.SPADES),
        card(Rank.TEN, Suit.SPADES),
    ])


def test_0040_straight_flush_completed_by_wild_card():
    assert_hand(PokerHand.STRAIGHT_FLUSH, [
        card(Rank.EIGHT, Suit.DIAMONDS),
        card(Rank.SEVEN, Suit.DIAMONDS),
        card(Rank.SIX, Suit.CLUBS, Enhancement.WILD),
        card(Rank.FIVE, Suit.DIAMONDS),
        card(Rank.FOUR, Suit.DIAMONDS),
    ])


# Flush house (requires duplicate cards and/or wild cards)


def test_0041_flush_house_regular_duplicate_cards():
    assert_hand(PokerHand.FLUSH_HOUSE, [
        card(Rank.KING, Suit.HEARTS),
        card(Rank.KING, Suit.HEARTS),
        card(Rank.KING, Suit.HEARTS),
        card(Rank.FOUR, Suit.HEARTS),
        card(Rank.FOUR, Suit.HEARTS),
    ])


def test_0042_flush_house_completed_by_one_wild_card():
    assert_hand(PokerHand.FLUSH_HOUSE, [
        card(Rank.QUEEN, Suit.CLUBS),
        card(Rank.QUEEN, Suit.CLUBS),
        card(Rank.QUEEN, Suit.DIAMONDS, Enhancement.WILD),
        card(Rank.SIX, Suit.CLUBS),
        card(Rank.SIX, Suit.CLUBS),
    ])


def test_0043_flush_house_with_pair_first():
    assert_hand(PokerHand.FLUSH_HOUSE, [
        card(Rank.ACE, Suit.SPADES),
        card(Rank.ACE, Suit.SPADES),
        card(Rank.NINE, Suit.SPADES),
        card(Rank.NINE, Suit.SPADES),
        card(Rank.NINE, Suit.SPADES),
    ])


def test_0044_flush_house_with_two_wild_cards():
    assert_hand(PokerHand.FLUSH_HOUSE, [
        card(Rank.TEN, Suit.DIAMONDS),
        card(Rank.TEN, Suit.HEARTS, Enhancement.WILD),
        card(Rank.TEN, Suit.CLUBS, Enhancement.WILD),
        card(Rank.THREE, Suit.DIAMONDS),
        card(Rank.THREE, Suit.DIAMONDS),
    ])


# Flush five (requires duplicate cards and/or wild cards)


def test_0045_flush_five_regular_duplicate_cards():
    assert_hand(PokerHand.FLUSH_FIVE, [
        card(Rank.ACE, Suit.HEARTS),
        card(Rank.ACE, Suit.HEARTS),
        card(Rank.ACE, Suit.HEARTS),
        card(Rank.ACE, Suit.HEARTS),
        card(Rank.ACE, Suit.HEARTS),
    ])


def test_0046_flush_five_completed_by_wild_card():
    assert_hand(PokerHand.FLUSH_FIVE, [
        card(Rank.SEVEN, Suit.CLUBS),
        card(Rank.SEVEN, Suit.CLUBS),
        card(Rank.SEVEN, Suit.CLUBS),
        card(Rank.SEVEN, Suit.CLUBS),
        card(Rank.SEVEN, Suit.DIAMONDS, Enhancement.WILD),
    ])


def test_0047_flush_five_with_multiple_wild_cards():
    assert_hand(PokerHand.FLUSH_FIVE, [
        card(Rank.FIVE, Suit.SPADES),
        card(Rank.FIVE, Suit.SPADES),
        card(Rank.FIVE, Suit.HEARTS, Enhancement.WILD),
        card(Rank.FIVE, Suit.CLUBS, Enhancement.WILD),
        card(Rank.FIVE, Suit.DIAMONDS, Enhancement.WILD),
    ])


def test_0048_flush_five_with_enhanced_duplicate():
    assert_hand(PokerHand.FLUSH_FIVE, [
        card(Rank.TWO, Suit.DIAMONDS),
        card(Rank.TWO, Suit.DIAMONDS),
        card(Rank.TWO, Suit.DIAMONDS),
        card(Rank.TWO, Suit.DIAMONDS),
        card(Rank.TWO, Suit.DIAMONDS, Enhancement.BONUS),
    ])


# Near-miss regressions


def test_0049_failed_flush_with_only_four_suited_cards_is_high_card():
    assert_hand(PokerHand.HIGH_CARD, [
        card(Rank.ACE, Suit.HEARTS),
        card(Rank.JACK, Suit.HEARTS),
        card(Rank.EIGHT, Suit.HEARTS),
        card(Rank.FIVE, Suit.HEARTS),
    ])


def test_0050_failed_straight_with_only_four_cards_is_high_card():
    assert_hand(PokerHand.HIGH_CARD, [
        card(Rank.NINE, Suit.HEARTS),
        card(Rank.EIGHT, Suit.CLUBS),
        card(Rank.SEVEN, Suit.DIAMONDS),
        card(Rank.SIX, Suit.SPADES),
    ])


def test_0051_failed_straight_with_rank_gap_is_high_card():
    assert_hand(PokerHand.HIGH_CARD, [
        card(Rank.NINE, Suit.HEARTS),
        card(Rank.EIGHT, Suit.CLUBS),
        card(Rank.SIX, Suit.DIAMONDS),
        card(Rank.FIVE, Suit.SPADES),
        card(Rank.FOUR, Suit.HEARTS),
    ])


def test_0052_failed_two_pair_with_only_one_pair_is_pair():
    assert_hand(PokerHand.PAIR, [
        card(Rank.KING, Suit.HEARTS),
        card(Rank.KING, Suit.CLUBS),
        card(Rank.NINE, Suit.DIAMONDS),
        card(Rank.FIVE, Suit.SPADES),
        card(Rank.TWO, Suit.HEARTS),
    ])


def test_0053_failed_full_house_with_triplet_only_is_three_of_a_kind():
    assert_hand(PokerHand.THREE_OF_A_KIND, [
        card(Rank.QUEEN, Suit.HEARTS),
        card(Rank.QUEEN, Suit.CLUBS),
        card(Rank.QUEEN, Suit.DIAMONDS),
        card(Rank.SEVEN, Suit.SPADES),
        card(Rank.THREE, Suit.HEARTS),
    ])


def test_0054_failed_straight_flush_with_mixed_suits_is_straight():
    assert_hand(PokerHand.STRAIGHT, [
        card(Rank.SIX, Suit.HEARTS),
        card(Rank.FIVE, Suit.HEARTS),
        card(Rank.FOUR, Suit.CLUBS),
        card(Rank.THREE, Suit.HEARTS),
        card(Rank.TWO, Suit.HEARTS),
    ])
