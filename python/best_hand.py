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


def calculate_score(
    hand_scoring: HandScoring,
    joker_plan: JokerPlan,
    condition_args: JokerScoringConditions,
) -> float:
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

    for i, card in enumerate(scoring_played):
        condition_args.card = card
        condition_args.card_index = i

        card_cache = joker_cache.get(card.card_id)
        if card_cache is None:
            card_cache = joker_cache[card.card_id] = {}

        trigger = card.trigger
        for joker in joker_plan.played_retrigger:
            trigger += calculate_joker_retrigger(joker, condition_args)

        for _ in range(trigger, 0, -1):
            best_hand.chips += card.chips
            best_hand.best_case_mult += card.add_mult
            best_hand.best_case_mult *= card.play_x_mult

            for joker in joker_plan.on_played:
                joker_key = joker.background_image
                cached = card_cache.get(joker_key)

                if cached is None:
                    cached = calculate_joker_scoring(joker, condition_args)
                    card_cache[joker_key] = cached

                j_chips, j_add_mult, j_x_mult = cached

                best_hand.chips += j_chips
                best_hand.best_case_mult += j_add_mult
                best_hand.best_case_mult *= j_x_mult

    for card in scoring_held:
        condition_args.card = card

        trigger = card.trigger
        # there is only mime which retriggers once and if we copy that is just mime + copys
        trigger += len(joker_plan.held_retrigger)

        for _ in range(trigger, 0, -1):
            best_hand.best_case_mult *= card.hand_x_mult

            for joker in joker_plan.on_held:
                _, j_add_mult, j_x_mult = calculate_joker_scoring(joker, condition_args)

                best_hand.best_case_mult += j_add_mult
                best_hand.best_case_mult *= j_x_mult

    for joker in joker_plan.after_hand:
        j_chips, j_add_mult, j_x_mult = calculate_joker_scoring(joker, condition_args)

        best_hand.chips += j_chips
        best_hand.best_case_mult += j_add_mult
        best_hand.best_case_mult *= j_x_mult

    return best_hand.chips * best_hand.best_case_mult


def get_best_scoring_hand(
    cards: list[Card], jokers: list[Joker], do_print: bool = False
) -> None:
    hand_cache = generate_scoring_hand_combinations(cards, jokers)
    all_possible_jokers = generate_scoring_jokers_combinations(jokers)
    # For early game when we have no jokers
    if len(all_possible_jokers) == 0:
        all_possible_jokers = [[]]

    joker_plan_cache = [
        build_joker_plan(joker_lineup) for joker_lineup in all_possible_jokers
    ]

    condition_args = JokerScoringConditions()

    best_score = 0
    best_hand_type = -1
    best_hand = []
    best_jokers = []
    best_cards_not_played = []

    for hand_scoring in hand_cache:
        for joker_index, joker_lineup in enumerate(all_possible_jokers):
            joker_plan = joker_plan_cache[joker_index]
            score = calculate_score(hand_scoring, joker_plan, condition_args)
            if score > best_score:
                best_score = score
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
        print(f"\nWhich scored {best_score:,.2f}\n")

        print("The jokers order was:")
        for joker in best_jokers:
            print(joker)

        print("\nThe cards that were not played were:")
        for card in best_cards_not_played:
            print(card)

    return


if __name__ == "__main__":
    command = sys.argv[1] if len(sys.argv) > 1 else None

    cards = [
        Card(Rank.KING, Suit.CLUBS, Enhancement.STEEL, Seal.RED, Edition.FOIL),
        Card(Rank.KING, Suit.SPADES, Enhancement.MULT, Seal.NONE, Edition.NONE),
        Card(
            Rank.KING, Suit.HEARTS, Enhancement.GLASS, Seal.PURPLE, Edition.POLYCHROME
        ),
        Card(
            Rank.QUEEN, Suit.SPADES, Enhancement.BONUS, Seal.GOLD, Edition.HOLOGRAPHIC
        ),
        Card(Rank.QUEEN, Suit.CLUBS, Enhancement.STONE, Seal.NONE, Edition.NONE),
        Card(Rank.JACK, Suit.SPADES, Enhancement.LUCKY, Seal.BLUE, Edition.NONE),
        Card(Rank.JACK, Suit.CLUBS, Enhancement.WILD, Seal.NONE, Edition.NONE),
        Card(Rank.TEN, Suit.SPADES, Enhancement.STEEL, Seal.NONE, Edition.NONE),
    ]

    jokers = [
        Joker.build(JokersName.BLACKBOARD),
        Joker.build(JokersName.BLUEPRINT),
        Joker.build(JokersName.RAISED_FIST),
        Joker.build(JokersName.THE_TRIO),
        Joker.build(JokersName.BARON),
        Joker.build(JokersName.MIME),
        Joker.build(JokersName.ONYX_AGATE),
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

    count = 1 if command is None else 1_000
    start_time = time.perf_counter()

    if count == 1:
        get_best_scoring_hand(cards, jokers, True)
    else:
        for _ in tqdm(range(count)):
            get_best_scoring_hand(cards, jokers, False)

    end_time = time.perf_counter()
    elapsed = end_time - start_time

    print(f"The time taken to calculate the best was {format_duration(elapsed)}")
