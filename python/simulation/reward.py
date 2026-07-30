from core.models import GameState


SCORE_PROGRESS_WEIGHT = 0.25
HAND_PLAY_COST = 0.2
WIN_REWARD = 10.0
LOSS_REWARD = -20.0
UNUSED_HAND_REWARD = 0.5


def calculate_score_progress_reward(
    previous_score: float, game_state: GameState
) -> float:
    """Provide light, path-independent reward for progress toward the blind."""
    target = max(float(game_state.score_to_beat), 1.0)
    previous_progress = min(previous_score / target, 1.0)
    current_progress = min(game_state.current_score / target, 1.0)
    progress_gained = max(current_progress - previous_progress, 0.0)
    return SCORE_PROGRESS_WEIGHT * progress_gained - HAND_PLAY_COST


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
