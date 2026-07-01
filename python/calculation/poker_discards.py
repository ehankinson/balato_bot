import math

from core.enums import Rank, Suit
from core.models import Card, Deck





def flush_odds(deck: Deck, dealt_cards: list[Card]) -> tuple[list[Card], float]:
    suit_bucket = [0] * 4
    for card in dealt_cards:
        suit_bucket[card.suit] += 1

    total_cards = deck.total_cards
    total_draw_combos = math.comb(total_cards, 5)

    best_prob = float("-inf")
    best_suit = -1

    for suit, dealt_suit_count in enumerate(suit_bucket):
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

        if total_probability > best_prob:
            best_prob = total_probability
            best_suit = suit

    cards_to_discard = []
    for card in dealt_cards:
        if card.suit == best_suit:
            continue

        cards_to_discard.append(card)

    return cards_to_discard, best_prob


def x_of_a_kind_odds(deck: Deck, dealt_cards: list[Card], x: int) -> tuple[list[Card], float, int]:
    bucket = [0] * 13
    for card in dealt_cards:
        bucket[card.rank] += 1

    best_prob = float("-inf")
    best_rank = -1
    
    total_cards = deck.total_cards
    total_draw_combos = math.comb(total_cards, 5)

    for rank, rank_amount in enumerate(bucket):
        # if we already have x of a kind for this card
        # just skip since we already have what we are looking for
        if rank_amount >= x:
            continue
            
        cards_needed = x - rank_amount
        deck_rank_total = len(deck.ranks[rank])
        non_rank_total = total_cards - deck_rank_total
        total_probability = 0.0

        if deck_rank_total < cards_needed:
            # there is no enought cards left in the deck
            # to get this
            continue

        for fetch_amount in range(cards_needed, deck_rank_total + 1):
            total_probability += (
                math.comb(deck_rank_total, fetch_amount)
                * math.comb(non_rank_total, 5 - fetch_amount)
                / total_draw_combos
            )

        if total_probability > best_prob:
            best_prob = total_probability
            best_rank = rank

    cards_to_discard = []
    for card in dealt_cards:
        if len(cards_to_discard) == 5:
            break

        if card.rank == best_rank:
            continue
    
        cards_to_discard.append(card)

    return cards_to_discard, best_prob, best_rank
        
    
    
        


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
    flust_discard, prob = flush_odds(deck, hand)
    x = 4
    pair_discard, prob, card = x_of_a_kind_odds(deck, hand, x)
    print(f"The prob for a {x} of a kind is: {prob}")
    print(f"We are looking for {card}")
    for card in pair_discard:
        print(card)
