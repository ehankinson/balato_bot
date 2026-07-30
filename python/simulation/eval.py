import os
import random

import torch
from tqdm import tqdm
from calculation.poker_discards import generate_discard_table
from calculation.score import get_best_scoring_hand
from config.settings import ROOT_DIR
from core.enums import PokerHand
from core.models import Deck, GameState
from simulation.blind_trainer import BlindModel
from simulation.decoder import build_mask, model_decoder
from simulation.encoder import encode_game_state
from simulation.reward import calculate_game_score

DEVICE = torch.device("cuda" if torch.cuda.is_available() else "cpu")
BLIND_MODEL_CHECKPOINT = torch.load(os.path.join(ROOT_DIR, "models", "blind_model.pt"), map_location="cuda")
BLIND_MODEL = BlindModel(BLIND_MODEL_CHECKPOINT["input_size"], BLIND_MODEL_CHECKPOINT["hidden_size"]).to(DEVICE)
BLIND_MODEL.load_state_dict(BLIND_MODEL_CHECKPOINT["state_dict"])
BLIND_MODEL.eval()

if __name__ == "__main__":
    iter = 500
    wins = 0
    rewards = []
    deck = Deck()
    game_state = GameState(score_to_beat=0)
    jokers = []

    for ep in tqdm(range(iter)):
        score_to_beat = random.choice([300, 450])
        game_state.score_to_beat = score_to_beat
        hand = deck.draw(game_state.hand_size)
        has_won = False

        while game_state.hands > 0:
            best_hand = get_best_scoring_hand(hand, jokers, game_state)
            discard_table = generate_discard_table(deck, hand)

            encoded_state = encode_game_state(
                hand, game_state, best_hand, discard_table
            )

            outputs = BLIND_MODEL(encoded_state.unsqueeze(0).to(DEVICE))
            masks = build_mask(game_state, hand, DEVICE)
            mode, count, card_indices, log_prob, entropy = model_decoder(
                outputs, masks, DEVICE, stochastic=False
            )

            selected_cards = [hand[index] for index in card_indices]
            for card in selected_cards:
                hand.remove(card)

            has_won = game_state.execute_hand_action(mode, selected_cards, hand, deck)
            if has_won:
                break

        if has_won:
            wins += 1
        rewards.append(calculate_game_score(game_state))

        deck.add_to_discard_pile(hand)
        deck.reset()
        game_state.reset()

    print(f"The Win Rate is {wins / iter * 100:.2f}%")
    print(f"The AVG Reward per blind was {sum(rewards) / iter:.2f}")

    for hand in PokerHand:
        print(f"Hand {hand.name} was played {game_state.played_hands[hand - 1]} times")
