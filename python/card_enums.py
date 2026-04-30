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



class PokerHand(IntEnum):
    HIGH_CARD = 1
    PAIR = 2
    TWO_PAIR = 6
    THREE_OF_A_KIND = 3
    STRAIGHT = 7
    FLUSH = 8
    FULL_HOUSE = 9
    FOUR_OF_A_KIND = 4
    STRAIGHT_FLUSH = 10
    FIVE_OF_A_KIND = 5
    FLUSH_HOUSE = 11
    FLUSH_FIVE = 12
