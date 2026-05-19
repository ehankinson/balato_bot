import sys
import time

from tqdm import tqdm

from calculation.joker_generation import generate_possible_jokers
from calculation.joker_retrigger import calculate_joker_retrigger
from calculation.joker_scoring import calculate_joker_scoring
from calculation.poker_eval import get_hand_type
from calculation.poker_generation import generate_playable_hands
from calculation.util import blackboard_helper
from core.enums import (
    Edition,
    Enhancement,
    JokersName,
    JokerTriggers,
    Rank,
    Seal,
    Suit,
)
from core.hand_stats import HandStats
from core.models import Card, CardBucket, Joker, JokerPlan

JOKER_CACHE: dict[int, dict[int, tuple[int, int, float]]] = {}


def filter_cards(
    main_bucket: dict[int, CardBucket], filter_cards: list[Card]
) -> list[Card]:
    for card in filter_cards:
        main_bucket[card.card_id].count -= 1

    cards_not_played = []
    for values in main_bucket.values():
        cards_not_played.extend(values.cards[: values.count])
        values.count = len(values.cards)

    steel_cards = [card for card in cards_not_played if card.enhancement == Enhancement.STEEL]
    other_cards = [card for card in cards_not_played if card.enhancement != Enhancement.STEEL]

    other_cards = sorted(other_cards, key=lambda card: card.rank)
    steel_cards = sorted(steel_cards, key=lambda card: card.rank)
    return other_cards + steel_cards


def build_joker_plan(jokers: list[Joker]) -> JokerPlan:
    plan = JokerPlan([], [], [], [], [])

    for joker in jokers:
        if joker.scoring is not None:
            match joker.scoring.trigger:
                case JokerTriggers.ON_PLAYED_CARDS:
                    plan.on_played.append(joker)
                case JokerTriggers.ON_HELD_CARDS:
                    plan.on_held.append(joker)
                case JokerTriggers.AFTER_HAND:
                    plan.after_hand.append(joker)

        if joker.retrigger is not None:
            if joker.retrigger.trigger == JokerTriggers.ON_PLAYED_CARDS:
                plan.played_retrigger.append(joker)
            else:
                plan.held_retrigger.append(joker)

    return plan


def add_to_joker_cache(
    card_id: int,
    joker: Joker,
    condition_args: dict,
) -> tuple[int, int, float]:
    if joker.background_image not in JOKER_CACHE[card_id]:
        joker_chips, joker_add_mult, joker_x_mult = calculate_joker_scoring(
            joker, condition_args
        )
        JOKER_CACHE[card_id][joker.background_image] = (
            joker_chips,
            joker_add_mult,
            joker_x_mult,
        )

    return JOKER_CACHE[card_id][joker.background_image]


def calculate_playing_card_score(
    chips: int,
    mult: float,
    card: Card,
    retrigger_jokers: list[Joker],
    condition_args: dict,
) -> None:
    trigger = card.trigger
    for joker in retrigger_jokers:
        trigger += calculate_joker_retrigger(joker, condition_args)

    for _ in range(trigger, 0, -1):
        chips += card.chips
        mult += card.add_mult
        mult *= card.play_x_mult


def calculate_score(
    hand: list[Card],
    hand_stats: HandStats,
    cards_not_played: list[Card],
    joker_plan: JokerPlan,
) -> float:
    chips, mult = hand_stats.chips, hand_stats.mult
    condition_args = {}
    condition_args["cards_not_played"] = cards_not_played
    condition_args["hand"] = hand

    for i, card in enumerate(hand):
        condition_args["card"] = card
        condition_args["card_pos"] = i
        if card.card_id not in JOKER_CACHE:
            JOKER_CACHE[card.card_id] = {}

        trigger = card.trigger
        for joker in joker_plan.played_retrigger:
            trigger += calculate_joker_retrigger(joker, condition_args)

        for _ in range(trigger, 0, -1):
            chips += card.chips
            mult += card.add_mult
            mult *= card.play_x_mult

            for joker in joker_plan.on_played:
                j_chips, j_add_mult, j_x_mult = add_to_joker_cache(
                    card.card_id, joker, condition_args
                )

                chips += j_chips
                mult += j_add_mult
                mult *= j_x_mult

    for card in cards_not_played:
        condition_args["card"] = card

        trigger = card.trigger
        # there is only mime which retriggers once and if we copy that is just mime + copys
        trigger += len(joker_plan.held_retrigger)

        for _ in range(trigger, 0, -1):
            mult *= card.hand_x_mult

            for joker in joker_plan.on_held:
                _, j_add_mult, j_x_mult = calculate_joker_scoring(joker, condition_args)

                mult += j_add_mult
                mult *= j_x_mult

    for joker in joker_plan.after_hand:
        joker_chips, joker_add_mult, joker_x_mutl = calculate_joker_scoring(
            joker, condition_args
        )
        chips += joker_chips
        mult += joker_add_mult
        mult *= joker_x_mutl

    return chips * mult


def get_best_scoring_hand(
    cards: list[Card], jokers: list[Joker], do_print: bool = False
) -> None:
    all_possible_hands = generate_playable_hands(cards)
    all_possible_jokers = generate_possible_jokers(jokers)
    # For early game when we have no jokers
    if len(all_possible_jokers) == 0:
        all_possible_jokers = [[]]

    main_bucket: dict[int, CardBucket] = {}
    for card in cards:
        if card.card_id not in main_bucket:
            main_bucket[card.card_id] = CardBucket(count=0, cards=[])

        main_bucket[card.card_id].count += 1
        main_bucket[card.card_id].cards.append(card)

    hand_cache = [
        (hand, get_hand_type(hand), filter_cards(main_bucket, hand))  # jokers, main_bucket, hand))
        for hand in all_possible_hands
    ]
    joker_plan_cache = [
        build_joker_plan(joker_lineup) for joker_lineup in all_possible_jokers
    ]

    best_score = 0
    best_hand = []
    best_joker = []

    for hand, hand_stats, cards_not_played in hand_cache:
        if (
            len(hand) == 3
            and hand[0].rank == Rank.KING
            and hand[1].rank == Rank.KING
            and hand[2].rank == Rank.KING
        ):
            a = 5
        for joker_index, joker_lineup in enumerate(all_possible_jokers):
            joker_plan = joker_plan_cache[joker_index]

            score = calculate_score(hand, hand_stats, cards_not_played, joker_plan)
            if score > best_score:
                best_score = score
                best_hand = hand
                best_joker = joker_lineup

    if do_print:
        print(
            f"Iterated over {len(all_possible_hands) * len(all_possible_jokers):,.0f}"
        )
        print(f"{best_score:,.2f}")
        print(best_joker)
        for card in best_hand:
            print(card)

    return


if __name__ == "__main__":
    command = sys.argv[1] if len(sys.argv) > 1 else None
    # cards = [
    #     Card(Rank.ACE, Suit.CLUBS, Enhancement.NONE, Seal.NONE, Edition.NONE),
    #     Card(Rank.KING, Suit.CLUBS, Enhancement.NONE, Seal.NONE, Edition.NONE),
    #     Card(Rank.KING, Suit.CLUBS, Enhancement.STEEL, Seal.RED, Edition.NONE),
    #     Card(Rank.QUEEN, Suit.CLUBS, Enhancement.NONE, Seal.NONE, Edition.NONE),
    #     Card(Rank.QUEEN, Suit.CLUBS, Enhancement.LUCKY, Seal.RED, Edition.NONE),
    #     Card(Rank.QUEEN, Suit.CLUBS, Enhancement.NONE, Seal.NONE, Edition.NONE),
    #     Card(Rank.JACK, Suit.CLUBS, Enhancement.NONE, Seal.NONE, Edition.NONE),
    #     Card(Rank.TEN, Suit.CLUBS, Enhancement.MULT, Seal.NONE, Edition.NONE),
    #     Card(Rank.FOUR, Suit.CLUBS, Enhancement.MULT, Seal.RED, Edition.POLYCHROME),
    #     Card(Rank.FOUR, Suit.CLUBS, Enhancement.MULT, Seal.RED, Edition.POLYCHROME),
    #     Card(Rank.FOUR, Suit.CLUBS, Enhancement.MULT, Seal.RED, Edition.POLYCHROME),
    # ]
    #

    cards = [
        Card(Rank.KING, Suit.CLUBS, Enhancement.NONE, Seal.NONE, Edition.NONE),
        Card(Rank.KING, Suit.HEARTS, Enhancement.NONE, Seal.NONE, Edition.NONE),
        Card(Rank.KING, Suit.SPADES, Enhancement.NONE, Seal.NONE, Edition.NONE),
        Card(Rank.JACK, Suit.DIAMONDS, Enhancement.NONE, Seal.NONE, Edition.NONE),
        Card(Rank.JACK, Suit.CLUBS, Enhancement.NONE, Seal.NONE, Edition.NONE),
        Card(Rank.TWO, Suit.HEARTS, Enhancement.NONE, Seal.NONE, Edition.NONE),
    ]

    ancient = Joker(JokersName.ANCIENT_JOKER)
    ancient.req = {"suit": Suit.CLUBS}

    jokers = [
        Joker(JokersName.BLACKBOARD)
        # Joker(JokersName.BLUEPRINT),
        # Joker(JokersName.MIME),
        # Joker(JokersName.RAISED_FIST),
        # Joker(JokersName.THE_TRIO),
        # Joker(JokersName.ZANY_JOKER),
        # ancient,
        # Joker(JokersName.ONYX_AGATE),
        # Joker(JokersName.BARON),
    ]

    # hand = Hand.random_hand(8)
    # cards = hand.cards

    count = 1 if command is None else 1_000
    start_time = time.perf_counter()

    if count == 1:
        get_best_scoring_hand(cards, jokers, True)
    else:
        for _ in tqdm(range(count)):
            get_best_scoring_hand(cards, jokers, False)

    end_time = time.perf_counter()
    print(f"The time taken to calcualte the best was {end_time - start_time}")
