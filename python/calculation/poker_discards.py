import math
import time

from core.enums import Rank, Suit
from core.models import Card, Deck


def pretty_time(seconds: float) -> str:
    if seconds < 0.001:
        return f"{seconds * 1_000_000:.0f}us"

    if seconds < 1:
        return f"{seconds * 1_000:.0f}ms"

    return f"{seconds:.2f}s"


def flush_odds(deck: Deck, dealt_cards: list[Card]):
    suit_bucket = {}
    for card in dealt_cards:
        if card.suit not in suit_bucket:
            suit_bucket[card.suit] = 0

        suit_bucket[card.suit] += 1

    total_cards = deck.total_cards
    total_draw_combos = math.comb(total_cards, 5)

    for suit, dealt_suit_count in suit_bucket.items():
        deck_suit_total = len(deck.suits[suit])
        total_probability = 0.0
        amount_needed_for_flush = 5 - dealt_suit_count
        non_flush_cards = total_cards - deck_suit_total
        
        for amount in range(amount_needed_for_flush, 6):    
            total_probability += (
                math.comb(deck_suit_total, amount)
                * math.comb(non_flush_cards, 5 - amount)
                / total_draw_combos
            )


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
    start_time = time.perf_counter()
    flush_odds(deck, hand)
    end_time = time.perf_counter()
    print(f"Total time taken was {pretty_time(end_time - start_time)}")
