from core.models import Card, GameState, JokerGameModifier


def build_game_state(
    game_state: GameState, jokers: list[JokerGameModifier], cards: list[Card]
):
    for joker in jokers:
        if joker.double_probabilities:
            game_state.probabily_mult *= 2

        if joker.all_cards_are_facecards:
            for card in cards:
                card.is_facecard = True
