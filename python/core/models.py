import random
from dataclasses import dataclass, field

from PIL import Image

from config.settings import JOKER_CONFIG
from core.class_indices import JOKER_TYPE_CLASSES
from core.enums import Edition, Enhancement, JokersName, JokerTriggers, Rank, Seal, Suit
from core.hand_stats import HandStats
from core.scoring import get_initial_card_chips
from utils.files import load_json

CONFIG = load_json(JOKER_CONFIG)

CARD_STRINGS = ["2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K", "A"]

BACKGROUND_JOKERS = {
    JokersName.CANIO_BACKGROUND,
    JokersName.CHICOT_BACKGROUND,
    JokersName.PERKEO_BACKGROUND,
    JokersName.YORICK_BACKGROUND,
    JokersName.HOLOGRAM_BACKGROUND,
    JokersName.TRIBOULET_BACKGROUND,
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
    trigger: int = 1
    play_x_mult: float = 1
    hand_x_mult: float = 1
    card_id: int = 0
    econ: int = 0
    card_score: int = 0
    # For cards like steel or gold, they need to be held in hand to activate
    in_hand: bool = False
    is_facecard: bool = False
    is_low_card: bool = False
    is_any_suit: bool = False


    @classmethod
    def build_dummy(cls) -> Card:
        return Card(Rank.ACE, Suit.HEARTS, Enhancement.NONE, Seal.NONE, Edition.NONE)


    def __post_init__(self):
        self.is_facecard = self.rank > Rank.TEN and self.rank < Rank.ACE
        self.is_low_card = self.rank < Rank.SIX
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
                self.in_hand = True
                self.econ += 3

            case Enhancement.BONUS:
                self.chips += 30

            case Enhancement.MULT:
                self.add_mult += 4

            case Enhancement.WILD:
                self.is_any_suit = True

            case Enhancement.LUCKY:
                return

            case Enhancement.GLASS:
                self.play_x_mult = 2

            case Enhancement.STEEL:
                self.hand_x_mult = 1.5
                self.in_hand = True

    def add_edition(self) -> None:
        match self.edition:
            case Edition.FOIL:
                self.chips += 50

            case Edition.HOLOGRAPHIC:
                self.add_mult += 10

            case Edition.POLYCHROME:
                self.play_x_mult *= 1.5

    def add_seal(self) -> None:
        if self.seal == Seal.RED:
            self.trigger += 1

    def score(self) -> int:
        val = self.rank & 0b1111
        val = (val << 2) | (self.suit & 0b11)
        val = (val << 4) | (self.enhancement & 0b1111)
        val = (val << 3) | (self.seal & 0b111)
        val = (val << 2) | (self.edition & 0b11)
        return val

    @classmethod
    def random(cls):
        return cls(
            rank=random.choice(list(Rank)),
            suit=random.choice(list(Suit)),
            enhancement=random.choice(list(Enhancement)),
            seal=random.choice(list(Seal)),
            edition=random.choice(list(Edition)),
        )

    def __repr__(self):
        base = f"{CARD_STRINGS[self.rank]} of {self.suit.name}"

        base = (
            f"{self.enhancement.name} {base}"
            if self.enhancement != Enhancement.NONE
            else f"Normal {base}"
        )

        if self.edition != Edition.NONE:
            base = f"{self.edition.name} {base}"

        if self.seal != Seal.NONE:
            base = f"{self.seal.name} seal {base}"

        return base

    def __hash__(self):
        return self.card_id


@dataclass
class HandScoring:
    hand_stats: HandStats
    scored_played: list[Card]
    unscored_played: list[Card]
    scored_held: list[Card]
    unscored_held: list[Card]


@dataclass
class JokerScoringConditions:
    card: Card = Card.build_dummy()
    card_index: int = -1
    hands_left: int = -1
    scoring_held: list[Card] = field(default_factory=list)
    unscoring_held: list[Card] = field(default_factory=list)
    scoring_played: list[Card] = field(default_factory=list)



@dataclass
class JokerScoring:
    trigger: JokerTriggers
    chips: int | dict | None = None
    add_mult: int | dict | None = None
    x_mult: float | dict | None = None
    condition: dict | None = None


@dataclass
class JokerRetrigger:
    trigger: JokerTriggers
    times: int
    condition: str | None = None


@dataclass
class JokerGameModifier:
    discards: int = 0
    hand_size: int = 0
    hands: int = 0
    all_cards_are_facecards: bool = False
    suit_groups: list[list[str]] = field(default_factory=list)
    straight_size: int | None = None
    flush_size: int | None = None
    straight_gap_allowed: int = 0
    all_played_cards_score: bool = False
    double_probabilities: bool = False
    allow_duplicate_shop_items: bool = False
    disable_boss_blind: bool = False


@dataclass
class JokerEcon:
    money: int | dict
    condition: dict | str | None = None
    when: str | None = None


@dataclass
class JokerUpgrade:
    target: str
    when: str
    condition: dict | str | None = None


@dataclass
class JokerConfig:
    rarity: str
    buy_price: int
    copyable: bool
    life: int | None = None


@dataclass
class CardBucket:
    count: int
    cards: list[Card]


@dataclass
class Joker:
    background_image: JokersName
    joker_id = -1
    face_image: JokersName | None = None
    negative: bool = False
    edition: Edition = Edition.NONE
    config: JokerConfig | None = None
    scoring: JokerScoring | None = None
    retrigger: JokerRetrigger | None = None
    econ: JokerEcon | None = None
    upgrade: JokerUpgrade | None = None
    game_modifier: JokerGameModifier | None = None
    copy: bool | None = None
    req: dict[str, int] | None = None

    def __post_init__(self):
        self._add_face()

        joker_key = (
            self.background_image.name.lower()
            if "BACKGROUND" not in self.background_image.name
            else self.background_image.name.split("_BACKGROUND")[0].lower()
        )
        joker_data = CONFIG[joker_key]
        self.copy = joker_data["copy"] if "copy" in joker_data else None

        self._build_joker_config(joker_data)
        self._build_joker_scoring(joker_data)
        self._build_joker_retrigger(joker_data)
        self._build_joker_econ(joker_data)
        self._build_joker_upgrade(joker_data)
        self._build_joker_game_modifier(joker_data)

        if joker_key in {JokersName.ANCIENT_JOKER, JokersName.THE_IDOL}:
            # will need to use vision functions to figure that out 0_0
            pass

    def _build_joker_config(self, joker_data: dict) -> None:
        self.config = JokerConfig(
            rarity=joker_data["rarity"],
            buy_price=joker_data["buy_price"],
            copyable=joker_data["copyable"],
            life=joker_data.get("life"),
        )

    def _build_joker_scoring(self, joker_data: dict) -> None:
        if "scoring" not in joker_data:
            return

        scoring_data = joker_data["scoring"]
        self.scoring = JokerScoring(
            chips=scoring_data.get("chips", None),
            add_mult=scoring_data.get("add_mult", None),
            x_mult=scoring_data.get("x_mult", None),
            condition=scoring_data.get("condition", None),
            trigger=JokerTriggers(scoring_data.get("trigger")),
        )

    def _build_joker_retrigger(self, joker_data: dict) -> None:
        if "retrigger" not in joker_data:
            return

        retrigger_data = joker_data["retrigger"]
        self.retrigger = JokerRetrigger(
            trigger=retrigger_data.get("trigger"),
            times=retrigger_data.get("times", 1),
            condition=retrigger_data.get("condition", None),
        )

    def _build_joker_econ(self, joker_data: dict) -> None:
        if "econ" not in joker_data:
            return

        econ_data = joker_data["econ"]
        self.econ = JokerEcon(
            money=econ_data["money"],
            condition=econ_data.get("condition"),
            when=econ_data.get("when"),
        )

    def _build_joker_upgrade(self, joker_data: dict) -> None:
        if "upgrade" not in joker_data:
            return

        upgrade_data = joker_data["upgrade"]
        self.upgrade = JokerUpgrade(
            target=upgrade_data["target"],
            when=upgrade_data["when"],
            condition=upgrade_data.get("condition"),
        )

    def _build_joker_game_modifier(self, joker_data: dict) -> None:
        if "game_modifier" not in joker_data:
            return

        game_modifier_data = joker_data["game_modifier"]
        self.game_modifier = JokerGameModifier(
            hands=game_modifier_data.get("hands", 0),
            discards=game_modifier_data.get("discards", 0),
            hand_size=game_modifier_data.get("hand_size", 0),
            all_cards_are_facecards=game_modifier_data.get(
                "all_cards_are_facecards", False
            ),
            suit_groups=game_modifier_data.get("suit_groups", []),
            straight_size=game_modifier_data.get("straight_size"),
            flush_size=game_modifier_data.get("flush_size"),
            straight_gap_allowed=game_modifier_data.get("straight_gap_allowed", 0),
            all_played_cards_score=game_modifier_data.get(
                "all_played_cards_score", False
            ),
            double_probabilities=game_modifier_data.get("double_probabilities", False),
            allow_duplicate_shop_items=game_modifier_data.get(
                "allow_duplicate_shop_items", False
            ),
            disable_boss_blind=game_modifier_data.get("disable_boss_blind", False),
        )

    def _add_face(self):
        if self.background_image in BACKGROUND_JOKERS:
            self.face_image = JokersName(int(self.background_image) + 10)

    @classmethod
    def random(cls):
        return cls(
            background_image=random.choice(RANDOM_JOKERS),
            negative=random.choice([True, False]),
            edition=random.choice(list(Edition)),
        )

    def __repr__(self):
        base = self.background_image.name.lower()
        if self.negative:
            base = f"negative {base}"
        elif self.edition != Edition.NONE:
            base = f"{self.edition.name.lower()} {base}"

        return base


@dataclass
class JokerPlan:
    on_played: list[Joker]
    on_held: list[Joker]
    after_hand: list[Joker]
    played_retrigger: list[Joker]
    held_retrigger: list[Joker]


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
        if self.has_joker(jokers, JokersName.ASTRONOMER) and item_type in {
            "planet_card",
            "celestial_pack",
        }:
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
