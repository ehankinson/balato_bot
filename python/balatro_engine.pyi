type CardData = tuple[int, int, int]
type DiscardResult = tuple[int, float, list[CardData]]


def generate_discard_table(
    total_cards: int,
    suit_counts: list[int],
    suit_scores: list[int],
    rank_counts: list[int],
    rank_scores: list[int],
    suit_rank_counts: list[int],
    suit_rank_scores: list[int],
    hand: list[CardData],
) -> list[DiscardResult]: ...
