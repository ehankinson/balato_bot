import sys
from pathlib import Path

PYTHON_ROOT = Path(__file__).resolve().parent
if str(PYTHON_ROOT) not in sys.path:
    sys.path.insert(0, str(PYTHON_ROOT))

from tqdm import tqdm

from best_hand import get_best_scoring_hand
from core.models import Deck, GameState

if __name__ == "__main__":
    iterations = 100_000
    wins = 0

    game_state = GameState(score_to_beat=300)
    deck = Deck()
    jokers = []

    for _ in tqdm(range(iterations)):
        deck.shuffle()
        current_score = 0
        hand = deck.draw(game_state.hand_size)
        while game_state.hands > 0:
            best_score = get_best_scoring_hand(hand, jokers, game_state)

            # for now since we are only simulating the frist blind
            # we are going to just take chips * worst_case_mult
            # but later we will dynamically calculate probability cards
            # like lucky, bloostone and others ...
            game_state.play_hand()
            current_score += best_score.best_hand.chips * best_score.best_hand.worst_case_mult
            
            if current_score >= game_state.score_to_beat:
                wins += 1
                break

            cards_played = best_score.hand_scoring.scored_played + best_score.hand_scoring.unscored_played
            for card in cards_played:
                hand.remove(card)
        
            deck.add_to_discard_pile(cards_played)
            hand.extend(deck.draw(game_state.hand_size - len(hand)))

        deck.add_to_discard_pile(hand)
        deck.reset()
        game_state.reset()

    print(f"The 'GREEDY' win% is {round(wins / iterations * 100, 2)}%")
