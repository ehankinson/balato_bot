import sys
import time

from tqdm import tqdm

from calculation.joker_generation import generate_scoring_jokers_combinations
from calculation.joker_retrigger import calculate_joker_retrigger
from calculation.joker_scoring import calculate_joker_scoring
from calculation.poker_generation import generate_scoring_hand_combinations
from core.enums import (
    Edition,
    Enhancement,
    JokersName,
    JokerTriggers,
    PokerHand,
    Rank,
    Seal,
    Suit,
)
from core.models import (
    BestHand,
    Card,
    HandScoring,
    Joker,
    JokerPlan,
    JokerRetrigger,
    JokerScoring,
    JokerScoringConditions,
)

JOKER_CACHE: dict[int, dict[int, tuple[int, int, float]]] = {}
ADD = 0
MULT = 1


def build_joker_plan(jokers: list[Joker]) -> JokerPlan:
    plan = JokerPlan([], [], [], [], [])

    for joker in jokers:
        if isinstance(joker, JokerScoring):
            match joker.trigger:
                case JokerTriggers.ON_PLAYED_CARDS:
                    plan.on_played.append(joker)
                case JokerTriggers.ON_HELD_CARDS:
                    plan.on_held.append(joker)
                case JokerTriggers.AFTER_HAND:
                    plan.after_hand.append(joker)

        elif isinstance(joker, JokerRetrigger):
            if joker.trigger == JokerTriggers.ON_PLAYED_CARDS:
                plan.played_retrigger.append(joker)
            else:
                plan.held_retrigger.append(joker)

    return plan


def add_to_order(
    add_mult: int,
    x_mult: float,
    prob: float,
    mult_scoring_order: list[tuple[int, int | float, float]],
) -> None:
    if add_mult > ADD:
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


def calculate_score(
    hand_scoring: HandScoring,
    joker_plan: JokerPlan,
    condition_args: JokerScoringConditions,
) -> BestHand:
    joker_cache = JOKER_CACHE

    hand_stats = hand_scoring.hand_stats
    scoring_held = hand_scoring.scored_held
    scoring_played = hand_scoring.scored_played

    condition_args.scoring_held = scoring_held
    condition_args.scoring_played = scoring_played
    condition_args.unscoring_held = hand_scoring.unscored_held

    best_hand = BestHand(
        chips=hand_stats.chips,
        worst_case_mult=hand_stats.mult,
        avg_case_mult=hand_stats.mult,
        best_case_mult=hand_stats.mult,
    )

    mult_scoring_order: list[tuple[int, int | float, float]] = []
    mult_start_index = 0

    for i, card in enumerate(scoring_played):
        condition_args.card = card
        condition_args.card_index = i
        condition_args.face_card_count += 1 if card.is_facecard else 0

        trigger = card.trigger
        for joker in joker_plan.played_retrigger:
            trigger += calculate_joker_retrigger(joker, condition_args)

        best_hand.chips += card.chips * trigger
        add_to_order(
            card.add_mult, card.play_x_mult, card.mult_prob, mult_scoring_order
        )

        card_cache = joker_cache.get(card.card_id)
        if card_cache is None:
            card_cache = joker_cache[card.card_id] = {}

        for joker in joker_plan.on_played:
            joker_key = joker.background_image
            cached = card_cache.get(joker_key)

            if cached is None:
                cached = calculate_joker_scoring(joker, condition_args)
                card_cache[joker_key] = cached

            j_chips, j_add_mult, j_x_mult = cached

            best_hand.chips += j_chips * trigger
            add_to_order(j_add_mult, j_x_mult, joker.prob, mult_scoring_order)

        mult_start_index = extend_order(mult_start_index, trigger, mult_scoring_order)

    for card in scoring_held:
        condition_args.card = card
        add_to_order(0, card.hand_x_mult, 1.0, mult_scoring_order)

        for joker in joker_plan.on_held:
            _, j_add_mult, j_x_mult = calculate_joker_scoring(joker, condition_args)
            add_to_order(j_add_mult, j_x_mult, joker.prob, mult_scoring_order)

        trigger = card.trigger + len(joker_plan.held_retrigger)
        mult_start_index = extend_order(mult_start_index, trigger, mult_scoring_order)

    for joker in joker_plan.after_hand:
        j_chips, j_add_mult, j_x_mult = calculate_joker_scoring(joker, condition_args)

        best_hand.chips += j_chips
        add_to_order(j_add_mult, j_x_mult, joker.prob, mult_scoring_order)

    best_mult = best_hand.best_case_mult
    avg_mult = best_hand.avg_case_mult
    worst_mult = best_hand.worst_case_mult

    for operator, val, prob in mult_scoring_order:
        if prob < 1:
            best_value = val
            avg_value = prob * val + (1 - prob) * operator
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


def get_best_scoring_hand(
    cards: list[Card],
    jokers: list[Joker],
    do_print: bool = False,
    scoring_type: str = "",
) -> None:
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

    score_to_beat = 0
    best_score = 0
    avg_score = 0
    worst_score = 0
    best_hand_type = -1
    best_hand = []
    best_jokers = []
    best_cards_not_played = []

    for hand_scoring in hand_cache:
        for joker_index, joker_lineup in enumerate(all_possible_jokers):
            joker_plan = joker_plan_cache[joker_index]

            score = calculate_score(hand_scoring, joker_plan, condition_args)
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
                best_score = score.chips * score.best_case_mult
                avg_score = score.chips * score.avg_case_mult
                worst_score = score.chips * score.worst_case_mult
                best_hand = hand_scoring.scored_played + hand_scoring.unscored_played
                best_jokers = joker_lineup
                best_hand_type = hand_scoring.hand_stats.name
                best_cards_not_played = (
                    hand_scoring.scored_held + hand_scoring.unscored_held
                )

    if do_print:
        print(
            f"Iterated over {len(hand_cache) * len(all_possible_jokers):,.0f} possible hands + joker combinations"
        )
        print(f"The hand played was a {PokerHand(best_hand_type).name}\n")
        for card in best_hand:
            print(card)

        worst_string = (
            f"{worst_score:e}"
            if worst_score > 99_999_999_999
            else f"{worst_score:,.2f}"
        )
        avg_string = (
            f"{avg_score:e}" if avg_score > 99_999_999_999 else f"{avg_score:,.2f}"
        )
        best_string = (
            f"{best_score:e}" if best_score > 99_999_999_999 else f"{best_score:,.2f}"
        )

        if worst_score != avg_score:
            print(
                f"\nWhich has a range of score from {worst_string} - {best_string}\nLikely ending up with {avg_string}\n"
            )
        else:
            print(f"\nWhich scored {best_string}\n")

        print("The jokers order was:")
        for joker in best_jokers:
            print(joker)

        print("\nThe cards that were not played were:")
        for card in best_cards_not_played:
            print(card)

        print()

    return


if __name__ == "__main__":
    command = sys.argv[1] if len(sys.argv) > 1 else None

    cards = [
        # Played hand: keep this simple, just enough to score something.
        # These are not where the huge score comes from.
        Card(Rank.ACE, Suit.SPADES, Enhancement.NONE, Seal.NONE, Edition.NONE),
    
        # Held cards: these are the actual score engine.
        Card(Rank.KING, Suit.HEARTS, Enhancement.STEEL, Seal.RED, Edition.POLYCHROME),
        Card(Rank.KING, Suit.DIAMONDS, Enhancement.STEEL, Seal.RED, Edition.POLYCHROME),
        Card(Rank.KING, Suit.CLUBS, Enhancement.STEEL, Seal.RED, Edition.POLYCHROME),
        Card(Rank.KING, Suit.SPADES, Enhancement.STEEL, Seal.RED, Edition.POLYCHROME),
        Card(Rank.KING, Suit.HEARTS, Enhancement.STEEL, Seal.RED, Edition.POLYCHROME),
        Card(Rank.KING, Suit.DIAMONDS, Enhancement.STEEL, Seal.RED, Edition.POLYCHROME),
        Card(Rank.KING, Suit.CLUBS, Enhancement.STEEL, Seal.RED, Edition.POLYCHROME),
    ]

    jokers = [
        # Copy Baron if your sim supports Blueprint copying the next compatible Joker.
        # Joker.build(JokersName.BLUEPRINT),
        Joker.build(JokersName.BARON),
    
        # Brainstorm usually copies the leftmost Joker.
        # Depending on your sim, this may copy Blueprint, which may effectively copy Baron again.
        Joker.build(JokersName.BRAINSTORM),
    
        # Mime retriggers held-card effects.
        Joker.build(JokersName.MIME),
    
        # Extra scaling XMult.
        Joker.build(JokersName.TRIBOULET_BACKGROUND),
    
        # More XMult if face cards are involved / if your sim applies it correctly.
        Joker.build(JokersName.PHOTOGRAPH),
    
        # Optional extra XMult source.
        Joker.build(JokersName.SOCK_AND_BUSKIN),
    ]

    def format_duration(seconds: float) -> str:
        if seconds < 1e-6:
            return f"{seconds * 1e9:.2f}ns"
        elif seconds < 1e-3:
            return f"{seconds * 1e6:.2f}µs"
        elif seconds < 1:
            return f"{seconds * 1e3:.2f}ms"
        else:
            return f"{seconds:.2f}s"

    count = 1
    if command is not None and command == "time":
        count = 1_000

    start_time = time.perf_counter()

    if count == 1:
        get_best_scoring_hand(cards, jokers, True, command or "")
    else:
        for _ in tqdm(range(count)):
            get_best_scoring_hand(cards, jokers, False)

    end_time = time.perf_counter()
    elapsed = end_time - start_time

    print(f"The time taken to calculate the best was {format_duration(elapsed)}")
