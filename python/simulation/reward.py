from core.models import GameState


def calculate_score_progress_reward(
    previous_score: float, game_state: GameState
) -> float:
    """Reward newly earned progress, capped at completion of the blind."""
    target = max(float(game_state.score_to_beat), 1.0)
    previous_progress = min(previous_score / target, 1.0)
    current_progress = min(game_state.current_score / target, 1.0)
    return current_progress - previous_progress


def calculate_terminal_reward(game_state: GameState) -> float:
    """Make winning primary; reward efficient wins without rewarding discards."""
    if game_state.current_score < game_state.score_to_beat:
        return -1.0
    return 5.0 + 0.25 * game_state.hands


def calculate_game_score(game_state: GameState) -> float:
    """Total shaped episode reward, primarily for evaluation/reporting."""
    target = max(float(game_state.score_to_beat), 1.0)
    progress = min(game_state.current_score / target, 1.0)
    return progress + calculate_terminal_reward(game_state)
