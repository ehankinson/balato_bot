import math
from core.models import GameState


def calculate_game_score(game_state: GameState) -> float:
    reward = 0

    # This is for, in the early game its better to have played a minimum amount of hands
    # since we get more money for hands left, while in the late game, since we should have
    # jokers/cards to generate econ/taror/planet... cards we want to use our hand
    ante_decay = int(math.pow(100, 1 / game_state.ante))

    reward += game_state.current_score
    reward += (game_state.hands - game_state.hands_played) * 50 / ante_decay
    reward += (game_state.discards - game_state.discards_used) * 50

    if game_state.current_score >= game_state.score_to_beat:
        reward += 1000
    
    return reward