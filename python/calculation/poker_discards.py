import math

from core.enums import Rank, Suit
from core.models import Card, Deck


def calculate_best_odds(
    bucket: list[int],
    deck_lookup: dict[int, list[Card]],
    total_cards: int,
    amount_to_look_for: int,
) -> tuple[float, int]:
    total_draw_combos = math.comb(total_cards, 5)
    best_prob = float("-inf")
    best_suit = -1

    for val, val_count in enumerate(bucket):
        if val_count == amount_to_look_for:
            continue

        cards_needed = amount_to_look_for - val_count
        deck_val_total = len(deck_lookup[val])
        if deck_val_total < cards_needed:
            continue

        non_val_cards = total_cards - deck_val_total
        total_probability = 0.0

        end_iter = 5 if deck_val_total >= 5 else deck_val_total
        for amount in range(cards_needed, end_iter + 1):
            total_probability += (
                math.comb(deck_val_total, amount)
                * math.comb(non_val_cards, 5 - amount)
                / total_draw_combos
            )

        if total_probability > best_prob:
            best_prob = total_probability
            best_suit = val

    return best_prob, best_suit


def flush_odds(deck: Deck, dealt_cards: list[Card]) -> tuple[list[Card], float, int]:
    suit_bucket = [0] * 4
    for card in dealt_cards:
        suit_bucket[card.suit] += 1

    suit_prob, suit = calculate_best_odds(suit_bucket, deck.suits, deck.total_cards, 5)
    cards_to_discard = [card for card in dealt_cards if card.suit != suit]

    return cards_to_discard, suit_prob, suit


def x_of_a_kind_odds(
    deck: Deck, dealt_cards: list[Card], x: int
) -> tuple[list[Card], float, int]:
    rank_bucket = [0] * 13
    for card in dealt_cards:
        rank_bucket[card.rank] += 1

    rank_prob, rank = calculate_best_odds(rank_bucket, deck.ranks, deck.total_cards, x)
    cards_to_discard = [card for card in dealt_cards if card.rank != rank]

    return cards_to_discard, rank_prob, rank


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
    flust_discard, prob, suit = flush_odds(deck, hand)
    x = 4
    pair_discard, prob, card = x_of_a_kind_odds(deck, hand, x)
    print(f"The prob for a {x} of a kind is: {prob}")
    print(f"We are looking for {card}")
    for card in pair_discard:
        print(card)
