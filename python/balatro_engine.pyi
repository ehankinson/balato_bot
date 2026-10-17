from typing import ClassVar, NotRequired, TypedDict

import numpy as np
from numpy.typing import NDArray

class Action:
    SelectBlind: ClassVar[Action]
    SkipBlind: ClassVar[Action]
    PlayCards: ClassVar[Action]
    DiscardCards: ClassVar[Action]
    BuyShopItem: ClassVar[Action]
    BuyVoucher: ClassVar[Action]
    RerollShop: ClassVar[Action]
    BuyPack: ClassVar[Action]
    SelectPackOption: ClassVar[Action]
    UseConsumable: ClassVar[Action]
    SellJoker: ClassVar[Action]
    SellConsumable: ClassVar[Action]
    LeaveShop: ClassVar[Action]
    SkipPack: ClassVar[Action]
    UsePackConsumable: ClassVar[Action]
    ReorderJokers: ClassVar[Action]
    ReorderHand: ClassVar[Action]
    RerollBoss: ClassVar[Action]

    def __int__(self) -> int: ...

class Deck:
    Red: ClassVar[Deck]
    Blue: ClassVar[Deck]
    Yellow: ClassVar[Deck]
    Green: ClassVar[Deck]
    Black: ClassVar[Deck]
    Magic: ClassVar[Deck]
    Nebula: ClassVar[Deck]
    Ghost: ClassVar[Deck]
    Abandoned: ClassVar[Deck]
    Checkered: ClassVar[Deck]
    Zodiac: ClassVar[Deck]
    Painted: ClassVar[Deck]
    Anaglyph: ClassVar[Deck]
    Plasma: ClassVar[Deck]
    Erratic: ClassVar[Deck]

    def __int__(self) -> int: ...

class Stake:
    White: ClassVar[Stake]
    Red: ClassVar[Stake]
    Green: ClassVar[Stake]
    Black: ClassVar[Stake]
    Blue: ClassVar[Stake]
    Purple: ClassVar[Stake]
    Orange: ClassVar[Stake]
    Gold: ClassVar[Stake]

    def __int__(self) -> int: ...

Observation = TypedDict(
    "Observation",
    {
        "schema_version": NDArray[np.int32],
        "sections": NDArray[np.bool_],
        "run.cat": NDArray[np.int32],
        "run.counts": NDArray[np.int64],
        "run.num": NDArray[np.float32],
        "run.extra": NDArray[np.int64],
        "run.used_planets": NDArray[np.bool_],
        "run.flags": NDArray[np.bool_],
        "run.visible": NDArray[np.bool_],
        "blind.cat": NDArray[np.int32],
        "blind.counts": NDArray[np.int64],
        "blind.num": NDArray[np.float32],
        "blind.flags": NDArray[np.bool_],
        "blind.visible": NDArray[np.bool_],
        "blind.played_types": NDArray[np.bool_],
        "shop.num": NDArray[np.float32],
        "shop.visible": NDArray[np.bool_],
        "pack.cat": NDArray[np.int32],
        "pack.counts": NDArray[np.int64],
        "pack.visible": NDArray[np.bool_],
        "jokers.cat": NDArray[np.int32],
        "jokers.num": NDArray[np.float32],
        "jokers.flags": NDArray[np.bool_],
        "jokers.present": NDArray[np.bool_],
        "jokers.visible": NDArray[np.bool_],
        "joker_values.cat": NDArray[np.int32],
        "joker_values.num": NDArray[np.float32],
        "joker_values.flags": NDArray[np.bool_],
        "joker_values.present": NDArray[np.bool_],
        "joker_values.visible": NDArray[np.bool_],
        "consumables.cat": NDArray[np.int32],
        "consumables.num": NDArray[np.float32],
        "consumables.flags": NDArray[np.bool_],
        "consumables.present": NDArray[np.bool_],
        "consumables.visible": NDArray[np.bool_],
        "last_consumable.cat": NDArray[np.int32],
        "last_consumable.num": NDArray[np.float32],
        "last_consumable.flags": NDArray[np.bool_],
        "last_consumable.present": NDArray[np.bool_],
        "last_consumable.visible": NDArray[np.bool_],
        "deck_catalogue.cat": NDArray[np.int32],
        "deck_catalogue.num": NDArray[np.float32],
        "deck_catalogue.flags": NDArray[np.bool_],
        "deck_catalogue.present": NDArray[np.bool_],
        "deck_catalogue.visible": NDArray[np.bool_],
        "poker_hands.cat": NDArray[np.int32],
        "poker_hands.num": NDArray[np.float32],
        "poker_hands.flags": NDArray[np.bool_],
        "poker_hands.present": NDArray[np.bool_],
        "poker_hands.visible": NDArray[np.bool_],
        "vouchers.cat": NDArray[np.int32],
        "vouchers.num": NDArray[np.float32],
        "vouchers.flags": NDArray[np.bool_],
        "vouchers.present": NDArray[np.bool_],
        "vouchers.visible": NDArray[np.bool_],
        "tags.cat": NDArray[np.int32],
        "tags.num": NDArray[np.float32],
        "tags.flags": NDArray[np.bool_],
        "tags.present": NDArray[np.bool_],
        "tags.visible": NDArray[np.bool_],
        "blind_offers.cat": NDArray[np.int32],
        "blind_offers.num": NDArray[np.float32],
        "blind_offers.flags": NDArray[np.bool_],
        "blind_offers.present": NDArray[np.bool_],
        "blind_offers.visible": NDArray[np.bool_],
        "hand.cat": NDArray[np.int32],
        "hand.num": NDArray[np.float32],
        "hand.flags": NDArray[np.bool_],
        "hand.present": NDArray[np.bool_],
        "hand.visible": NDArray[np.bool_],
        "shop_items.cat": NDArray[np.int32],
        "shop_items.num": NDArray[np.float32],
        "shop_items.flags": NDArray[np.bool_],
        "shop_items.present": NDArray[np.bool_],
        "shop_items.visible": NDArray[np.bool_],
        "shop_vouchers.cat": NDArray[np.int32],
        "shop_vouchers.num": NDArray[np.float32],
        "shop_vouchers.flags": NDArray[np.bool_],
        "shop_vouchers.present": NDArray[np.bool_],
        "shop_vouchers.visible": NDArray[np.bool_],
        "shop_packs.cat": NDArray[np.int32],
        "shop_packs.num": NDArray[np.float32],
        "shop_packs.flags": NDArray[np.bool_],
        "shop_packs.present": NDArray[np.bool_],
        "shop_packs.visible": NDArray[np.bool_],
        "pack_options.cat": NDArray[np.int32],
        "pack_options.num": NDArray[np.float32],
        "pack_options.flags": NDArray[np.bool_],
        "pack_options.present": NDArray[np.bool_],
        "pack_options.visible": NDArray[np.bool_],
        "deck_multiplicity": NDArray[np.int64],
        "poker_hand_counts": NDArray[np.int64],
        "action_kinds": NDArray[np.bool_],
    },
)

class ActionConstraints(TypedDict):
    source_mask: NDArray[np.bool_]
    source_known: NDArray[np.bool_]
    target_collection: NDArray[np.int32]
    target_mask: NDArray[np.bool_]
    required_targets: NDArray[np.bool_]
    count_mask: NDArray[np.bool_]
    ordered: NDArray[np.bool_]
    permutation_collection: NDArray[np.int32]
    can_submit: NDArray[np.bool_]

class Game:
    @property
    def ante(self) -> int: ...
    @property
    def blind_index(self) -> int | None: ...
    @property
    def decision_point(self) -> int: ...
    def observe(self) -> Observation: ...
    def action_constraints(
        self, action: Action, source: int | None = None
    ) -> ActionConstraints: ...
    def selection_constraints(
        self, action: Action, prefix: NDArray[np.int32], source: int | None = None
    ) -> ActionConstraints: ...
    def next_step(self) -> int: ...
    def apply_action(
        self,
        action: Action,
        source: int | None = None,
        targets: NDArray[np.int32] | None = None,
        order: NDArray[np.int32] | None = None,
    ) -> int: ...

def new(seed: int, deck: Deck = Deck.Red, stake: Stake = Stake.White) -> Game: ...

type CardData = tuple[int, int, int]
type DiscardResult = tuple[int, float, list[CardData]]

def generate_discard_table(
    total_cards: int,
    suit_counts: list[int],
    suit_scores: list[int],
    rank_counts: list[int],
    rank_scores: list[int],
    suit_rank_counts: list[int],
    suit_rank_scores: list[int],
    hand: list[CardData],
) -> list[DiscardResult]: ...

type BatchArrays = dict[
    str, NDArray[np.int32] | NDArray[np.int64] | NDArray[np.float32] | NDArray[np.bool_]
]

class BatchStepResult(TypedDict):
    accepted: NDArray[np.bool_]
    attempted: NDArray[np.bool_]
    phases: NDArray[np.int32]
    errors: list[str | None]
    observations: BatchArrays | None
    timings: NotRequired[dict[str, float]]

class GameBatch:
    def __len__(self) -> int: ...
    def observe(self) -> BatchArrays: ...
    def reset(self, indices: NDArray[np.int32], seeds: NDArray[np.uint64]) -> None: ...
    def constraints(
        self,
        kinds: NDArray[np.int32],
        sources: NDArray[np.int32],
        prefixes: NDArray[np.int32],
    ) -> BatchArrays: ...
    def step(
        self, commands: NDArray[np.int32], observe: bool = True, profile: bool = False
    ) -> BatchStepResult: ...

def new_batch(
    seeds: NDArray[np.uint64],
    decks: list[Deck] | None = None,
    stakes: list[Stake] | None = None,
) -> GameBatch: ...
def build_profile() -> str: ...
