import random


def get_initial_card_chips(rank: int) -> int:
    if rank < 8:
        return rank + 2

    if rank < 12:
        return 10

    return 11


def calculate_lucky() -> list[int]:
    final = [0, 0]
    rand = random.random()

    if rand < 1 / 15:
        final[1] = 20

    if rand < 1 / 5:
        final[0] = 20

    return final

