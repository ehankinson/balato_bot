from itertools import product

from calculation.util import add_combination, add_permutation, nested_getattr
from core.enums import JokerTriggers
from core.models import Joker


def get_scoring_type_joker(trigger: int, jokers: list[Joker]) -> list[Joker]:
    return [joker for joker in jokers if joker.scoring.trigger == trigger]


def check(joker: Joker, trigger: JokerTriggers) -> bool:
    return joker.scoring is not None and joker.scoring.trigger == trigger


def get_trigger_jokers(jokers: list[Joker], trigger: JokerTriggers) -> list[Joker]:
    return_list = []

    for i, joker in enumerate(jokers):
        if joker.copy and check(jokers[i + 1], trigger):
            return_list.append(jokers[i + 1])

        if check(joker, trigger):
            return_list.append(joker)

    return return_list


def get_retrigger_jokers(jokers: list[Joker]) -> list[Joker]:
    return [joker for joker in jokers if joker.retrigger is not None]


def get_jokers_with(jokers: list[Joker], *checks: str) -> list[Joker]:
    return [
        joker
        for joker in jokers
        if any(nested_getattr(joker, check) is not None for check in checks)
    ]


def insert_copy_joker(
    copy_jokers: list[Joker], insert_jokers: list[Joker]
) -> list[list[Joker]]:
    chips = []
    for copy in copy_jokers:
        for i in range(len(insert_jokers)):
            val = []
            for j, chip in enumerate(insert_jokers):
                if j == i:
                    val.extend([copy, chip])
                else:
                    val.append(chip)

            chips.append(val)

    return chips


def add_copy_joker(copy: Joker, values: list[list[Joker]]) -> list[list[Joker]]:
    final_combos = []
    for combo in values:
        combo_len = len(combo)
        for i in range(combo_len):
            update = []
            has_added = False
            for j in range(combo_len):
                if i == j and not has_added:
                    update.append(copy)
                    has_added = True

                update.append(combo[j])
            final_combos.append(update)
    return final_combos


def check_for_order(jokers: list[Joker]) -> list[list[Joker]]:
    mul = []
    add = []

    for joker in jokers:
        if joker.scoring.x_mult is not None:
            mul.append(joker)
        else:
            add.append(joker)

    # since add will always be played before x_mult it is never better
    # to play x_mult first
    return [add + mul]


def generate_with_copy_joker(
    jokers: list[Joker],
    copy_jokers: list[Joker],
    mult_jokers: list[Joker],
    after_hand_combos: list[list[Joker]],
    per_card_combos: list[list[Joker]],
):
    final_jokers = []
    chip_jokers = get_jokers_with(jokers, "scoring.chips")

    if len(chip_jokers) > 0:
        chip_combos = insert_copy_joker(copy_jokers, chip_jokers)
        if len(after_hand_combos) > 0 or len(per_card_combos) > 0:
            other_jokers = [
                joker
                for joker in jokers
                if joker not in chip_jokers
                and joker not in mult_jokers
                and joker not in copy_jokers
            ]

            for chip_order, after_order, per_card_order in product(
                chip_combos, after_hand_combos, per_card_combos
            ):
                final_jokers.append(
                    other_jokers + chip_order + list(after_order) + list(per_card_order)
                )
        else:
            other_jokers = [
                joker
                for joker in jokers
                if joker not in chip_jokers and joker not in copy_jokers
            ]

            for chip in chip_combos:
                final_jokers.append(other_jokers + chip)

    if len(mult_jokers) > 0:
        other_jokers = [
            joker
            for joker in jokers
            if joker not in mult_jokers and joker not in copy_jokers
        ]

        for copy in copy_jokers:
            for i in range(2):
                arg, opp = (
                    (after_hand_combos, per_card_combos)
                    if i == 0
                    else (per_card_combos, after_hand_combos)
                )
                with_copy = add_copy_joker(copy, arg)

                for val_with_copy, original_val in product(with_copy, opp):
                    final_jokers.append(val_with_copy + original_val + other_jokers)

    retrigger_jokers = [joker for joker in jokers if joker.retrigger is not None]
    if len(retrigger_jokers) > 0:
        trigger_combos = insert_copy_joker(copy_jokers, retrigger_jokers)
        other_jokers = [
            joker
            for joker in jokers
            if joker not in retrigger_jokers
            and joker not in mult_jokers
            and joker not in copy_jokers
        ]

        for retrigger, afterhand, percard in product(
            trigger_combos, after_hand_combos, per_card_combos
        ):
            final_jokers.append(retrigger + afterhand + percard + other_jokers)
    return final_jokers


def generate_possible_jokers(jokers: list[Joker]) -> list[list[Joker]]:
    scoring_jokers = get_jokers_with(jokers, "scoring")
    if len(scoring_jokers) == 0:
        return [jokers]

    mult_jokers = get_jokers_with(jokers, "scoring.add_mult", "scoring.x_mult")

    none_scoring_jokers = [joker for joker in jokers if joker not in mult_jokers]
    after_hand_jokers = get_scoring_type_joker(JokerTriggers.AFTER_HAND, mult_jokers)
    played_card_jokers = get_scoring_type_joker(
        JokerTriggers.ON_PLAYED_CARDS, mult_jokers
    )
    helded_card_jokers = get_scoring_type_joker(
        JokerTriggers.ON_HELD_CARDS, mult_jokers
    )

    possible_after_hand_jokers = check_for_order(after_hand_jokers)
    possible_played_card_jokers = check_for_order(played_card_jokers)
    possible_helded_card_jokers = check_for_order(helded_card_jokers)

    copy_jokers = get_jokers_with(jokers, "copy")
    if len(copy_jokers) > 0:
        return generate_with_copy_joker(
            jokers,
            copy_jokers,
            mult_jokers,
            possible_after_hand_jokers,
            possible_played_card_jokers,
        )

    return [
        none_scoring_jokers
        + list(after_hand_order)
        + list(played_card_order)
        + list(held_card_order)
        for after_hand_order, played_card_order, held_card_order in product(
            possible_after_hand_jokers,
            possible_played_card_jokers,
            possible_helded_card_jokers,
        )
    ]
