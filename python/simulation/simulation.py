from tqdm import tqdm

from best_hand import get_best_scoring_hand
from calculation.poker_discards import calculate_good_draws
from core.models import Deck, GameState
from simulation.encoder import encode_game_state

if __name__ == "__main__":
    iterations = 100_000
    wins = 0

    game_state = GameState(score_to_beat=300)
    deck = Deck()
    jokers = []

    for _ in tqdm(range(iterations)):
        deck.shuffle()
        hand = deck.draw(game_state.hand_size)
        while game_state.hands > 0:
            best_score = get_best_scoring_hand(hand, jokers, game_state)
            discard_table = calculate_good_draws(deck, hand)

            encoded_game_state = encode_game_state(hand, game_state, best_score, discard_table)

            # for now since we are only simulating the frist blind
            # we are going to just take chips * worst_case_mult
            # but later we will dynamically calculate probability cards
            # like lucky, bloostone and others ...
            game_state.play_hand()
            game_state.current_score += best_score.best_hand.chips * best_score.best_hand.worst_case_mult
            
            if game_state.current_score >= game_state.score_to_beat:
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
