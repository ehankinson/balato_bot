from config.poker_hands import HAND_STATS
from core.enums import PokerHand
from core.models import GameState

SCORE_PROGRESS_WEIGHT = 0.45
HAND_PLAY_COST = 0.25
HAND_VALUE_WEIGHT = 0.5
WIN_REWARD = 15.0
LOSS_REWARD = -30.0
UNUSED_HAND_REWARD = 1.5

BASE_HAND_SCORE = {
    PokerHand.FLUSH_FIVE: 5,
    PokerHand.FLUSH_HOUSE: 5,
    PokerHand.FIVE_OF_A_KIND: 4.5,
    PokerHand.STRAIGHT_FLUSH: 4,
    PokerHand.FOUR_OF_A_KIND: 3.5,
    PokerHand.FULL_HOUSE: 3,
    PokerHand.FLUSH: 3,
    PokerHand.STRAIGHT: 2.5,
    PokerHand.THREE_OF_A_KIND: 2,
    PokerHand.TWO_PAIR: 1.5,
    PokerHand.PAIR: 1,
    PokerHand.HIGH_CARD: 0.5,
}

HAND_SCORES = {
    hand: base_score + (HAND_STATS[hand].chips * HAND_STATS[hand].mult) // 100
    for hand, base_score in BASE_HAND_SCORE.items()
}


def calculate_score_progress_reward(
    previous_score: float, game_state: GameState
) -> float:
    """Provide light, path-independent reward for progress toward the blind."""
    target = max(float(game_state.score_to_beat), 1.0)
    previous_progress = min(previous_score / target, 1.0)
    current_progress = min(game_state.current_score / target, 1.0)
    progress_gained = max(current_progress - previous_progress, 0.0)
    return SCORE_PROGRESS_WEIGHT * progress_gained - HAND_PLAY_COST


def calculate_hand_value_reward(played_hand: PokerHand) -> float:
    """Reward playing higher-value hand types so straights and flushes are
    valued alongside full houses instead of defaulting to pairs."""
    return HAND_VALUE_WEIGHT * HAND_SCORES[played_hand]


def calculate_terminal_reward(game_state: GameState) -> float:
    """Keep winning primary, with unused hands as a small tie-breaker."""
    if game_state.current_score < game_state.score_to_beat:
        return LOSS_REWARD
    return WIN_REWARD + UNUSED_HAND_REWARD * game_state.hands


def calculate_game_score(game_state: GameState) -> float:
    """Terminal evaluation metric based on final progress and win outcome."""
    target = max(float(game_state.score_to_beat), 1.0)
    progress = min(game_state.current_score / target, 1.0)
    return progress + calculate_terminal_reward(game_state)
