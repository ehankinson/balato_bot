import heapq
import math
from itertools import product

from core.enums import PokerHand, Rank, Suit
from core.models import CARD_STRINGS, Card, Deck


def generate_draw_combos(
    remaining: list[int], minimums: list[int], cards_to_draw: int
) -> list[list[int]]:
    results = []
    for drawn in product(*(range(count + 1) for count in remaining)):
        if any(drawn[i] < minimums[i] for i in range(len(drawn))):
            continue

        other_cards = cards_to_draw - sum(drawn)

        if other_cards >= 0:
            results.append([*drawn, other_cards])

    return results


def calculate_odds(
    deck: Deck,
    hand: list[Card],
    bucket: list[int],
    values: list[list[list[int]]],
):
    good_draws = 1
    max_iter = len(bucket)
    total_cards = deck.total_cards
    total_draws = math.comb(total_cards, 5)
    val_weights = [0.0] * max_iter

    shift_amount, equal_val, attr = -1, -1, ""
    match max_iter:
        case 4:
            (
                shift_amount,
                equal_val,
                id_shift,
                attr,
            ) = 9, 0b11, 2, "suits"

        case 13:
            shift_amount, equal_val, id_shift, attr = 11, 0b1111, 4, "ranks"

        case _:
            shift_amount, equal_val, id_shift, attr = 9, 0b111111, 6, "suit_rank"

    best_val = -1
    best_score = -1
    best_prob = 0.0

    for data in values:
        score = 0
        amount_needed = []
        deck_val_amounts = []

        for val, req_amount in data:
            needed = max(0, req_amount - bucket[val])
            amount_needed.append(needed)
            deck_val = getattr(deck, attr)
            deck_val_count = len(deck_val[val])
            deck_val_amounts.append(deck_val_count)

            max_score = []
            for card in hand:
                if (card.card_id >> shift_amount) & equal_val == val:
                    heapq.heappush_max(max_score, card.score)

            expected_deck_score = (
                sum(card.score for card in deck_val) / deck_val_count * needed
            )

            score += sum(max_score[: bucket[val]]) + expected_deck_score

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
    suit_bucket = [0] * 4
    rank_bucket = [0] * 13
    suit_rank_bucket = [0] * ((Suit.SPADES << 4 | Rank.ACE) + 1)
    for card in dealt_cards:
        rank, suit = card.rank, card.suit
        suit_rank_key = suit << 4 | rank
        suit_rank_bucket[suit_rank_key] += 1
        suit_bucket[suit] += 1
        rank_bucket[rank] += 1

    table = {}

    rank_array = [rank for rank in Rank]
    suit_array = [suit for suit in Suit]

    for hand in PokerHand:
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

            case PokerHand.FLUSH:
                bucket = suit_bucket
                values = [[[suit, 5]] for suit in suit_array]

            case PokerHand.TWO_PAIR:
                combinations = list(product(rank_array, suit_array))
                bucket = suit_bucket
                values = [[[rank, 5]] for rank in Rank]

        val, prob, discard = calculate_odds(deck, dealt_cards, bucket, values)

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
