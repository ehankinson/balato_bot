from calculation.score import calculate_scoring_hand
from core.enums import PokerHand, Rank, Suit
from core.models import Card, GameState


def test_played_hand_tracking_uses_zero_based_enum_indices():
    game_state = GameState(score_to_beat=300)

    calculate_scoring_hand(
        [Card(rank=Rank.ACE, suit=Suit.SPADES)],
        [],
        game_state,
    )
    calculate_scoring_hand(
        [Card(rank=Rank.ACE, suit=Suit.SPADES) for _ in range(5)],
        [],
        game_state,
    )

    assert game_state.played_hands[PokerHand.HIGH_CARD - 1] == 1
    assert game_state.played_hands[PokerHand.FLUSH_FIVE - 1] == 1
    assert sum(game_state.played_hands) == 2
