from collections.abc import Callable, Iterable
from functools import reduce
from itertools import combinations, permutations
from typing import TypeVar

from core.enums import Enhancement, Rank, Suit
from core.hand_stats import HandStats
from core.models import Card

K = TypeVar("K")


def add_combination(iter: list, size: int) -> list:
    return [list(val) for val in combinations(iter, size)]


def add_permutation(iter: list, size: int) -> list:
    return [list(val) for val in permutations(iter, size)]


def unique_cards(cards: list[Card]) -> int:
    return len(set(card.rank for card in cards))


def get_stone_cards(cards: list[Card]) -> list[Card]:
    return [card for card in cards if card.enhancement == Enhancement.STONE]


def get_steel_cards(cards: list[Card]) -> list[Card]:
    return [card for card in cards if card.enhancement == Enhancement.STEEL]


def bucket_id(cards: Iterable[Card]) -> dict[int, list[Card]]:
    return bucket_by(cards, lambda card: card.card_id)


def bucket_rank(cards: Iterable[Card]) -> dict[Rank, list[Card]]:
    return bucket_by(cards, lambda card: card.rank, skip_stones=True)


def bucket_suit(cards: Iterable[Card]) -> dict[Suit, list[Card]]:
    return bucket_by(cards, lambda card: card.suit, skip_stones=True)


def nested_getattr(obj, attr_path, default=None):
    try:
        return reduce(getattr, attr_path.split("."), obj)
    except AttributeError:
        return default


def bucket_by(
    cards: Iterable[Card],
    key_func: Callable[[Card], K],
    *,
    skip_stones: bool = False,
) -> dict[K, list[Card]]:
    bucket: dict[K, list[Card]] = {}

    for card in cards:
        if skip_stones and card.enhancement == Enhancement.STONE:
            continue

        key = key_func(card)

        if key not in bucket:
            bucket[key] = []

        bucket[key].append(card)

    return bucket
