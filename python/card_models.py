import random

from PIL import Image
from dataclasses import dataclass

from card_enums import Rank, Suit, Enhancement, Seal
from util import get_initial_card_chips, calculate_lucky

CARD_STRINGS = [
    "2",
    "3",
    "4",
    "5",
    "6",
    "7",
    "8",
    "9",
    "10",
    "J",
    "Q",
    "K",
    "A"
]

@dataclass
class Card:
    rank: Rank
    suit: Suit
    enhancement: Enhancement
    seal: Seal
    chips: int = 0
    add_mult: int = 0
    times_mult: float = 1
    score: int = 0
    econ: int = 0
    # For cards like steel or gold, they need to be held in hand to activate
    in_hand: bool = False

    def __post_init__(self):
        self.chips = get_initial_card_chips(self.rank)
        self.add_enhancement()



    def add_enhancement(self) -> None:
        match self.enhancement:
            case Enhancement.NONE:
                return

            case Enhancement.STONE:
                self.chips = 50

            case Enhancement.GOLD:
                # if Joker.GOLDEN_TICKET in jokers:
                #     self.econ = 4
                # Needs to wait for Jokers to be added

                self.in_hand = True
                self.econ += 3

            case Enhancement.BONUS:
                self.chips += 30

            case Enhancement.MULT:
                self.add_mult += 4

            case Enhancement.WILD:
                return

            case Enhancement.LUCKY:
                mult_gain, econ_gain = calculate_lucky()
                self.add_mult += mult_gain
                self.econ += econ_gain

            case Enhancement.GLASS:
                self.times_mult = 2

            case Enhancement.STEEL:
                self.times_mult = 1.5
                self.in_hand = True

        if self.seal == Seal.RED:
            self.chips *= 2
            self.add_mult *= 2
            self.times_mult *= self.times_mult



    @classmethod
    def random(cls):
        return cls(
            rank=random.choice(list(Rank)),
            suit=random.choice(list(Suit)),
            enhancement=random.choice(list(Enhancement)),
            seal=random.choice(list(Seal)),
        )



    def __repr__(self):
        base = f"{CARD_STRINGS[self.rank]} of {self.suit.name}"

        base = f"{self.enhancement.name} {base}" \
            if self.enhancement != Enhancement.NONE\
                else f"Normal {base}"

        if self.seal != Seal.NONE:
            base = f"{self.seal.name} seal {base}"

        return base



@dataclass
class Hand:
    cards: list[Card]

    @classmethod
    def random_hand(cls, card_amount: int):
        return cls(cards=[
            Card.random() for _ in range(card_amount)
        ])



    def sort_by_rank(self):
        self.cards = sorted(self.cards, key=lambda x: (x.rank, x.suit), reverse=True)



    def sort_by_suit(self):
        self.cards = sorted(self.cards, key=lambda x: (x.suit, x.rank), reverse=True)



@dataclass
class CardAnnotation:
    card: Card
    box: list[float]



@dataclass
class RenderedHand:
    image: Image.Image
    annotations: list[CardAnnotation]



@dataclass(frozen=True)
class HandStats:
    chips: int
    mult: int
    level: int = 1
