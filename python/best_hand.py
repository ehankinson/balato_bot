import time
from collections import Counter

from calculation.joker_generation import (
    generate_possible_jokers,
    get_retrigger_jokers,
    get_trigger_jokers,
)
from calculation.joker_retrigger import calculate_joker_retrigger
from calculation.joker_scoring import calculate_joker_scoring
from calculation.poker_eval import get_hand_type
from calculation.poker_generation import generate_playable_hands
from core.enums import (
    Edition,
    Enhancement,
    JokersName,
    JokerTriggers,
    Rank,
    Seal,
    Suit,
)
from core.models import Card, Joker

JOKER_CACHE: dict[int, dict[int, tuple[int, int, float]]] = {}


def filter_cards(
    jokers: list[Joker], main_cards: list[Card], filter_cards: list[Card]
) -> list[Card]:
    cards_not_played = list((Counter(main_cards) - Counter(filter_cards)).elements())

    important_cards = []
    if any(joker.background_image == JokersName.RAISED_FIST for joker in jokers):
        lowest_rank = min(cards_not_played, key=lambda card: card.rank)
        important_cards.append(lowest_rank)

    if any(joker.background_image == JokersName.SHOOT_THE_MOON for joker in jokers):
        queen_cards = [card for card in cards_not_played if card.rank == Rank.QUEEN]
        important_cards.extend(queen_cards)

    if any(joker.background_image == JokersName.BARON for joker in jokers):
        king_cards = [card for card in cards_not_played if card.rank == Rank.KING]
        important_cards.extend(king_cards)

    steel_cards = [
        card
        for card in cards_not_played
        if card not in important_cards and card.enhancement == Enhancement.STEEL
    ]
    important_cards.extend(steel_cards)

    important_cards = sorted(important_cards, key=lambda card: card.rank)
    return important_cards


def add_to_joker_cache(card_id: int, condition_args: dict) -> tuple[int, int, float]:
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
    cards_not_played: list[Card],
    on_held_card_jokers: list[Joker],
    on_played_card_jokers: list[Joker],
    after_hand_joker: list[Joker],
    retrigger_jokers: list[Joker],
) -> float:
    hand_stats = get_hand_type(hand)
    chips, mult = hand_stats.chips, hand_stats.mult
    condition_args: dict = {}
    condition_args["cards_not_played"] = cards_not_played
    condition_args["hand"] = hand

    played_card_retrigger_jokers = [
        joker for joker in retrigger_jokers if joker.background_image != JokersName.MIME
    ]
    helded_card_retrigger_jokers = [
        joker for joker in retrigger_jokers if joker.background_image == JokersName.MIME
    ]
    for i, card in enumerate(hand):
        condition_args["card"] = card
        condition_args["card_pos"] = i
        if card.card_id not in JOKER_CACHE:
            JOKER_CACHE[card.card_id] = {}

        trigger = card.trigger
        for joker in played_card_retrigger_jokers:
            trigger += calculate_joker_retrigger(joker, condition_args)

        for _ in range(trigger, 0, -1):
            chips += card.chips
            mult += card.add_mult
            mult *= card.play_x_mult

            for joker in on_played_card_jokers:
                j_chips, j_add_mult, j_x_mult = add_to_joker_cache(
                    card.card_id, condition_args
                )

                chips += j_chips
                mult += j_add_mult
                mult *= j_x_mult

    for card in cards_not_played:
        condition_args["card"] = card

        trigger = card.trigger
        # there is only mime which retriggers once and if we copy that is just mime + copys
        trigger += len(helded_card_retrigger_jokers)

        for _ in range(trigger, 0, -1):
            mult *= card.hand_x_mult

            for joker in on_held_card_jokers:
                _, j_add_mult, j_x_mult = calculate_joker_scoring(joker, condition_args)

                mult += j_add_mult
                mult *= j_x_mult

    for joker in after_hand_joker:
        joker_chips, joker_add_mult, joker_x_mutl = calculate_joker_scoring(
            joker, condition_args
        )
        chips += joker_chips
        mult += joker_add_mult
        mult *= joker_x_mutl

    return chips * mult


def get_best_scoring_hand(cards: list[Card], jokers: list[Joker]) -> None:
    all_possible_hands = generate_playable_hands(cards)
    all_possible_jokers = generate_possible_jokers(jokers)

    best_score = 0
    best_hand = []
    best_joker = []

    for joker_linup in all_possible_jokers:
        retrigger_jokers = get_retrigger_jokers(joker_linup)
        after_hand_jokers = get_trigger_jokers(joker_linup, JokerTriggers.AFTER_HAND)
        on_held_cards_jokers = get_trigger_jokers(
            joker_linup, JokerTriggers.ON_HELD_CARDS
        )
        on_played_cards_jokers = get_trigger_jokers(
            joker_linup, JokerTriggers.ON_PLAYED_CARDS
        )

        for hand in all_possible_hands:
            card_not_played = filter_cards(joker_linup, cards, hand)
            score = calculate_score(
                hand,
                card_not_played,
                on_held_cards_jokers,
                on_played_cards_jokers,
                after_hand_jokers,
                retrigger_jokers,
            )
            if score > best_score:
                best_score = score
                best_hand = hand
                best_joker = joker_linup

    print(f"Iterated over {len(all_possible_hands) * len(all_possible_jokers):,.0f}")
    print(f"{best_score:,.2f}")
    print(best_joker)
    for card in best_hand:
        print(card)

    return


if __name__ == "__main__":
    cards = [
        Card(Rank.ACE, Suit.CLUBS, Enhancement.NONE, Seal.NONE, Edition.NONE),
        Card(Rank.KING, Suit.CLUBS, Enhancement.STEEL, Seal.RED, Edition.NONE),
        Card(Rank.QUEEN, Suit.CLUBS, Enhancement.NONE, Seal.NONE, Edition.NONE),
        Card(Rank.JACK, Suit.CLUBS, Enhancement.NONE, Seal.NONE, Edition.NONE),
        Card(Rank.TEN, Suit.CLUBS, Enhancement.MULT, Seal.NONE, Edition.NONE),
        Card(Rank.FOUR, Suit.CLUBS, Enhancement.MULT, Seal.RED, Edition.POLYCHROME),
        Card(Rank.FOUR, Suit.CLUBS, Enhancement.MULT, Seal.RED, Edition.POLYCHROME),
        Card(Rank.FOUR, Suit.CLUBS, Enhancement.MULT, Seal.RED, Edition.POLYCHROME),
    ]

    joker = Joker(JokersName.RAISED_FIST)
    # joker.scoring.x_mult = 3
    jokers = [joker, Joker(JokersName.MIME)]

    # hand = Hand.random_hand(8)
    # cards = hand.cards

    start_time = time.perf_counter()
    # for _ in range(10_000):
    get_best_scoring_hand(cards, jokers)
    end_time = time.perf_counter()

    print(f"The time taken to calcualte the best was {end_time - start_time}")
