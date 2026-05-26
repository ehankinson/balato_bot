from core.enums import JokersName, JokerTriggers
from core.models import Joker, JokerCopy, JokerRetrigger, JokerScoring


def generate_blueprint_permutations(
    jokers: list[JokerScoring] | list[JokerRetrigger],
) -> list[list[Joker]]:
    final_list = []
    for i in range(len(jokers)):
        new_list = jokers[:i] + [jokers[i]] + jokers[i:]
        final_list.append(new_list)

    return final_list


COPY_FUNCTION = {JokersName.BLUEPRINT: generate_blueprint_permutations}


def build_mult_jokers(
    jokers: list[JokerScoring],
) -> tuple[list[JokerScoring], list[JokerScoring]]:
    add_mult = []
    x_mult = []
    chips = []
    for joker in jokers:
        if joker.add_mult is not None:
            add_mult.append(joker)

        elif joker.x_mult is not None:
            x_mult.append(joker)

        elif joker.chips is not None:
            chips.append(joker)

    add_mult = sorted(add_mult, key=lambda joker: joker.trigger)
    x_mult = sorted(x_mult, key=lambda joker: joker.trigger)

    return chips, add_mult + x_mult


def generate_copy_combos(
    edit_jokers: list[JokerScoring] | list[JokerRetrigger],
    copy_jokers: list[JokerCopy],
) -> list[list[Joker]]:
    final_list: list[list[Joker]] = []

    if len(copy_jokers) == 1:
        copy_joker = copy_jokers[0]
        function = COPY_FUNCTION[copy_joker.background_image]
        final_list.extend(function(edit_jokers))

    return final_list


def build_copy_combos(
    copy_jokers: list[JokerCopy],
    scoring_jokers: list[JokerScoring],
    retrigger_jokers: list[JokerRetrigger],
) -> list[list[Joker]]:
    final_jokers = []

    chip_jokers, mult_jokers = build_mult_jokers(scoring_jokers)

    if len(chip_jokers) > 0:
        chip_combos = generate_copy_combos(mult_jokers, copy_jokers)
        final_jokers.extend(
            mult_jokers + chip_combo + retrigger_jokers for chip_combo in chip_combos
        )

    if len(mult_jokers) > 0:
        mult_combos = generate_copy_combos(mult_jokers, copy_jokers)
        final_jokers.extend(
            mult_combo + chip_jokers + retrigger_jokers for mult_combo in mult_combos
        )

    if len(retrigger_jokers) > 0:
        retrigger_combos = generate_copy_combos(retrigger_jokers, copy_jokers)
        final_jokers.extend(
            mult_jokers + chip_jokers + retrigger_combo
            for retrigger_combo in retrigger_combos
        )

    return final_jokers


def get_joker_type[T: Joker](jokers: list[Joker], joker_type: type[T]) -> list[T]:
    return [joker for joker in jokers if isinstance(joker, joker_type)]


def generate_scoring_jokers_combinations(jokers: list[Joker]) -> list[list[Joker]]:
    # checking if there are any jokers that affect the final score
    if not any(
        type(joker) in [JokerScoring, JokerCopy, JokerRetrigger] for joker in jokers
    ):
        return []  # return an empty list since there are no jokers to itera over for best score

    copy_jokers = get_joker_type(jokers, JokerCopy)
    scoring_jokers = get_joker_type(jokers, JokerScoring)
    retrigger_jokers = get_joker_type(jokers, JokerRetrigger)

    if len(copy_jokers) > 0:
        # Blueprint only copies jokers to their right, so we can remove permutations when blueprint is the right most
        # Brainstome only copies jokers that are the leftmost, so we only need all permutations of other cards to at the left most position
        # When we have 2 blueprint, ummmmmm
        # when we have 2 brainstroms, ummm
        # when we have both blueprint and brainstorme, ummmmmmmmmmmmmmmmmmmmmmmmmmm

        return build_copy_combos(copy_jokers, scoring_jokers, retrigger_jokers)

    return [mult_jokers]
