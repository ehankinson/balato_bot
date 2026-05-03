import random

from PIL import Image
from dataclasses import dataclass

from core.enums import Edition, Rank, Suit, Enhancement, Seal, Jokers
from core.class_indices import JOKER_TYPE_CLASSES
from core.hand_stats import HandStats
from core.scoring import get_initial_card_chips, calculate_lucky

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

BACKGROUND_JOKERS = {
    Jokers.CANIO_BACKGROUND, Jokers.CHICOT_BACKGROUND, Jokers.PERKEO_BACKGROUND, Jokers.YORICK_BACKGROUND, Jokers.HOLOGRAM_BACKGROUND, Jokers.TRIBOULET_BACKGROUND
}

RANDOM_JOKERS = list(JOKER_TYPE_CLASSES)

@dataclass
class Card:
    rank: Rank
    suit: Suit
    enhancement: Enhancement
    seal: Seal
    edition: Edition
    chips: int = 0
    add_mult: int = 0
    play_times_mult: float = 1
    hand_times_mult: float = 1
    card_id: int = 0
    econ: int = 0
    card_score: int = 0
    # For cards like steel or gold, they need to be held in hand to activate
    in_hand: bool = False

    def __post_init__(self):
        self.chips = get_initial_card_chips(self.rank)
        self.add_enhancement()
        self.add_edition()
        self.add_seal()
        self.card_id = self.score()



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
                self.play_times_mult = 2

            case Enhancement.STEEL:
                self.hand_times_mult = 1.5
                self.in_hand = True


    def add_edition(self) -> None:
        match self.edition:
            case Edition.FOIL:
                self.chips += 50

            case Edition.HOLOGRAPHIC:
                self.add_mult += 10

            case Edition.POLYCHROME:
                self.play_times_mult *= 1.5

    
    def add_seal(self) -> None:
        if self.seal == Seal.RED:
            self.chips *= 2
            self.add_mult *= 2
            self.play_times_mult *= self.play_times_mult
            self.hand_times_mult *= self.hand_times_mult


    def score(self) -> int:
        val = self.rank & 0b1111
        val = (val << 2) | (self.suit & 0b11)
        val = (val << 4) | (self.enhancement & 0b1111)
        val = (val << 3) | (self.suit & 0b111)
        val = (val << 2) | (self.edition & 0b11)
        return val



    @classmethod
    def random(cls):
        return cls(
            rank=random.choice(list(Rank)),
            suit=random.choice(list(Suit)),
            enhancement=random.choice(list(Enhancement)),
            seal=random.choice(list(Seal)),
            edition=random.choice(list(Edition))
        )



    def __repr__(self):
        base = f"{CARD_STRINGS[self.rank]} of {self.suit.name}"

        base = f"{self.enhancement.name} {base}" \
            if self.enhancement != Enhancement.NONE\
                else f"Normal {base}"

        if self.edition != Edition.NONE:
            base = f"{self.edition.name} {base}"

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
class Joker:
    background_image: Jokers
    face_image: Jokers | None = None
    negative: bool = False
    edition: Edition = Edition.NONE


    def __post_init__(self):
        self._add_face()


    def _add_face(self):
        if self.background_image in BACKGROUND_JOKERS:
            self.face_image = Jokers(int(self.background_image) + 10)

    @classmethod
    def random(cls):
        return cls(
            background_image=random.choice(RANDOM_JOKERS),
            negative=random.choice([True, False]),
            edition=random.choice(list(Edition))
        )


    def __repr__(self):
        base = self.background_image.name.lower()
        if self.negative:
            base = f"negative {base}"
        elif self.edition != Edition.NONE:
            base = f"{self.edition.name.lower()} {base}"

        return base

@dataclass
class CardAnnotation:
    card: Card | Joker
    box: list[float]



@dataclass
class RenderedHand:
    image: Image.Image
    annotations: list[CardAnnotation]
