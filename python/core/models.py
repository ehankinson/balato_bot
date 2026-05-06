import random

from PIL import Image
from dataclasses import dataclass, field
from typing import Any

from core.enums import Edition, Rank, Suit, Enhancement, Seal, JokersName
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
    JokersName.CANIO_BACKGROUND, JokersName.CHICOT_BACKGROUND, JokersName.PERKEO_BACKGROUND, JokersName.YORICK_BACKGROUND, JokersName.HOLOGRAM_BACKGROUND, JokersName.TRIBOULET_BACKGROUND
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


@dataclass(frozen=True)
class JokerScoring:
    chips: int = 0
    add_mult: int | float = 0
    x_mult: float = 1
    card_chips: int = 0
    chips_from_state: str | None = None
    add_mult_from_state: str | None = None
    x_mult_from_state: str | None = None
    chips_from: dict[str, Any] | None = None
    add_mult_from: dict[str, Any] | None = None
    x_mult_from: dict[str, Any] | None = None

    @classmethod
    def from_dict(cls, config: dict[str, Any] | None) -> "JokerScoring":
        if not config:
            return cls()

        return cls(
            chips=config.get("chips", 0),
            add_mult=config.get("add_mult", 0),
            x_mult=config.get("x_mult", 1),
            card_chips=config.get("card_chips", 0),
            chips_from_state=config.get("chips_from_state"),
            add_mult_from_state=config.get("add_mult_from_state"),
            x_mult_from_state=config.get("x_mult_from_state"),
            chips_from=config.get("chips_from"),
            add_mult_from=config.get("add_mult_from"),
            x_mult_from=config.get("x_mult_from"),
        )


@dataclass(frozen=True)
class JokerScaling:
    event: str
    stat: str | None = None
    change: int | float = 0
    condition: dict[str, Any] = field(default_factory=dict)
    scope: str = "while_owned"
    counter: str | None = None
    threshold: int | float | None = None
    change_from: dict[str, Any] | None = None
    after: dict[str, Any] = field(default_factory=dict)

    @classmethod
    def from_dict(cls, config: dict[str, Any]) -> "JokerScaling":
        return cls(
            event=config["event"],
            stat=config.get("stat"),
            change=config.get("change", 0),
            condition=config.get("condition", {}),
            scope=config.get("scope", "while_owned"),
            counter=config.get("counter"),
            threshold=config.get("threshold"),
            change_from=config.get("change_from"),
            after=config.get("after", {}),
        )


@dataclass(frozen=True)
class JokerReset:
    event: str
    state: dict[str, Any]
    condition: dict[str, Any] = field(default_factory=dict)

    @classmethod
    def from_dict(cls, config: dict[str, Any]) -> "JokerReset":
        return cls(
            event=config["event"],
            state=config.get("state", {}),
            condition=config.get("condition", {}),
        )


@dataclass(frozen=True)
class JokerConfig:
    rarity: str
    buy_price: int
    sell_price: int
    activation: str | None = None
    copyable: bool = False
    scoring: JokerScoring = field(default_factory=JokerScoring)
    scaling: list[JokerScaling] = field(default_factory=list)
    reset: list[JokerReset] = field(default_factory=list)
    state: dict[str, Any] = field(default_factory=dict)
    condition: dict[str, Any] = field(default_factory=dict)
    shop_effects: list[str] = field(default_factory=list)
    econ: int = 0
    econ_from: dict[str, Any] | None = None
    hands: int = 0
    discards: int = 0
    hand_size: int = 0
    hand_size_from_state: str | None = None
    retrigger: bool = False
    generate: str | None = None
    upgrade: str | None = None
    copy: bool = False
    sell_effect: dict[str, Any] | None = None
    dynamic_target: dict[str, Any] | None = None

    @classmethod
    def from_dict(cls, config: dict[str, Any]) -> "JokerConfig":
        return cls(
            rarity=config["rarity"],
            buy_price=config["buy_price"],
            sell_price=config["sell_price"],
            activation=config.get("activation"),
            copyable=config.get("copyable", False),
            scoring=JokerScoring.from_dict(config.get("scoring")),
            scaling=[
                JokerScaling.from_dict(scaling)
                for scaling in config.get("scaling", [])
            ],
            reset=[
                JokerReset.from_dict(reset)
                for reset in config.get("reset", [])
            ],
            state=config.get("state", {}),
            condition=config.get("condition", {}),
            shop_effects=config.get("shop_effects", []),
            econ=config.get("econ", 0),
            econ_from=config.get("econ_from"),
            hands=config.get("hands", 0),
            discards=config.get("discards", 0),
            hand_size=config.get("hand_size", 0),
            hand_size_from_state=config.get("hand_size_from_state"),
            retrigger=config.get("retrigger", False),
            generate=config.get("generate"),
            upgrade=config.get("upgrade"),
            copy=config.get("copy", False),
            sell_effect=config.get("sell_effect"),
            dynamic_target=config.get("dynamic_target"),
        )


@dataclass
class Joker:
    background_image: JokersName
    face_image: JokersName | None = None
    negative: bool = False
    edition: Edition = Edition.NONE
    config: JokerConfig | None = None
    state: dict[str, Any] = field(default_factory=dict)


    def __post_init__(self):
        self._add_face()
        if self.config is not None:
            self.state.update(self.config.state)


    def _add_face(self):
        if self.background_image in BACKGROUND_JOKERS:
            self.face_image = JokersName(int(self.background_image) + 10)

    @property
    def key(self) -> str:
        return self.background_image.name.lower()

    @property
    def scoring(self) -> JokerScoring:
        if self.config is None:
            return JokerScoring()

        return self.config.scoring

    @property
    def scaling(self) -> list[JokerScaling]:
        if self.config is None:
            return []

        return self.config.scaling

    @property
    def buy_price(self) -> int | None:
        return None if self.config is None else self.config.buy_price

    @property
    def sell_price(self) -> int | None:
        return None if self.config is None else self.config.sell_price

    @property
    def copyable(self) -> bool:
        return False if self.config is None else self.config.copyable

    @classmethod
    def from_config(
        cls,
        background_image: JokersName,
        joker_configs: dict[str, dict[str, Any]],
        negative: bool = False,
        edition: Edition = Edition.NONE,
    ) -> "Joker":
        key = background_image.name.lower()
        return cls(
            background_image=background_image,
            negative=negative,
            edition=edition,
            config=JokerConfig.from_dict(joker_configs[key]),
        )

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
class ShopState:
    rerolls_used: int = 0
    chaos_free_reroll_used: bool = False

    def has_joker(self, jokers: list[Joker], joker_name: JokersName) -> bool:
        return any(joker.background_image == joker_name for joker in jokers)

    def has_free_chaos_reroll(self, jokers: list[Joker]) -> bool:
        return (
            self.has_joker(jokers, JokersName.CHAOS_THE_CLOWN)
            and not self.chaos_free_reroll_used
        )

    def reroll_cost(self, jokers: list[Joker], base_cost: int) -> int:
        if self.has_free_chaos_reroll(jokers):
            return 0

        return base_cost

    def use_reroll(self, jokers: list[Joker], base_cost: int) -> int:
        cost = self.reroll_cost(jokers, base_cost)
        if cost == 0 and self.has_free_chaos_reroll(jokers):
            self.chaos_free_reroll_used = True

        self.rerolls_used += 1
        return cost

    def item_cost(self, jokers: list[Joker], item_type: str, base_cost: int) -> int:
        if (
            self.has_joker(jokers, JokersName.ASTRONOMER)
            and item_type in {"planet_card", "celestial_pack"}
        ):
            return 0

        return base_cost

    def minimum_money(self, jokers: list[Joker]) -> int:
        if self.has_joker(jokers, JokersName.CREDIT_CARD):
            return -20

        return 0

    def can_afford(self, jokers: list[Joker], money: int, cost: int) -> bool:
        return money - cost >= self.minimum_money(jokers)

@dataclass
class CardAnnotation:
    card: Card | Joker
    box: list[float]



@dataclass
class RenderedHand:
    image: Image.Image
    annotations: list[CardAnnotation]
