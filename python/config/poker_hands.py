from core.enums import PokerHand
from core.hand_stats import HandStats

HAND_STATS: dict[PokerHand, HandStats] = {
    PokerHand.FLUSH_FIVE: HandStats(PokerHand.FLUSH_FIVE, 160, 16),
    PokerHand.FLUSH_HOUSE: HandStats(PokerHand.FLUSH_HOUSE, 140, 14),
    PokerHand.FIVE_OF_A_KIND: HandStats(PokerHand.FIVE_OF_A_KIND, 120, 12),
    PokerHand.STRAIGHT_FLUSH: HandStats(PokerHand.STRAIGHT_FLUSH, 100, 8),
    PokerHand.FOUR_OF_A_KIND: HandStats(PokerHand.FOUR_OF_A_KIND, 60, 7),
    PokerHand.FULL_HOUSE: HandStats(PokerHand.FULL_HOUSE, 40, 4),
    PokerHand.FLUSH: HandStats(PokerHand.FLUSH, 35, 4),
    PokerHand.STRAIGHT: HandStats(PokerHand.STRAIGHT, 30, 4),
    PokerHand.THREE_OF_A_KIND: HandStats(PokerHand.THREE_OF_A_KIND, 30, 3),
    PokerHand.TWO_PAIR: HandStats(PokerHand.TWO_PAIR, 20, 2),
    PokerHand.PAIR: HandStats(PokerHand.PAIR, 10, 2),
    PokerHand.HIGH_CARD: HandStats(PokerHand.HIGH_CARD, 5, 1),
}
