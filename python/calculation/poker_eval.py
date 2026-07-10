from calculation.util import bucket_rank, unique_cards
from config.poker_hands import HAND_STATS
from core.enums import PokerHand
from core.hand_stats import HandStats
from core.models import Card


def is_flush(cards: list[Card]) -> bool:
    initial_suit = cards[0].suit
    return all(card.is_any_suit or card.suit == initial_suit for card in cards)


def is_straight(cards: list[Card]) -> bool:
    cards = sorted(cards, key=lambda x: x.rank, reverse=True)
    for i in range(1, len(cards)):
        diff = cards[i - 1].rank - cards[i].rank
        if i == 1 and diff == 9:
            continue
            
        if diff != 1:
            return False

    return True


def contains_2_pair(cards: list[Card]) -> bool:
    bucket = bucket_rank(cards)
    count = 0
    for value in bucket.values():
        if len(value) >= 2:
            count += 1

    return count > 1


def is_same_rank(cards: list[Card]) -> bool:
    initial_rank = cards[0].rank
    return all(card.rank == initial_rank for card in cards)


def contain_n_of_a_kind(n: int, cards: list[Card]):
    bucket = bucket_rank(cards)
    return any(len(val) >= n for val in bucket.values())


def get_hand_type(hand: list[Card]) -> HandStats:
    hand_len = len(hand)
    if hand_len > 5 or hand_len < 1:
        raise ValueError(f"Current Hand '{hand}' is not possible")

    unique_rank_count = unique_cards(hand)

    all_same_rank = unique_rank_count == 1
    has_two_ranks = unique_rank_count == 2

    match hand_len:
        case 5:
            flush = is_flush(hand)
            straight = is_straight(list(hand))

            checks = [
                (flush and all_same_rank, PokerHand.FLUSH_FIVE),
                (flush and has_two_ranks and not straight, PokerHand.FLUSH_HOUSE),
                (all_same_rank, PokerHand.FIVE_OF_A_KIND),
                (flush and straight, PokerHand.STRAIGHT_FLUSH),
                (has_two_ranks and not flush and not straight, PokerHand.FULL_HOUSE),
                (flush and not straight, PokerHand.FLUSH),
                (straight, PokerHand.STRAIGHT),
            ]

            for condition, hand_type in checks:
                if condition:
                    return HAND_STATS[hand_type]

        case 4:
            return HAND_STATS[
                PokerHand.FOUR_OF_A_KIND if all_same_rank else PokerHand.TWO_PAIR
            ]

        case _:
            return HAND_STATS[PokerHand(hand_len)]

    raise ValueError(
        f"The current hand: {hand} does not have a possible hand, This hand is impossible"
    )
