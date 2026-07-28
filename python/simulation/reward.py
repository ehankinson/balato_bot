import math

from core.models import GameState


HAND_SCORE_QUALITY_WEIGHT = 0.5
NON_WINNING_HAND_COST = 0.2
WIN_BASE_REWARD = 1.0
MAX_HAND_EFFICIENCY_BONUS = 4.0
HAND_EFFICIENCY_BASE = 2.0


def calculate_score_progress_reward(
    previous_score: float, game_state: GameState
) -> float:
    """Reward larger scoring hands while charging for plays that do not win."""
    target = max(float(game_state.score_to_beat), 1.0)
    previous_progress = min(previous_score / target, 1.0)
    current_progress = min(game_state.current_score / target, 1.0)
    progress_reward = max(current_progress - previous_progress, 0.0)

    score_gained = max(game_state.current_score - previous_score, 0.0)
    normalized_hand_score = min(score_gained / target, 1.0)
    hand_quality_bonus = (
        HAND_SCORE_QUALITY_WEIGHT * normalized_hand_score**2
    )

    reward = progress_reward + hand_quality_bonus
    if game_state.current_score < game_state.score_to_beat:
        reward -= NON_WINNING_HAND_COST

    return reward


def calculate_terminal_reward(game_state: GameState) -> float:
    """Reward wins exponentially more when they preserve additional hands."""
    if game_state.current_score < game_state.score_to_beat:
        return -1.0

    initial_hands = game_state.hands + game_state.hands_played
    max_remaining_hands = max(initial_hands - 1, 0)
    if max_remaining_hands == 0:
        return WIN_BASE_REWARD + MAX_HAND_EFFICIENCY_BONUS

    remaining_hands = min(max(game_state.hands, 0), max_remaining_hands)
    efficiency = (
        math.pow(HAND_EFFICIENCY_BASE, remaining_hands - 1.0)
    ) / (
        math.pow(HAND_EFFICIENCY_BASE, max_remaining_hands - 1.0)
    )

    return WIN_BASE_REWARD + MAX_HAND_EFFICIENCY_BONUS * efficiency


def calculate_game_score(game_state: GameState) -> float:
    """Terminal evaluation metric based on final progress and win outcome."""
    target = max(float(game_state.score_to_beat), 1.0)
    progress = min(game_state.current_score / target, 1.0)
    return progress + calculate_terminal_reward(game_state)
