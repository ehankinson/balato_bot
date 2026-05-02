from core.enums import PokerHand
from core.hand_stats import HandStats


HAND_STATS: dict[PokerHand, HandStats] = {
    PokerHand.FLUSH_FIVE: HandStats(160, 16),
    PokerHand.FLUSH_HOUSE: HandStats(140, 14),
    PokerHand.FIVE_OF_A_KIND: HandStats(120, 12),
    PokerHand.STRAIGHT_FLUSH: HandStats(100, 8),
    PokerHand.FOUR_OF_A_KIND: HandStats(60, 7),
    PokerHand.FULL_HOUSE: HandStats(40, 4),
    PokerHand.FLUSH: HandStats(35, 4),
    PokerHand.STRAIGHT: HandStats(30, 4),
    PokerHand.THREE_OF_A_KIND: HandStats(30, 3),
    PokerHand.TWO_PAIR: HandStats(20, 2),
    PokerHand.PAIR: HandStats(10, 2),
    PokerHand.HIGH_CARD: HandStats(5, 1),
}

