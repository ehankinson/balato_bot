from enum import Enum, IntEnum

class Rank(IntEnum):
    TWO = 0
    THREE = 1
    FOUR = 2
    FIVE = 3
    SIX = 4
    SEVEN = 5
    EIGHT = 6
    NINE = 7
    TEN = 8
    JACK = 9
    QUEEN = 10
    KING = 11
    ACE = 12



class Suit(IntEnum):
    HEARTS = 0
    DIAMONDS = 1
    CLUBS = 2
    SPADES = 3



class Enhancement(IntEnum):
    NONE = 0
    STONE = 1
    GOLD = 2
    BONUS = 3
    MULT = 4
    WILD = 5
    LUCKY = 6
    GLASS = 7
    STEEL = 8



class Seal(IntEnum):
    NONE = 0
    GOLD = 1
    PURPLE = 2
    RED = 3
    BLUE = 4



class CardFeatureTrainingType(IntEnum):
    RANK = 0
    SUIT = 1
    ENHANCEMENT = 2
    SEAL = 3



class PokerHand(Enum):
    HIGH_CARD = "High Card"
    PAIR = "Pair"
    TWO_PAIR = "Two Pair"
    THREE_OF_A_KIND = "Three of a Kind"
    STRAIGHT = "Straight"
    FLUSH = "Flush"
    FULL_HOUSE = "Full House"
    FOUR_OF_A_KIND = "Four of a Kind"
    STRAIGHT_FLUSH = "Straight Flush"
    FIVE_OF_A_KIND = "Five of a Kind"
    FLUSH_HOUSE = "Flush House"
    FLUSH_FIVE = "Flush Five"
