import time
from itertools import permutations

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

JOKER_CACHE: dict[Card, dict[Joker, tuple[int, int, float]]] = {}


def get_per_card_jokers(jokers: list[Joker]) -> list[Joker]:
    return [
        joker
        for joker in jokers
        if joker.scoring.trigger == JokerTriggers.ON_PLAYED_CARDS
    ]


def get_after_hand_jokers(jokers: list[Joker]) -> list[Joker]:
    return [
        joker for joker in jokers if joker.scoring.trigger == JokerTriggers.AFTER_HAND
    ]


def filter_steel(hand_steel_cards: list[Card], steel_cards: list[Card]) -> list[int]:
    min_length = min(len(hand_steel_cards), len(steel_cards))
    skip_index = []
    for i in range(min_length):
        if hand_steel_cards[i] == steel_cards[i]:
            skip_index.append(i)

    return skip_index


def filter_cards(played_cards: list[Card], cards_in_hand: list[Card]) -> list[Card]:
    hand_card_count = bucket_id(cards_in_hand)
    played_card_count = bucket_id(played_cards)

    cards_held_in_hand: list[Card] = []
    for key, curr_cards in hand_card_count.items():
        hand_cards = played_card_count.get(key, [])

        # if we played this card
        if len(curr_cards) == len(hand_cards):
            continue

        diff = len(curr_cards) - len(hand_cards)
        cards_held_in_hand.extend(curr_cards[:diff])

    return cards_held_in_hand


def generate_possible_jokers(jokers: list[Joker]) -> list[list[Joker]]:
    return [list(val) for val in permutations(jokers, len(jokers))]


def calculate_score(
    hand: list[Card],
    after_hand_joker: list[Joker],
    per_hand_jokers: list[Joker],
) -> float:
    hand_stats = get_hand_type(hand)
    chips, mult = hand_stats.chips, hand_stats.mult
    condition_args: dict = {}

    condition_args["hand"] = hand

    for card in hand:
        condition_args["card"] = card
        # if card not in JOKER_CACHE:
            # JOKER_CACHE[]

        trigger = card.trigger
        for _ in range(trigger, 0, -1):
            chips += card.chips
            mult += card.add_mult
            mult *= card.play_x_mult

            for joker in per_hand_jokers:
                joker_chips, joker_add_mult, joker_x_mult = calculate_joker_scoring(
                    joker, condition_args
                )
                chips += joker_chips
                mult += joker_add_mult
                mult *= joker_x_mult

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
        after_hand_jokers = get_after_hand_jokers(joker_linup)
        per_hand_jokers = get_per_card_jokers(joker_linup)
        for hand in all_possible_hands:
            if len(hand) == 5 and hand[0].rank == Rank.KING and hand[1  ].rank == Rank.QUEEN and hand[2].rank == Rank.FOUR and hand[3].rank == Rank.FOUR and hand[4].rank == Rank.FOUR :
                a = 5
            score = calculate_score(hand, after_hand_jokers, per_hand_jokers)
            if score > best_score:
                best_score = score
                best_hand = hand
                best_joker = joker_linup

    print(f"Iterated over {len(all_possible_hands) * len(all_possible_jokers)}")
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
        Card(Rank.FOUR, Suit.CLUBS, Enhancement.LUCKY, Seal.RED, Edition.POLYCHROME),
        Card(Rank.FOUR, Suit.CLUBS, Enhancement.LUCKY, Seal.RED, Edition.POLYCHROME),
        Card(Rank.FOUR, Suit.CLUBS, Enhancement.LUCKY, Seal.RED, Edition.POLYCHROME),
    ]

    ancient_jokers = Joker(JokersName.ANCIENT_JOKER)
    ancient_jokers.req = {"suit": Suit.CLUBS}
    jokers = [
        Joker(JokersName.ZANY_JOKER),
        Joker(JokersName.WILY_JOKER),
        Joker(JokersName.TRIBOULET_BACKGROUND),
        ancient_jokers,
    ]

    # hand = Hand.random_hand(8)
    # cards = hand.cards

    start_time = time.perf_counter()
    get_best_scoring_hand(cards, jokers)
    end_time = time.perf_counter()

    print(f"The time taken to calcualte the best was {end_time - start_time}")
