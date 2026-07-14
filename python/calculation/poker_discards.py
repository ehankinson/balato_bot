import heapq
import math
import time
from dataclasses import dataclass, field
from itertools import combinations, permutations

from core.enums import PokerHand, Rank, Suit
from core.models import Card, Deck


@dataclass(slots=True)
class Holder:
    count: int = 0
    score: list[int] = field(default_factory=list)


def format_duration(elapsed_ns: int) -> str:
    if elapsed_ns < 1_000:
        return f"{elapsed_ns} ns"
    if elapsed_ns < 1_000_000:
        return f"{elapsed_ns / 1_000:.2f} us"
    if elapsed_ns < 1_000_000_000:
        return f"{elapsed_ns / 1_000_000:.2f} ms"
    return f"{elapsed_ns / 1_000_000_000:.2f} s"


def generate_draw_combos(
    remaining: list[int], minimums: list[int], cards_to_draw: int
) -> list[list[int]]:
    if sum(minimums) > cards_to_draw:
        return []

    results = []

    def build_combo(index: int, used_cards: int, drawn: list[int]) -> None:
        if index == len(remaining):
            results.append([*drawn, cards_to_draw - used_cards])
            return

        min_draw = minimums[index]
        max_draw = min(remaining[index], cards_to_draw - used_cards)
        for amount in range(min_draw, max_draw + 1):
            build_combo(index + 1, used_cards + amount, [*drawn, amount])

    build_combo(0, 0, [])

    return results


def calculate_odds(
    deck: Deck,
    hand: list[Card],
    bucket: list[Holder],
    values: list[list[list[int]]],
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

        for val, req_amount in data:
            needed = max(0, req_amount - bucket[val].count)
            if needed == 0:
                continue
                
            amount_needed.append(needed)
            deck_val = getattr(deck, attr)
            deck_val_count = len(deck_val[val].cards)
            if deck_val_count == 0:
                continue

            deck_val_amounts.append(deck_val_count)

            expected_deck_score = deck_val[val].score / deck_val_count

            score += sum(bucket[val].score[: bucket[val].count]) + expected_deck_score

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

        val_id = 0
        weighted_score = probability * score
        for val, _ in data:
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
    total_start = time.perf_counter_ns()
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
        suit.score.sort()

    for rank in rank_bucket:
        rank.score.sort()

    for suit_rank in suit_rank_bucket:
        suit_rank.score.sort()


    table = {}

    rank_array = [rank for rank in Rank]
    suit_array = [suit for suit in Suit]
    straight_array = rank_array[::-1]
    straight_array.append(straight_array[0])
    full_house_combo = list(permutations(rank_array, 2))

    for hand in PokerHand:
        if hand == PokerHand.HIGH_CARD:
            continue

        start_time = time.perf_counter_ns()
        val, prob, discard = -1, 0, []
        values = []
        bucket = []
        match hand:
            case PokerHand.PAIR:
                bucket = rank_bucket
                values = [[[rank, 2]] for rank in rank_array]

            case PokerHand.THREE_OF_A_KIND:
                bucket = rank_bucket
                values = [[[rank, 3]] for rank in rank_array]

            case PokerHand.FOUR_OF_A_KIND:
                bucket = rank_bucket
                values = [[[rank, 4]] for rank in rank_array]

            case PokerHand.FIVE_OF_A_KIND:
                bucket = rank_bucket
                values = [[[rank, 5]] for rank in rank_array]

            case PokerHand.TWO_PAIR:
                two_pair_combo = list(combinations(rank_array, 2))
                bucket = rank_bucket
                values = [[[val, 2] for val in two_pair] for two_pair in two_pair_combo]

            case PokerHand.STRAIGHT:
                straight_combo = []
                for cutoff in range(5, len(straight_array) + 1):
                    straight_combo.append(straight_array[cutoff - 5 : cutoff])

                bucket = rank_bucket
                values = [[[x, 1] for x in straight] for straight in straight_combo]

            case PokerHand.FLUSH:
                bucket = suit_bucket
                values = [[[suit, 5]] for suit in suit_array]

            case PokerHand.FULL_HOUSE:
                bucket = rank_bucket
                values = [
                    [[val, 3 if i == 0 else 2] for i, val in enumerate(full_house)]
                    for full_house in full_house_combo
                ]

            case PokerHand.STRAIGHT_FLUSH:
                bucket = suit_rank_bucket
                straight_flush_array = [
                    suit << 4 | val for suit in suit_array for val in straight_array
                ]

                straight_flush_combo = []
                cutoff = 5
                while cutoff < len(straight_flush_array) + 1:
                    straight_flush_combo.append(
                        straight_flush_array[cutoff - 5 : cutoff]
                    )
                    add_val = 5 if cutoff % 14 == 0 else 1
                    cutoff += add_val

                values = [
                    [[val, 1] for val in straight_flush]
                    for straight_flush in straight_flush_combo
                ]

            case PokerHand.FLUSH_HOUSE:
                bucket = suit_rank_bucket
                values = [
                    [
                        [suit << 4 | val, 3 if i == 0 else 2]
                        for i, val in enumerate(full_house)
                    ]
                    for full_house in full_house_combo
                    for suit in suit_array
                ]

            case PokerHand.FLUSH_FIVE:
                bucket = suit_rank_bucket
                values = [
                    [[suit << 4 | rank, 5]]
                    for rank in rank_array
                    for suit in suit_array
                ]

        val, prob, discard = calculate_odds(deck, dealt_cards, bucket, values)
        end_time = time.perf_counter_ns()

        print(f"{hand.name:<18} {format_duration(end_time - start_time)}")

        table[hand] = {"value": val, "probability": prob, "discard": discard}

    total_end = time.perf_counter_ns()

    print(f"Total time taken was {format_duration(total_end - total_start)}")

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
