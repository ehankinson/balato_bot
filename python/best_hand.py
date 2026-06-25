import sys
import time
from copy import copy, deepcopy

from cv2 import decomposeProjectionMatrix
from tqdm import tqdm
from typing_extensions import Literal, overload

from calculation.joker_generation import generate_scoring_jokers_combinations
from calculation.joker_retrigger import calculate_joker_retrigger
from calculation.joker_scoring import calculate_joker_scoring
from calculation.joker_update import calculate_joker_update
from calculation.poker_generation import generate_scoring_hand_combinations
from core.enums import (
    Edition,
    Enhancement,
    JokersName,
    JokerTriggers,
    Rank,
    Seal,
    Suit,
)
from core.models import (
    BestHand,
    Card,
    FinalScoringResults,
    GameState,
    HandScoring,
    Joker,
    JokerPlan,
    JokerRetrigger,
    JokerScoring,
    JokerScoringConditions,
    JokerUpdate,
)

JOKER_CACHE: dict[int, dict[int, tuple[int, int, float]]] = {}
ADD = 0
MULT = 1
NO_CACHE_JOKERS = {JokersName.PHOTOGRAPH}


def build_joker_plan(jokers: list[Joker]) -> JokerPlan:
    plan = JokerPlan([], [], [], [], [], [])

    for joker in jokers:
        if isinstance(joker, JokerScoring):
            match joker.trigger:
                case JokerTriggers.ON_PLAYED_CARDS:
                    plan.on_played.append(joker)
                case JokerTriggers.ON_HELD_CARDS:
                    plan.on_held.append(joker)
                case JokerTriggers.AFTER_HAND:
                    plan.after_hand.append(joker)

            if joker.update is not None:
                plan.update_jokers.append(joker)

        elif isinstance(joker, JokerRetrigger):
            if joker.trigger == JokerTriggers.ON_PLAYED_CARDS:
                plan.played_retrigger.append(joker)
            else:
                plan.held_retrigger.append(joker)

        elif isinstance(joker, JokerUpdate):
            plan.update_jokers.append(joker)

    return plan


def add_to_order(
    add_mult: int,
    x_mult: float,
    prob: float,
    mult_scoring_order: list[tuple[int, int | float, float]],
    is_lucky: bool = False,
) -> None:
    # This is annoying, since if we have a lucky hologram card, the way its currently implemented
    # the card would have +30 mult with a 1/5 change of hitting, BUT it should be 20 * 1/5 + 10
    # so we have this special check
    if add_mult > ADD:
        if is_lucky:
            mult_scoring_order.append((ADD, 20, prob))
            add_mult -= 20
            prob = 1

        if add_mult > 0:
            mult_scoring_order.append((ADD, add_mult, prob))

    if x_mult > MULT:
        mult_scoring_order.append((MULT, x_mult, prob))


def extend_order(
    start_index: int,
    trigger: int,
    mult_scoring_order: list[tuple[int, int | float, float]],
) -> int:
    mult_scoring_order.extend(
        mult_scoring_order[start_index : len(mult_scoring_order)] * (trigger - 1)
    )

    return len(mult_scoring_order)


def apply_joker_update(
    joker_plan: JokerPlan,
    hand_scoring: HandScoring,
    on_play_jokers: list[JokerScoring],
    on_held_jokers: list[JokerScoring],
    after_hand_jokers: list[JokerScoring],
) -> tuple[
    list[Card],
    list[JokerScoring],
    list[JokerScoring],
    list[JokerScoring],
]:
    # this code currently causes an issue since we are update references
    # and each other instand in hand_scoring uses that same reference, so we need to
    # update it back to the original state of the card
    copy_update_jokers = [
        deepcopy(joker) if isinstance(joker, JokerScoring) else joker
        for joker in joker_plan.update_jokers
    ]

    copy_hand_scoring = copy(hand_scoring)
    copy_hand_scoring.scored_played = deepcopy(hand_scoring.scored_played)

    calculate_joker_update(
        copy_update_jokers, copy_hand_scoring, JokerTriggers.BEFORE_PLAYED_CARDS
    )

    # use the copy referenced card(s)
    scoring_played = copy_hand_scoring.scored_played

    def replace_deep_copy_joker(
        jokers: list[JokerScoring], updated_joker: JokerScoring
    ) -> list[JokerScoring]:
        new_array = copy(jokers)
        replace_index = next(
            i
            for i, og_joker in enumerate(jokers)
            if og_joker.joker_id == updated_joker.joker_id
        )
        new_array[replace_index] = updated_joker

        return new_array

    for joker in copy_update_jokers:
        if not isinstance(joker, JokerScoring):
            continue

        match joker.trigger:
            case JokerTriggers.ON_PLAYED_CARDS:
                on_play_jokers = replace_deep_copy_joker(on_play_jokers, joker)

            case JokerTriggers.ON_HELD_CARDS:
                on_held_jokers = replace_deep_copy_joker(on_held_jokers, joker)

            case JokerTriggers.AFTER_HAND:
                after_hand_jokers = replace_deep_copy_joker(after_hand_jokers, joker)

    return scoring_played, on_play_jokers, on_held_jokers, after_hand_jokers


def calculate_score(
    hand_scoring: HandScoring,
    joker_plan: JokerPlan,
    game_state: GameState,
    condition_args: JokerScoringConditions,
) -> BestHand:
    joker_cache = JOKER_CACHE

    hand_stats = hand_scoring.hand_stats
    scoring_held = hand_scoring.scored_held
    scoring_played = hand_scoring.scored_played

    played_joker_retriggers = joker_plan.played_retrigger
    on_play_jokers = joker_plan.on_played
    on_held_jokers = joker_plan.on_held
    after_hand_jokers = joker_plan.after_hand

    lucky_triggers: int = 0

    best_hand = BestHand(
        chips=hand_stats.chips,
        worst_case_mult=hand_stats.mult,
        avg_case_mult=hand_stats.mult,
        best_case_mult=hand_stats.mult,
    )

    update_joker_len = len(joker_plan.update_jokers)
    if update_joker_len > 0:
        scoring_played, on_play_jokers, on_held_jokers, after_hand_jokers = (
            apply_joker_update(
                joker_plan,
                hand_scoring,
                on_play_jokers,
                on_held_jokers,
                after_hand_jokers,
            )
        )

    condition_args.scoring_held = scoring_held
    condition_args.scoring_played = scoring_played
    condition_args.unscoring_held = hand_scoring.unscored_held
    condition_args.face_card_count = -1

    mult_scoring_order: list[tuple[int, int | float, float]] = []
    mult_start_index = 0

    for i, card in enumerate(scoring_played):
        condition_args.card = card
        condition_args.card_index = i
        condition_args.face_card_count += 1 if card.is_facecard else 0

        trigger = card.trigger
        for joker in played_joker_retriggers:
            trigger += calculate_joker_retrigger(joker, condition_args)

        if card.enhancement == Enhancement.LUCKY:
            lucky_triggers += 1

        best_hand.chips += card.chips * trigger
        add_to_order(
            card.add_mult,
            card.play_x_mult,
            card.mult_prob,
            mult_scoring_order,
            card.enhancement == Enhancement.LUCKY,
        )

        card_cache = joker_cache.get(card.card_id)
        if card_cache is None:
            card_cache = joker_cache[card.card_id] = {}

        for joker in on_play_jokers:
            joker_key = joker.background_image
            score = card_cache.get(joker_key)

            if joker_key in NO_CACHE_JOKERS:
                score = calculate_joker_scoring(joker, condition_args)
            elif score is None:
                score = calculate_joker_scoring(joker, condition_args)
                card_cache[joker_key] = score

            j_chips, j_add_mult, j_x_mult = score

            best_hand.chips += j_chips * trigger
            add_to_order(j_add_mult, j_x_mult, joker.prob, mult_scoring_order)

        mult_start_index = extend_order(mult_start_index, trigger, mult_scoring_order)

    for card in scoring_held:
        condition_args.card = card
        add_to_order(0, card.hand_x_mult, 1.0, mult_scoring_order)

        for joker in on_held_jokers:
            _, j_add_mult, j_x_mult = calculate_joker_scoring(joker, condition_args)
            add_to_order(j_add_mult, j_x_mult, joker.prob, mult_scoring_order)

        trigger = card.trigger + len(joker_plan.held_retrigger)
        mult_start_index = extend_order(mult_start_index, trigger, mult_scoring_order)

    for joker in after_hand_jokers:
        j_chips, j_add_mult, j_x_mult = 0, 0, 1
        if joker.background_image == JokersName.LUCKY_CAT:
            j_x_mult = 0.25 * lucky_triggers * 2
        else:
            j_chips, j_add_mult, j_x_mult = calculate_joker_scoring(joker, condition_args)

        best_hand.chips += j_chips
        add_to_order(j_add_mult, j_x_mult, joker.prob, mult_scoring_order)

    best_mult = best_hand.best_case_mult
    avg_mult = best_hand.avg_case_mult
    worst_mult = best_hand.worst_case_mult

    for operator, val, prob in mult_scoring_order:
        if prob < 1:
            prob_value = prob * game_state.probabily_mult
            best_value = val
            avg_value = prob_value * val + (1 - prob_value) * operator
            worst_value = operator
        else:
            best_value = avg_value = worst_value = val

        if operator == ADD:
            best_mult += best_value
            avg_mult += avg_value
            worst_mult += worst_value
        else:
            best_mult *= best_value
            avg_mult *= avg_value
            worst_mult *= worst_value

    best_hand.best_case_mult = best_mult
    best_hand.avg_case_mult = avg_mult
    best_hand.worst_case_mult = worst_mult

    return best_hand


@overload
def get_best_scoring_hand(
    cards: list[Card],
    jokers: list[Joker],
    game_state: GameState,
    scoring_type: str = "avg",
    *,
    test: Literal[False] = False,
) -> FinalScoringResults: ...


@overload
def get_best_scoring_hand(
    cards: list[Card],
    jokers: list[Joker],
    game_state: GameState,
    scoring_type: str = "avg",
    *,
    test: Literal[True],
) -> tuple[FinalScoringResults, int]: ...


def get_best_scoring_hand(
    cards: list[Card],
    jokers: list[Joker],
    game_state: GameState,
    scoring_type: str = "avg",
    *,
    test: bool = False,
) -> FinalScoringResults | tuple[FinalScoringResults, int]:
    hand_cache = generate_scoring_hand_combinations(cards, jokers)
    all_possible_jokers = generate_scoring_jokers_combinations(jokers)
    # For early game when we have no jokers
    if len(all_possible_jokers) == 0:
        all_possible_jokers = [[]]

    joker_plan_cache = [
        build_joker_plan(joker_lineup) for joker_lineup in all_possible_jokers
    ]

    score_area = 0
    if "avg" in scoring_type:
        score_area = 0
    elif "worst" in scoring_type:
        score_area = 1
    else:
        score_area = 2

    condition_args = JokerScoringConditions()

    score_to_beat = float("-inf")
    final_results = FinalScoringResults()

    for hand_scoring in hand_cache:
        for joker_plan in joker_plan_cache:
            a = 5
            score = calculate_score(
                hand_scoring, joker_plan, game_state, condition_args
            )
            highest_score = 0.0

            match score_area:
                case 0:
                    highest_score = score.chips * score.avg_case_mult

                case 1:
                    highest_score = score.chips * score.worst_case_mult

                case _:
                    highest_score = score.chips * score.best_case_mult

            if highest_score > score_to_beat:
                score_to_beat = highest_score
                final_results.best_hand = score
                final_results.hand_scoring = hand_scoring
                final_results.joker_plan = joker_plan

    if test:
        return final_results, len(hand_cache) * len(joker_plan_cache)

    return final_results
