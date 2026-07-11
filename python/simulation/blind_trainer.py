import torch
from torch import nn


class BlindPolicy(nn.Module):
    def __init__(self, input_size: int, hidden_size: int = 128):
        super().__init__()

        self.trunk = nn.Sequential(
            nn.Linear(input_size, hidden_size),
            nn.ReLU(),
            nn.Linear(hidden_size, hidden_size),
            nn.ReLU(),
        )

        self.mode_head = nn.Linear(hidden_size, 2)
        self.count_head = nn.Linear(hidden_size, 5)
        self.card_head = nn.Linear(hidden_size, 8)

    def forward(self, state_features: torch.Tensor):
        hidden = self.trunk(state_features)

        return {
            "mode_logits": self.mode_head(hidden),
            "count_logits": self.count_head(hidden),
            "card_logits": self.card_head(hidden),
        }
