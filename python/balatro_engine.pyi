from typing import ClassVar


class Action:
    SelectBlind: ClassVar["Action"]
    SkipBlind: ClassVar["Action"]
    PlayCards: ClassVar["Action"]
    DiscardCards: ClassVar["Action"]
    BuyShopItem: ClassVar["Action"]
    BuyVoucher: ClassVar["Action"]
    RerollShop: ClassVar["Action"]
    BuyPack: ClassVar["Action"]
    SelectPackOption: ClassVar["Action"]
    UseConsumable: ClassVar["Action"]
    SellJoker: ClassVar["Action"]
    SellConsumable: ClassVar["Action"]
    LeaveShop: ClassVar["Action"]
    SkipPack: ClassVar["Action"]
    UsePackConsumable: ClassVar["Action"]
    ReorderJokers: ClassVar["Action"]
    ReorderHand: ClassVar["Action"]
    RerollBoss: ClassVar["Action"]

    def __int__(self) -> int: ...


class Deck:
    Red: ClassVar["Deck"]
    Blue: ClassVar["Deck"]
    Yellow: ClassVar["Deck"]
    Green: ClassVar["Deck"]
    Black: ClassVar["Deck"]
    Magic: ClassVar["Deck"]
    Nebula: ClassVar["Deck"]
    Ghost: ClassVar["Deck"]
    Abandoned: ClassVar["Deck"]
    Checkered: ClassVar["Deck"]
    Zodiac: ClassVar["Deck"]
    Painted: ClassVar["Deck"]
    Anaglyph: ClassVar["Deck"]
    Plasma: ClassVar["Deck"]
    Erratic: ClassVar["Deck"]

    def __int__(self) -> int: ...


class Stake:
    White: ClassVar["Stake"]
    Red: ClassVar["Stake"]
    Green: ClassVar["Stake"]
    Black: ClassVar["Stake"]
    Blue: ClassVar["Stake"]
    Purple: ClassVar["Stake"]
    Orange: ClassVar["Stake"]
    Gold: ClassVar["Stake"]

    def __int__(self) -> int: ...


class Game:
    @property
    def ante(self) -> int: ...
    @property
    def blind_index(self) -> int | None: ...
    @property
    def decision_point(self) -> int: ...
    def next_step(self) -> int: ...
    def apply_action(
        self,
        action: Action,
        source: int | None = None,
        targets: list[int] | None = None,
        order: list[int] | None = None,
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
