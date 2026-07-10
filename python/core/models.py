import random
from dataclasses import dataclass, field
from itertools import combinations

from PIL import Image
from sympy.functions.combinatorial.numbers import E

from config.settings import JOKER_CONFIG
from core.class_indices import JOKER_TYPE_CLASSES
from core.enums import Edition, Enhancement, JokersName, JokerTriggers, Rank, Seal, Suit
from core.hand_stats import HandStats
from core.scoring import (
    EDITION_SCORING,
    ENHANCEMENT_SCORING,
    SEAL_SCORING,
    get_initial_card_chips,
)
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


@dataclass(slots=True)
class Card:
    rank: Rank
    suit: Suit
    enhancement: Enhancement = Enhancement.NONE
    seal: Seal = Seal.NONE
    edition: Edition = Edition.NONE
    chips: int = 0
    add_mult: int = 0
    trigger: int = 1
    play_x_mult: float = 1
    hand_x_mult: float = 1
    card_id: int = 0
    econ: int = 0
    score: int = 1
    card_score: int = 0
    mult_prob: float = 1.0
    econ_prob: float = 1.0

    is_facecard: bool = False
    is_low_card: bool = False
    is_any_suit: bool = False

    def __post_init__(self):
        self.is_facecard = self.rank > Rank.TEN and self.rank < Rank.ACE
        self.is_low_card = self.rank < Rank.SIX
        self.chips = get_initial_card_chips(self.rank)
        self.add_enhancement()
        self.add_edition()
        self.add_seal()
        self.card_id = self.id()
        self.score = self.calculate_score()

    def add_enhancement(self) -> None:
        match self.enhancement:
            case Enhancement.STONE:
                self.chips = 50

            case Enhancement.GOLD:
                self.econ += 3

            case Enhancement.BONUS:
                self.chips += 30

            case Enhancement.MULT:
                self.add_mult += 4

            case Enhancement.WILD:
                self.is_any_suit = True

            case Enhancement.LUCKY:
                self.add_mult += 20
                self.econ += 20
                self.mult_prob = 0.2  # 1/5
                self.econ_prob = 0.05  # 1/20

            case Enhancement.GLASS:
                self.play_x_mult *= 2

            case Enhancement.STEEL:
                self.hand_x_mult *= 1.5

        self.calculate_score()

    def remove_enhancement(self) -> None:
        match self.enhancement:
            case Enhancement.STONE:
                self.chips = 0

            case Enhancement.GOLD:
                self.econ -= 3

            case Enhancement.BONUS:
                self.chips -= 30

            case Enhancement.MULT:
                self.add_mult -= 4

            case Enhancement.WILD:
                self.is_any_suit = False

            case Enhancement.LUCKY:
                self.add_mult -= 20
                self.econ -= 20
                self.mult_prob = 1
                self.econ_prob = 1

            case Enhancement.GLASS:
                self.play_x_mult /= 2

            case Enhancement.STEEL:
                self.hand_x_mult -= 1.5

        self.enhancement = Enhancement.NONE
        self.calculate_score()

    def add_edition(self) -> None:
        match self.edition:
            case Edition.FOIL:
                self.chips += 50

            case Edition.HOLOGRAPHIC:
                self.add_mult += 10

            case Edition.POLYCHROME:
                self.play_x_mult *= 1.5

        self.calculate_score()

    def add_seal(self) -> None:
        if self.seal == Seal.RED:
            self.trigger += 1

        self.calculate_score()

    def id(self) -> int:
        val = self.rank & 0b1111
        val = (val << 2) | (self.suit & 0b11)
        val = (val << 4) | (self.enhancement & 0b1111)
        val = (val << 3) | (self.seal & 0b111)
        val = (val << 2) | (self.edition & 0b11)
        return val

    def calculate_score(self) -> int:
        score = 1
        score *= ENHANCEMENT_SCORING[self.enhancement]
        score *= SEAL_SCORING[self.seal]
        score *= EDITION_SCORING[self.edition]
        score += get_initial_card_chips(self.rank)
        return score

    @classmethod
    def random(cls):
        return cls(
            rank=random.choice(list(Rank)),
            suit=random.choice(list(Suit)),
            enhancement=random.choice(list(Enhancement)),
            seal=random.choice(list(Seal)),
            edition=random.choice(list(Edition)),
        )

    @classmethod
    def build_dummy(cls):
        return Card(Rank.ACE, Suit.HEARTS, Enhancement.NONE, Seal.NONE, Edition.NONE)

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

    def __eq__(self, other):
        return self.card_id == other.card_id


@dataclass
class Hand:
    cards: list[Card]

    @classmethod
    def random_hand(cls, card_amount: int):
        return cls(cards=[Card.random() for _ in range(card_amount)])


@dataclass(slots=True)
class Deck:
    total_cards: int = 0
    cards: dict[Card, int] = field(default_factory=dict)
    facecards: list[Card] = field(default_factory=list)
    suits: dict[int, list[Card]] = field(default_factory=dict)
    ranks: dict[int, list[Card]] = field(default_factory=dict)
    discards: dict[Card, int] = field(default_factory=dict)
    suit_rank: dict[int, list[Card]] = field(default_factory=dict)

    def __post_init__(self):
        self._build_deck()

    def _build_deck(self) -> None:
        for rank in Rank:
            for suit in Suit:
                card = Card(
                    rank=rank,
                    suit=suit,
                    enhancement=Enhancement.NONE,
                    seal=Seal.NONE,
                    edition=Edition.NONE,
                )
                self.cards[card] = 1

                if card.is_facecard:
                    self.facecards.append(card)

                if card.suit not in self.suits:
                    self.suits[card.suit] = []
                self.suits[card.suit].append(card)

                if card.rank not in self.ranks:
                    self.ranks[card.rank] = []
                self.ranks[card.rank].append(card)

                suit_rank_key = suit << 5 | rank
                if suit_rank_key not in self.suit_rank:
                    self.suit_rank[suit_rank_key] = []
                self.suit_rank[suit_rank_key].append(card)

                self.total_cards += 1

    def filter(self, hand: list[Card]) -> None:
        for card in hand:
            self.cards[card] -= 1
            self.suits[card.suit].remove(card)
            self.ranks[card.rank].remove(card)

            if card.is_facecard:
                self.facecards.remove(card)

            if card not in self.discards:
                self.discards[card] = 0

            self.discards[card] += 1
            self.total_cards -= 1


@dataclass(slots=True)
class CardBucket:
    count: int
    cards: list[Card]


@dataclass(slots=True)
class JokerScoringConditions:
    card: Card = Card.build_dummy()
    card_index: int = -1
    hands_left: int = -1
    face_card_count: int = -1
    scoring_held: list[Card] = field(default_factory=list)
    unscoring_held: list[Card] = field(default_factory=list)
    scoring_played: list[Card] = field(default_factory=list)


# @dataclass(slots=True)
# class JokerConfig:
#     rarity: str
#     buy_price: int
#     copyable: bool
#     life: int | None = None


@dataclass(slots=True)
class JokerReq:
    rank: Rank | None = None
    suit: Suit | None = None


@dataclass(slots=True)
class Joker:
    background_image: JokersName
    face_image: JokersName | None
    negative: bool
    edition: Edition
    req: JokerReq
    copyable: bool
    joker_id: int = field(init=False, default=0)

    def _build_id(self):
        val = self.background_image | 0b11111111
        val = (val << 1) | self.negative
        val = (val << 4) | (self.edition & 0b111)
        self.joker_id = val

    def _add_face(self):
        if self.background_image in BACKGROUND_JOKERS:
            self.face_image = JokersName(int(self.background_image) + 10)

    def __post_init__(self):
        self._add_face()
        self._build_id()

    def __repr__(self):
        base = self.background_image.name.lower()
        if self.negative:
            base = f"negative {base}"
        elif self.edition != Edition.NONE:
            base = f"{self.edition.name.lower()} {base}"

        return base

    def __hash__(self) -> int:
        return self.joker_id

    @classmethod
    def random(cls):
        return Joker(
            background_image=random.choice(list(JokersName)),
            face_image=None,
            negative=random.choice([True, False]),
            edition=Edition(
                random.choice(
                    [edition.value for edition in Edition if edition.value >= 0]
                )
            ),
            req=JokerReq(),
            copyable=False,
        )

    @classmethod
    def build(cls, joker_name: JokersName):
        joker_key = (
            joker_name.name.lower()
            if "BACKGROUND" not in joker_name.name
            else joker_name.name.split("_BACKGROUND")[0].lower()
        )
        joker_data = CONFIG[joker_key]

        if "scoring" in joker_data:
            scoring_data = joker_data["scoring"]

            update = None
            if "update" in scoring_data:
                update_data = scoring_data["update"]
                update = JokerScoringUpdate(
                    count=update_data.get("count", ""),
                    effect=update_data.get("effect", ""),
                    trigger=JokerTriggers(update_data.get("trigger")),
                    condition=update_data.get("trigger", {}),
                    each=update_data.get("each", -1),
                    value=update_data.get("value", -1),
                    change=update_data.get("change", -1),
                )

            return JokerScoring(
                background_image=joker_name,
                face_image=None,
                negative=False,
                edition=Edition.NONE,
                req=JokerReq(),
                copyable=joker_data.get("copyable", False),
                trigger=JokerTriggers(scoring_data.get("trigger")),
                prob=scoring_data.get("prob", 1),
                chips=scoring_data.get("chips"),
                add_mult=scoring_data.get("add_mult"),
                x_mult=scoring_data.get("x_mult"),
                condition=scoring_data.get("condition"),
                update=update,
            )

        elif "retrigger" in joker_data:
            retrigger_data = joker_data["retrigger"]

            return JokerRetrigger(
                background_image=joker_name,
                face_image=None,
                negative=False,
                edition=Edition.NONE,
                req=JokerReq(),
                copyable=joker_data.get("copyable", False),
                trigger=JokerTriggers(retrigger_data.get("trigger")),
                times=retrigger_data.get("times"),
                condition=retrigger_data.get("condition", ""),
            )

        elif "copy" in joker_data:
            copy_data = joker_data["copy"]

            return JokerCopy(
                background_image=joker_name,
                face_image=None,
                negative=False,
                edition=Edition.NONE,
                req=JokerReq(),
                copyable=joker_data.get("copyable", False),
                position=copy_data.get("position"),
            )

        elif "game_modifier" in joker_data:
            modifier_data = joker_data["game_modifier"]

            return JokerGameModifier(
                background_image=joker_name,
                face_image=None,
                negative=False,
                edition=Edition.NONE,
                req=JokerReq(),
                copyable=joker_data.get("copyable", False),
                discards=modifier_data.get("discards", 0),
                hand_size=modifier_data.get("hand_size", 0),
                hands=modifier_data.get("hands", 0),
                all_cards_are_facecards=modifier_data.get(
                    "all_cards_are_facecards", False
                ),
                suit_groups=modifier_data.get("suit_groups", []),
                straight_size=modifier_data.get("straight_size", 5),
                flush_size=modifier_data.get("flush_size", 5),
                straight_gap_allowed=modifier_data.get("straight_gap_allowed", 1),
                all_played_cards_score=modifier_data.get(
                    "all_played_cards_score", False
                ),
                double_probabilities=modifier_data.get("double_probabilities", False),
                allow_duplicate_shop_items=modifier_data.get(
                    "allow_duplicate_shop_items", False
                ),
                disable_boss_blind=modifier_data.get("disable_boss_blind", False),
            )

        elif "update" in joker_data:
            modifier_data = joker_data["update"]

            rank_data = modifier_data["rank"]
            rank = Rank(rank_data) if isinstance(rank_data, int) else rank_data

            return JokerUpdate(
                background_image=joker_name,
                face_image=None,
                negative=False,
                edition=Edition.NONE,
                req=JokerReq(),
                copyable=joker_data["copyable"],
                trigger=JokerTriggers(modifier_data["trigger"]),
                rank=rank,
                enhacnement=Enhancement(modifier_data.get("enhancement", None)),
            )

        elif "econ" in joker_data:
            modifier_data = joker_data["econ"]

            return JokerEcon(
                background_image=joker_name,
                face_image=None,
                negative=False,
                edition=Edition.NONE,
                req=JokerReq(),
                copyable=joker_data["copyable"],
                trigger=JokerTriggers(modifier_data.get("trigger")),
                money=modifier_data.get("money"),
                condition=modifier_data.get("condition", None),
            )

        else:
            return Joker.random()


@dataclass(slots=True, repr=False, eq=False)
class JokerScoringUpdate:
    count: str
    effect: str
    trigger: JokerTriggers
    condition: dict
    each: int
    value: int
    change: int


@dataclass(slots=True, repr=False, eq=False)
class JokerScoring(Joker):
    trigger: JokerTriggers
    prob: float
    chips: int | dict[str, str | int] | None
    add_mult: int | dict[str, str | int] | None
    x_mult: float | dict[str, str | int] | None
    condition: dict | None
    update: JokerScoringUpdate | None


@dataclass(slots=True, repr=False, eq=False)
class JokerRetrigger(Joker):
    trigger: JokerTriggers
    times: int
    condition: str


@dataclass(slots=True, repr=False, eq=False)
class JokerCopy(Joker):
    position: str


@dataclass(slots=True, repr=False, eq=False)
class JokerGameModifier(Joker):
    discards: int
    hand_size: int
    hands: int
    all_cards_are_facecards: bool
    suit_groups: list[list[str]]
    straight_size: int
    flush_size: int
    straight_gap_allowed: int
    all_played_cards_score: bool
    double_probabilities: bool
    allow_duplicate_shop_items: bool
    disable_boss_blind: bool


@dataclass(slots=True, repr=False, eq=False)
class JokerUpdate(Joker):
    trigger: JokerTriggers
    rank: Rank | str | None
    enhacnement: Enhancement


@dataclass(slots=True, repr=False, eq=False)
class JokerEcon(Joker):
    money: int
    trigger: JokerTriggers
    condition: dict | None


@dataclass(slots=True)
class BestHand:
    chips: int = 0
    worst_case_mult: float = 0
    avg_case_mult: float = 0
    best_case_mult: float = 0


@dataclass(slots=True)
class HandScoring:
    hand_stats: HandStats = HandStats()
    scored_played: list[Card] = field(default_factory=list)
    unscored_played: list[Card] = field(default_factory=list)
    scored_held: list[Card] = field(default_factory=list)
    unscored_held: list[Card] = field(default_factory=list)


@dataclass(slots=True)
class JokerPlan:
    on_played: list[JokerScoring] = field(default_factory=list)
    on_held: list[JokerScoring] = field(default_factory=list)
    after_hand: list[JokerScoring] = field(default_factory=list)
    played_retrigger: list[JokerRetrigger] = field(default_factory=list)
    held_retrigger: list[JokerRetrigger] = field(default_factory=list)
    update_jokers: list[JokerUpdate | JokerScoring] = field(default_factory=list)


@dataclass(slots=True)
class FinalScoringResults:
    best_hand: BestHand = field(default_factory=BestHand)
    hand_scoring: HandScoring = field(default_factory=HandScoring)
    joker_plan: JokerPlan = field(default_factory=JokerPlan)


@dataclass(slots=True)
class GameState:
    hands: int = 4
    discards: int = 3
    hand_size: int = 8
    flush_size: int = 5
    straight_size: int = 5
    probabily_mult: int = 1
    straight_gap_allowed: int = 1
    disable_boss_blind: bool = False
    all_played_cards_score: bool = False
    all_cards_are_facecards: bool = False
    allow_duplicate_shop_items: bool = False
    suit_groups: list[list[str]] = field(default_factory=list)


@dataclass(slots=True)
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


@dataclass
class CardData:
    card: Card
    location: list[float]
