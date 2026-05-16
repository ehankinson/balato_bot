from itertools import product

from calculation.util import add_combination, add_permutation, nested_getattr
from core.enums import JokersName, JokerTriggers
from core.models import Joker


def generate_blueprint_permutations(
    jokers: list[Joker], copy_joker: Joker
) -> list[list[Joker]]:
    final_list: list[list[Joker]] = []
    for i in range(len(jokers)):
        new_list = jokers[:i] + [copy_joker] + jokers[i:]
        final_list.append(new_list)

    return final_list


COPY_FUNCTION = {JokersName.BLUEPRINT: generate_blueprint_permutations}


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

    # since add_mult will always be played before x_mult
    # because ganruteed higher score 0_0
    return [add + mul]


def generate_copy_combos(
    jokers: list[Joker], edit_jokers: list[Joker], copy_jokers: list[Joker]
) -> list[list[Joker]]:
    final_list: list[list[Joker]] = []
    other_jokers = [
        joker
        for joker in jokers
        if joker not in edit_jokers and joker not in copy_jokers
    ]

    copy_combinations: list[list[Joker]] = []
    if len(copy_jokers) == 1:
        copy_joker = copy_jokers[0]
        function = COPY_FUNCTION[copy_joker.background_image]
        copy_combinations = function(edit_jokers, copy_joker)

    final_list.extend([copy_combo + other_jokers for copy_combo in copy_combinations])
    return final_list


def build_copy_combos(
    jokers: list[Joker],
    copy_jokers: list[Joker],
    mult_jokers: list[Joker],
    after_hand_combos: list[list[Joker]],
    per_card_combos: list[list[Joker]],
):
    final_jokers = []
    
    chip_jokers = get_jokers_with(jokers, "scoring.chips")
    if len(chip_jokers) > 0:
        chip_combos = generate_copy_combos(jokers, mult_jokers, copy_jokers)
        final_jokers.extend(chip_combos)

    if len(mult_jokers) > 0:
        mult_combos = generate_copy_combos(jokers, mult_jokers, copy_jokers)
        final_jokers.extend(mult_combos)

    retrigger_jokers = [joker for joker in jokers if joker.retrigger is not None]
    if len(retrigger_jokers) > 0:
        retrigger_combos = generate_copy_combos(jokers, retrigger_jokers, copy_jokers)
        final_jokers.extend(retrigger_combos)

    return final_jokers


def generate_possible_jokers(jokers: list[Joker]) -> list[list[Joker]]:
    scoring_jokers = get_jokers_with(jokers, "scoring")
    if len(scoring_jokers) == 0:
        return [jokers]

    mult_jokers = get_jokers_with(jokers, "scoring.add_mult", "scoring.x_mult")

    none_mult_jokers = [joker for joker in jokers if joker not in mult_jokers]
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
        # Blueprint only copies jokers to their right, so we can remove permutations when blueprint is the right most
        # Brainstome only copies jokers that are the leftmost, so we only need all permutations of other cards to at the left most position
        # When we have 2 blueprint, ummmmmm
        # when we have 2 brainstroms, ummm
        # when we have both blueprint and brainstorme, ummmmmmmmmmmmmmmmmmmmmmmmmmm

        return build_copy_combos(
            jokers,
            copy_jokers,
            mult_jokers,
            possible_after_hand_jokers,
            possible_played_card_jokers,
        )

    return [
        none_mult_jokers
        + list(after_hand_order)
        + list(played_card_order)
        + list(held_card_order)
        for after_hand_order, played_card_order, held_card_order in product(
            possible_after_hand_jokers,
            possible_played_card_jokers,
            possible_helded_card_jokers,
        )
    ]
