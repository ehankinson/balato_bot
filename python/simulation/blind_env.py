import torch

from calculation.poker_discards import generate_discard_table
from calculation.score import get_best_scoring_hand
from core.models import Card, Deck, GameState
from simulation.encoder import encode_game_state


def reset(hand: list[Card], deck: Deck, game_state: GameState) -> torch.Tensor:
    deck.add_to_discard_pile(hand)
    deck.reset()
    game_state.reset()
    hand = deck.draw(game_state.hand_size)

    best_hand = get_best_scoring_hand(hand, [], game_state)
    discard_table = generate_discard_table(deck, hand)
    encoded_game_state = encode_game_state(hand, game_state, best_hand, discard_table)
    return encoded_game_state


def play_hand(card_indices: list[int], hand: list[Card], game_state: GameState, deck: Deck):
    playing_hand = [hand[i] for i in card_indices]
    return game_state.play_hand(playing_hand, hand, deck)