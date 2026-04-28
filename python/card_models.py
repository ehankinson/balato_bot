import random

from PIL import Image
from dataclasses import dataclass

from card_enums import Rank, Suit, Enhancement, Seal

CARD_STRINGS = [
    "A",
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
    "K"
]

@dataclass
class Card:
    rank: Rank
    suit: Suit
    enhancement: Enhancement
    seal: Seal

    @classmethod
    def random(cls):
        return cls(
            rank=random.choice(list(Rank)),
            suit=random.choice(list(Suit)),
            enhancement=random.choice(list(Enhancement)),
            seal=random.choice(list(Seal))
        )

    def __repr__(self):
        base = f"{CARD_STRINGS[self.rank - 1]} of {self.suit.name}"

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




@dataclass
class CardAnnotation:
    card: Card
    box: list[float]



@dataclass
class RenderedHand:
    image: Image
    annotations: list[CardAnnotation]
