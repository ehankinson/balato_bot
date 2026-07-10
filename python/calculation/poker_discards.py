import math
import time

from core.enums import Rank, Suit
from core.models import Card, Deck


def print_timing(label: str, elapsed_ns: int) -> None:
    elapsed_ms = elapsed_ns / 1_000_000
    elapsed_s = elapsed_ns / 1_000_000_000
    print(f"{label:<20} {elapsed_s:>10.6f}s  {elapsed_ms:>10.3f}ms  {elapsed_ns:>12,d}ns")


def odds_for_single_value(deck: Deck, hand: list[Card], bucket: list[int], amount: int):
    total_cards = deck.total_cards
    total_draws = math.comb(total_cards, 5)
    max_iter = len(bucket)
    val_weights = {i: 0.0 for i in range(max_iter)}

    best_score = 0.0
    best_option = -1
    best_probability = 0.0

    for val in range(max_iter):
        amount_needed = amount - bucket[val]
        if amount_needed == 0:
            continue

        shift_amount, equal_val = (9, 0b11) if max_iter == 4 else (11, 0b11111)
        score = sum(
            card.score
            for card in hand
            if ((card.card_id >> shift_amount & equal_val) == val)
        )

        deck_val = deck.suits[val] if max_iter == 4 else deck.ranks[val]
        deck_val_count = len(deck_val)

        max_fetch_amount = 5 if deck_val_count > 5 else deck_val_count

        every_other_card = total_cards - deck_val_count
        good_draws = 0
        for fetched_rank in range(amount_needed, max_fetch_amount + 1):
            good_draws += math.comb(deck_val_count, fetched_rank) * math.comb(
                every_other_card, 5 - fetched_rank
            )

        total_probability = good_draws / total_draws

        score_of_cards = sum(card.score for card in deck_val) / deck_val_count
        expected_card_score = score_of_cards * amount_needed
        total_card_score = score + expected_card_score
        val_weights[val] = total_probability * total_card_score

        if total_probability > best_probability:
            best_probability = total_probability
            best_option = val
            best_score = total_card_score

        elif total_probability == best_probability and total_card_score > best_score:
            best_score = total_card_score
            best_option = val

    is_suit = max_iter == 4
    ordered_hand = sorted(
        hand, key=lambda x: val_weights[x.suit if is_suit else x.rank]
    )

    cards_to_discard = ordered_hand[:5]
    return best_option, best_probability, cards_to_discard


def odds_for_double_value(deck: Deck, hand: list[Card], bucket: list[int], amount: int):
    total_cards = deck.total_cards
    total_draws = math.comb(total_cards, 5)
    max_iter = len(bucket)
    rank_weights = { i: 0.0 for i in range(max_iter) }

    is_two_pair = amount == 2
    left_amount = amount
    right_amount = 2

    best_score = 0
    best_option = (0, 0)
    best_probability = 0.0

    for left_rank in range(max_iter):
        left_deck = deck.ranks[left_rank]
        left_deck_count = len(left_deck)

        left_amount_needed = left_amount - bucket[left_rank]
        left_score = sum(card.score for card in hand if card.rank == left_rank)
        right_vals = (
            [i for i in range(left_rank + 1, max_iter)]
            if is_two_pair
            else [i for i in range(max_iter) if i != left_rank]
        )

        for right_rank in right_vals:
            right_deck = deck.ranks[right_rank]
            right_deck_count = len(right_deck)

            right_amount_needed = right_amount - bucket[right_rank]
            if left_amount_needed + right_amount_needed == 0:
                continue
                
            right_score = sum(card.score for card in hand if card.rank == right_rank)

            left_deck_amount, right_deck_amount = left_deck_count, right_deck_count
            rest_of_deck = total_cards - (left_deck_amount + right_deck_amount)

            right_max_iter = 5 if right_deck_count > 5 else right_deck_count
            left_max_iter = 5 if left_deck_count > 5 else left_deck_count

            good_draws = 0
            if left_amount_needed == 0:
                rest_of_deck = total_cards - right_deck_amount
                good_draws = sum(
                    math.comb(right_deck_amount, right_draw)
                    * math.comb(rest_of_deck, 5 - right_draw)
                    for right_draw in range(right_amount_needed, right_max_iter)
                )
            elif right_amount_needed == 0:
                rest_of_deck = total_cards - left_deck_amount
                good_draws = sum(
                    math.comb(right_deck_amount, left_draw)
                    * math.comb(rest_of_deck, 5 - left_draw)
                    for left_draw in range(left_amount_needed, left_max_iter)
                )
            else:
                rest_of_deck = total_cards - (left_deck_amount + right_deck_amount)
                good_draws = sum(
                    math.comb(left_deck_amount, left_draw)
                    * math.comb(right_deck_amount, right_draw)
                    * math.comb(rest_of_deck, 5 - (left_draw + right_draw))
                    for right_draw in range(right_amount_needed, right_max_iter)
                    for left_draw in range(left_amount_needed, left_max_iter)
                    if right_draw + left_draw <= 5
                )
            probability = good_draws / total_draws

            key = (left_rank, right_rank)
            left_expected_score = (
                sum(card.score for card in left_deck) / left_deck_count * left_amount_needed
            )
            right_expected_score = (
                sum(card.score for card in right_deck) / right_deck_count * right_amount_needed
            )

            total_score = (
                left_score + right_score + left_expected_score + right_expected_score
            )

            weighted_score = total_score * probability
            rank_weights[left_rank] += weighted_score
            rank_weights[right_rank] += weighted_score

            if best_probability < probability:
                best_probability = probability
                best_option = key
                best_score = total_score

            elif best_probability == probability and total_score > best_score:
                best_score = total_score
                best_option = key

    ordered_hand = sorted(hand, key=lambda x: (rank_weights[x.rank]))
    cards_to_discard = ordered_hand[:5]

    return best_option, best_probability, cards_to_discard


def calculate_odds(deck: Deck, dealt_cards: list[Card]):
    suit_bucket = [0] * 4
    rank_bucket = [0] * 13
    for card in dealt_cards:
        suit_bucket[card.suit] += 1
        rank_bucket[card.rank] += 1

    total_start = time.perf_counter_ns()

    pair_start = time.perf_counter_ns()
    pair_val, pair_prob, pair_discards = odds_for_single_value(
        deck, dealt_cards, rank_bucket, 2
    )
    pair_end = time.perf_counter_ns()
    print_timing("pair odds", pair_end - pair_start)

    three_start = time.perf_counter_ns()
    three_val, three_prob, three_discards = odds_for_single_value(
        deck, dealt_cards, rank_bucket, 3
    )
    three_end = time.perf_counter_ns()
    print_timing("three of a kind", three_end - three_start)

    four_start = time.perf_counter_ns()
    four_val, four_prob, four_discards = odds_for_single_value(
        deck, dealt_cards, rank_bucket, 4
    )
    four_end = time.perf_counter_ns()
    print_timing("four of a kind", four_end - four_start)

    five_start = time.perf_counter_ns()
    five_val, five_prob, five_discards = odds_for_single_value(
        deck, dealt_cards, rank_bucket, 5
    )
    five_end = time.perf_counter_ns()
    print_timing("five of a kind", five_end - five_start)

    flush_start = time.perf_counter_ns()
    flush_val, flush_prob, flush_discards = odds_for_single_value(
        deck, dealt_cards, suit_bucket, 5
    )
    flush_end = time.perf_counter_ns()
    print_timing("flush odds", flush_end - flush_start)

    two_pair_start = time.perf_counter_ns()
    two_pair_val, two_pair_prob, two_pair_discards = odds_for_double_value(
        deck, dealt_cards, rank_bucket, 2
    )
    two_pair_end = time.perf_counter_ns()
    print_timing("two pair odds", two_pair_end - two_pair_start)

    full_house_start = time.perf_counter_ns()
    full_house_val, full_house_prob, full_house_discards = odds_for_double_value(
        deck, dealt_cards, rank_bucket, 3
    )
    full_house_end = time.perf_counter_ns()
    print_timing("full house odds", full_house_end - full_house_start)

    total_end = time.perf_counter_ns()
    print_timing("total odds time", total_end - total_start)
    # flush_discards, flush_prob, flush_val = flush_odds(deck, dealt_cards, suit_bucket)


if __name__ == "__main__":
    deck = Deck()

    hand = [
        Card(
            rank=Rank.ACE,
            suit=Suit.DIAMONDS,
        ),
        Card(
            rank=Rank.QUEEN,
            suit=Suit.HEARTS,
        ),
        Card(
            rank=Rank.TEN,
            suit=Suit.DIAMONDS,
        ),
        Card(
            rank=Rank.EIGHT,
            suit=Suit.HEARTS,
        ),
        Card(
            rank=Rank.SEVEN,
            suit=Suit.HEARTS,
        ),
        Card(
            rank=Rank.SIX,
            suit=Suit.CLUBS,
        ),
        Card(
            rank=Rank.SIX,
            suit=Suit.DIAMONDS,
        ),
        Card(
            rank=Rank.FIVE,
            suit=Suit.CLUBS,
        ),
    ]

    deck.filter(hand)
    calculate_odds(deck, hand)
