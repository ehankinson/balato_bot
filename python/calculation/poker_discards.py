import balatro_engine
from core.enums import PokerHand, Rank, Suit
from core.models import Card, Deck


def generate_discard_table(
    deck: Deck, dealt_cards: list[Card]
) -> dict[PokerHand, tuple[int, float, list[Card]]]:
    suit_counts = [0] * 4
    suit_scores = [0] * 4
    for suit, bucket in deck.suits.items():
        index = int(suit)
        suit_counts[index] = len(bucket.cards)
        suit_scores[index] = bucket.score

    rank_counts = [0] * 13
    rank_scores = [0] * 13
    for rank, bucket in deck.ranks.items():
        index = int(rank)
        rank_counts[index] = len(bucket.cards)
        rank_scores[index] = bucket.score

    suit_rank_counts = [0] * 64
    suit_rank_scores = [0] * 64
    for suit_rank, bucket in deck.suit_rank.items():
        index = int(suit_rank)
        suit_rank_counts[index] = len(bucket.cards)
        suit_rank_scores[index] = bucket.score

    rust_hand = [(int(card.rank), int(card.suit), card.score) for card in dealt_cards]
    rust_table = balatro_engine.generate_discard_table(
        deck.total_cards,
        suit_counts,
        suit_scores,
        rank_counts,
        rank_scores,
        suit_rank_counts,
        suit_rank_scores,
        rust_hand,
    )

    discard_hands = tuple(
        poker_hand for poker_hand in PokerHand if poker_hand != PokerHand.HIGH_CARD
    )
    if len(rust_table) != len(discard_hands):
        raise RuntimeError(
            "Rust returned an unexpected number of discard-table entries: "
            f"expected {len(discard_hands)}, received {len(rust_table)}"
        )

    table: dict[PokerHand, tuple[int, float, list[Card]]] = {}
    for poker_hand, (value, probability, rust_discard) in zip(
        discard_hands, rust_table, strict=True
    ):
        available_cards = list(dealt_cards)
        discard: list[Card] = []

        for rank, suit, score in rust_discard:
            for index, card in enumerate(available_cards):
                if (
                    int(card.rank) == rank
                    and int(card.suit) == suit
                    and card.score == score
                ):
                    discard.append(available_cards.pop(index))
                    break
            else:
                raise RuntimeError(
                    "Rust returned a discard card that was not present "
                    f"in the dealt hand: {(rank, suit, score)}"
                )

        table[poker_hand] = (value, probability, discard)

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

    import time

    deck.filter(hand)
    benchmark_duration = 1.0
    start_time = time.perf_counter()
    tables_generated = 0
    discard_table = {}
    while time.perf_counter() - start_time < benchmark_duration:
        discard_table = generate_discard_table(deck, hand)
        tables_generated += 1
    end_time = time.perf_counter()
    elapsed = end_time - start_time

    print("\nDiscard table")
    print(f"{'Hand':<18} {'Probability':>12}  Discard")
    print("-" * 78)
    for poker_hand, (_value, probability, discard) in discard_table.items():
        cards = ", ".join(repr(card) for card in discard) or "-"
        hand_name = poker_hand.name.replace("_", " ").title()
        print(f"{hand_name:<18} {probability:>11.2%}  {cards}")

    print(
        f"\nGenerated {tables_generated} discard tables in {elapsed:.3f} s "
        f"({tables_generated / elapsed:.0f} tables/sec, "
        f"{elapsed * 1_000 / tables_generated:.3f} ms/table)"
    )
