from itertools import product

from calculation.util import add_combination, add_permutation
from core.enums import JokerTriggers
from core.models import Joker


def get_scoring_type_joker(trigger: int, jokers: list[Joker]) -> list[Joker]:
    return [joker for joker in jokers if joker.scoring.trigger == trigger]


def get_per_card_jokers(jokers: list[Joker]) -> list[Joker]:
    return [
        joker
        for joker in jokers
        if joker.scoring is not None
        and joker.scoring.trigger == JokerTriggers.ON_PLAYED_CARDS
    ]


def get_after_hand_jokers(jokers: list[Joker]) -> list[Joker]:
    return [
        joker
        for joker in jokers
        if joker.scoring is not None
        and joker.scoring.trigger == JokerTriggers.AFTER_HAND
    ]


def get_retrigger_jokers(jokers: list[Joker]) -> list[Joker]:
    return [joker for joker in jokers if joker.retrigger is not None]


def get_mult_jokers(jokers: list[Joker]) -> list[Joker]:
    return [
        joker
        for joker in jokers
        if joker.scoring is not None
        and (joker.scoring.add_mult is not None or joker.scoring.x_mult is not None)
    ]


def check_for_order(jokers: list[Joker]) -> list[list[Joker]]:
    mul = []
    add = []

    for joker in jokers:
        if joker.scoring.x_mult is not None:
            mul.append(joker)

        if joker.scoring.add_mult is not None or joker.scoring.chips is not None:
            add.append(joker)

    final_jokers: list[list[Joker]] = []
    if len(mul) > 0 and len(add) == 0:
        final_jokers.extend(add_combination(mul, len(mul)))

    elif len(add) > 0 and len(mul) == 0:
        final_jokers.extend(add_combination(add, len(add)))

    else:
        final_jokers.extend(add_permutation(jokers, len(jokers)))

    return final_jokers


def generate_possible_jokers(jokers: list[Joker]) -> list[list[Joker]]:
    mult_jokers = get_mult_jokers(jokers)
    if len(mult_jokers) == 0:
        return [jokers]

    none_scoring_jokers = [joker for joker in jokers if joker not in mult_jokers]
    after_hand_jokers = get_scoring_type_joker(JokerTriggers.AFTER_HAND, mult_jokers)
    per_card_jokers = get_scoring_type_joker(JokerTriggers.ON_PLAYED_CARDS, mult_jokers)

    possible_after_hand_jokers = check_for_order(after_hand_jokers)
    possible_per_hand_jokers = check_for_order(per_card_jokers)

    return [
        none_scoring_jokers + list(after_hand_order) + list(per_card_order)
        for after_hand_order, per_card_order in product(
            possible_after_hand_jokers, possible_per_hand_jokers
        )
    ]
