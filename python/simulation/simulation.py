from tqdm import tqdm

from core.models import Deck, GameState
from simulation.blind_env import reset

if __name__ == "__main__":
    iterations = 100_000
    wins = 0

    hand = []
    jokers = []
    deck = Deck()
    game_state = GameState(score_to_beat=300)

    for _ in tqdm(range(iterations)):
        reset(hand, deck, game_state)

    print(f"The 'GREEDY' win% is {round(wins / iterations * 100, 2)}%")
