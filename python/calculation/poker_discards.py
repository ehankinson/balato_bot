import heapq
import math
import time
from dataclasses import dataclass, field
from itertools import combinations, permutations

from core.enums import PokerHand, Rank, Suit
from core.models import CARD_STRINGS, Card, Deck


@dataclass(slots=True)
class Holder:
    count: int = 0
    score: list[int] = field(default_factory=list)


def format_duration(elapsed_ns: int) -> str:
    if elapsed_ns < 1_000:
        return f"{elapsed_ns}ns"
    if elapsed_ns < 1_000_000:
        return f"{elapsed_ns / 1_000:.2f}us"
    if elapsed_ns < 1_000_000_000:
        return f"{elapsed_ns / 1_000_000:.2f}ms"
    return f"{elapsed_ns / 1_000_000_000:.2f}s"


def generate_draw_combos(
    remaining: list[int], minimums: list[int], cards_to_draw: int
) -> list[list[int]]:
    if sum(minimums) > cards_to_draw:
        return []

    results = []
    stack = [(0, 0, [])]
    while stack:
        index, used_cards, drawn = stack.pop()
        if index == len(remaining):
            results.append([*drawn, cards_to_draw - used_cards])
            continue

        min_draw = minimums[index]
        max_draw = min(remaining[index], cards_to_draw - used_cards)
        for amount in range(max_draw, min_draw - 1, -1):
            stack.append((index + 1, used_cards + amount, [*drawn, amount]))

    return results


def calculate_odds(
    deck: Deck,
    hand: list[Card],
    bucket: list[Holder],
    values: list[list[int]],
    amount: list[int],
):
    max_iter = len(bucket)
    total_cards = deck.total_cards
    total_draws = math.comb(total_cards, 5)
    val_weights = [0.0] * max_iter

    attr = ""
    match max_iter:
        case 4:
            id_shift, attr = 2, "suits"

        case 13:
            id_shift, attr = 4, "ranks"

        case _:
            id_shift, attr = 6, "suit_rank"

    best_val = -1
    best_score = -1
    best_prob = 0.0

    for data in values:
        good_draws = 0
        score = 0
        amount_needed = []
        deck_val_amounts = []
        
        skip_count = 0
        for val, req_amount in zip(data, amount):
            needed = max(0, req_amount - bucket[val].count)
            if needed == 0:
                skip_count += 1
                continue

            amount_needed.append(needed)
            deck_val = getattr(deck, attr)
            deck_val_count = len(deck_val[val].cards)
            if deck_val_count == 0 or needed > deck_val_count:
                skip_count += 1
                continue

            deck_val_amounts.append(deck_val_count)

            expected_deck_score = deck_val[val].score / deck_val_count

            score += sum(bucket[val].score[: bucket[val].count]) + expected_deck_score

        if skip_count == len(data):
            continue

        draw_combos = generate_draw_combos(deck_val_amounts, amount_needed, 5)
        # we append this here since the generate combos adds a discard amount (like leftover is there is any)
        # that way we can use zip when iterating over the good draws
        deck_val_amounts.append(total_cards - sum(deck_val_amounts))

        good_draws += sum(
            math.prod(
                math.comb(available, amount)
                for available, amount in zip(deck_val_amounts, draw)
            )
            for draw in draw_combos
        )

        probability = good_draws / total_draws

        val_id = 1
        weighted_score = probability * score
        for val in data:
            val_weights[val] += weighted_score
            val_id = val_id << id_shift

        if probability > best_prob:
            best_prob = probability
            best_score = score
            best_val = val_id

        elif probability == best_prob and score > best_score:
            best_score = score
            best_val = val_id

    ordered_hand = sorted(
        hand,
        key=lambda x: val_weights[
            x.suit
            if max_iter == 4
            else x.rank
            if max_iter == 13
            else x.suit << 4 | x.rank
        ],
    )

    discard = ordered_hand[:5]

    return best_val, best_prob, discard


def generate_discard_table(
    deck: Deck, dealt_cards: list[Card]
) -> dict[PokerHand, dict[str, int | float | list[Card]]]:
    suit_bucket = [Holder() for _ in range(4)]
    rank_bucket = [Holder() for _ in range(13)]
    suit_rank_bucket = [Holder() for _ in range((Suit.SPADES << 4 | Rank.ACE) + 1)]
    for card in dealt_cards:
        rank, suit = card.rank, card.suit

        suit_rank_key = suit << 4 | rank
        suit_rank_bucket[suit_rank_key].count += 1
        suit_rank_bucket[suit_rank_key].score.append(card.score)

        suit_bucket[suit].count += 1
        suit_bucket[suit].score.append(card.score)

        rank_bucket[rank].count += 1
        rank_bucket[rank].score.append(card.score)

    for suit in suit_bucket:
        suit.score.sort(reverse=True)

    for rank in rank_bucket:
        rank.score.sort(reverse=True)

    for suit_rank in suit_rank_bucket:
        suit_rank.score.sort(reverse=True)

    table = {}
    rows: list[tuple[PokerHand, int, float, list[Card], int]] = []

    rank_array = [rank for rank in Rank]
    suit_array = [suit for suit in Suit]

    straight_array = rank_array[::-1]
    straight_array.append(straight_array[0])
    straight_combo = []
    for cutoff in range(5, len(straight_array) + 1):
        straight_combo.append(straight_array[cutoff - 5 : cutoff])

    full_house_combo = list(permutations(rank_array, 2))

    for hand in PokerHand:
        if hand == PokerHand.HIGH_CARD:
            continue

        val, prob, discard, amount = -1, 0, [], []
        values = []
        bucket = []
        match hand:
            case PokerHand.PAIR:
                bucket = rank_bucket
                values = [[rank] for rank in rank_array]
                amount = [2]

            case PokerHand.THREE_OF_A_KIND:
                bucket = rank_bucket
                values = [[rank] for rank in rank_array]
                amount = [3]

            case PokerHand.FOUR_OF_A_KIND:
                bucket = rank_bucket
                values = [[rank] for rank in rank_array]
                amount = [4]

            case PokerHand.FIVE_OF_A_KIND:
                bucket = rank_bucket
                values = [[rank] for rank in rank_array]
                amount = [5]

            case PokerHand.TWO_PAIR:
                two_pair_combo = list(combinations(rank_array, 2))
                bucket = rank_bucket
                values = [[val for val in two_pair] for two_pair in two_pair_combo]
                amount = [2, 2]

            case PokerHand.STRAIGHT:
                bucket = rank_bucket
                values = straight_combo
                amount = [1] * 5

            case PokerHand.FLUSH:
                bucket = suit_bucket
                values = [[suit] for suit in suit_array]
                amount = [5]

            case PokerHand.FULL_HOUSE:
                bucket = rank_bucket
                values = [
                    [val for val in full_house] for full_house in full_house_combo
                ]
                amount = [3, 2]

            case PokerHand.STRAIGHT_FLUSH:
                bucket = suit_rank_bucket
                values = [
                    [suit << 4 | rank for suit in suit_array for rank in straight]
                    for straight in straight_combo
                ]
                amount = [1] * 5

            case PokerHand.FLUSH_HOUSE:
                bucket = suit_rank_bucket
                values = [
                    [suit << 4 | rank for rank in full_house]
                    for full_house in full_house_combo
                    for suit in suit_array
                ]
                amount = [3, 2]

            case PokerHand.FLUSH_FIVE:
                bucket = suit_rank_bucket
                values = [
                    [suit << 4 | rank] for rank in rank_array for suit in suit_array
                ]
                amount = [5]

        val, prob, discard = calculate_odds(deck, dealt_cards, bucket, values, amount)

        table[hand] = {"value": val, "probability": prob, "discard": discard}

    return table


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

    deck._filter(hand)
    generate_discard_table(deck, hand)
