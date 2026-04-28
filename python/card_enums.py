from enum import IntEnum

class Rank(IntEnum):
    TWO = 2
    THREE = 3
    FOUR = 4
    FIVE = 5
    SIX = 6
    SEVEN = 7
    EIGHT = 8
    NINE = 9
    TEN = 10
    JACK = 11
    QUEEN = 12
    KING = 13
    ACE = 1



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
    Rank = 0
    Suit = 1
    Enhancement = 2
    Seal = 3
