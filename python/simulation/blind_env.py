from dataclasses import dataclass

import torch

# Action mode constants: 0 = play a hand, 1 = discard cards.
MODE_PLAY = 0
MODE_DISCARD = 1

# Max cards the model can select per action.
MAX_PLAY_CARDS = 5
MAX_DISCARD_CARDS = 5

# Output head sizes — used by BlindModel in blind_trainer.py.
COUNT_HEAD_SIZE = 5
CARD_HEAD_SIZE = 8


@dataclass(slots=True)
class ActionMasks:
    """1/0 mask tensors blocking illegal choices for each output head."""
    mode: torch.Tensor   # shape [2]: legal modes (play, discard)
    count: torch.Tensor   # shape [5]: legal card counts (1..5)
    card: torch.Tensor    # shape [8]: legal hand slots