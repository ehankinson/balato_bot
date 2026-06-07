from core.models import GameState, JokerGameModifier


def build_game_state(game_state: GameState, jokers: list[JokerGameModifier]):
    for joker in jokers:
        if joker.double_probabilities:
            game_state.probabily_mult *= 2