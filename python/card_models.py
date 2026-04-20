import random

from PIL import Image
from dataclasses import dataclass

from card_enums import Rank, Suit, Enhancement, Seal

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
