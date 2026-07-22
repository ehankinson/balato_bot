from dataclasses import dataclass

import torch
from torch import nn
from torch.distributions import Categorical

from simulation.blind_env import CARD_HEAD_SIZE, COUNT_HEAD_SIZE, ActionMasks

MODE_HEAD_SIZE = 2


@dataclass(slots=True)
class PolicyAction:
    mode: int
    count: int
    card_indices: list[int]
    log_prob: float
    value: float
    entropy: float


class BlindModel(nn.Module):
    def __init__(self, input_size: int, hidden_size: int = 128):
        super().__init__()

        self.trunk = nn.Sequential(
            nn.Linear(input_size, hidden_size),
            nn.ReLU(),
            nn.Linear(hidden_size, hidden_size),
            nn.ReLU(),
        )

        self.mode_head = nn.Linear(hidden_size, MODE_HEAD_SIZE)
        # Playing and discarding have different count/card objectives. Keeping
        # separate heads lets the policy learn e.g. "play a pair" without also
        # forcing it to discard two cards.
        self.play_count_head = nn.Linear(hidden_size, COUNT_HEAD_SIZE)
        self.discard_count_head = nn.Linear(hidden_size, COUNT_HEAD_SIZE)
        self.play_card_head = nn.Linear(hidden_size, CARD_HEAD_SIZE)
        self.discard_card_head = nn.Linear(hidden_size, CARD_HEAD_SIZE)
        self.value_head = nn.Linear(hidden_size, 1)

    def forward(self, state_features: torch.Tensor) -> dict[str, torch.Tensor]:
        hidden = self.trunk(state_features)
        return {
            "mode_logits": self.mode_head(hidden),
            "play_count_logits": self.play_count_head(hidden),
            "discard_count_logits": self.discard_count_head(hidden),
            "play_card_logits": self.play_card_head(hidden),
            "discard_card_logits": self.discard_card_head(hidden),
            "value": self.value_head(hidden).squeeze(-1),
        }


def evaluate_state(model: BlindModel, observation: torch.Tensor) -> float:
    model.eval()
    device = next(model.parameters()).device
    with torch.no_grad():
        out = model(observation.unsqueeze(0).to(device))
    return float(out["value"].item())


def act(
    model: BlindModel,
    observation: torch.Tensor,
    masks: ActionMasks,
    stochastic: bool = True,
) -> PolicyAction:
    model.eval()
    device = next(model.parameters()).device
    with torch.no_grad():
        out = model(observation.unsqueeze(0).to(device))
        action, log_prob, entropy = _sample_action(out, masks, stochastic)
        value = float(out["value"].item())
    model.train()
    return PolicyAction(*action, log_prob, value, entropy)


def _sample_action(
    outputs: dict[str, torch.Tensor],
    masks: ActionMasks,
    stochastic: bool,
) -> tuple[tuple[int, int, list[int]], float, float]:
    mode_logits = outputs["mode_logits"][0]
    mode_mask = masks.mode.to(mode_logits.device)

    masked_mode = mode_logits.masked_fill(mode_mask == 0, float("-inf"))
    mode_dist = Categorical(logits=masked_mode)

    device = mode_logits.device

    def _t(val: int) -> torch.Tensor:
        return torch.tensor(val, device=device)

    if stochastic:
        mode = int(mode_dist.sample().item())
    else:
        mode = int(torch.argmax(masked_mode).item())

    if mode == MODE_HEAD_SIZE - 1:
        count_logits = outputs["discard_count_logits"][0]
        card_logits = outputs["discard_card_logits"][0]
    else:
        count_logits = outputs["play_count_logits"][0]
        card_logits = outputs["play_card_logits"][0]

    count_mask = masks.count.to(count_logits.device)
    card_mask = masks.card.to(card_logits.device)
    masked_count = count_logits.masked_fill(count_mask == 0, float("-inf"))
    count_dist = Categorical(logits=masked_count)

    if stochastic:
        count = int(count_dist.sample().item()) + 1
        log_prob = mode_dist.log_prob(_t(mode))
        log_prob = log_prob + count_dist.log_prob(_t(count - 1))

        card_indices: list[int] = []
        remaining = card_mask.clone()
        card_log_probs: list[torch.Tensor] = []
        card_entropies: list[torch.Tensor] = []
        for _ in range(count):
            masked = card_logits.masked_fill(remaining == 0, float("-inf"))
            dist = Categorical(logits=masked)
            idx = int(dist.sample().item())
            card_log_probs.append(dist.log_prob(_t(idx)))
            card_entropies.append(dist.entropy())
            remaining[idx] = 0.0
            card_indices.append(idx)
        log_prob = log_prob + torch.stack(card_log_probs).sum()
        entropy = (
            mode_dist.entropy()
            + count_dist.entropy()
            + torch.stack(card_entropies).sum()
        ).item()
    else:
        count = int(torch.argmax(masked_count).item()) + 1
        log_prob = torch.tensor(0.0, device=device)
        card_indices = []
        remaining = card_mask.clone()
        for _ in range(count):
            masked = card_logits.masked_fill(remaining == 0, float("-inf"))
            idx = int(torch.argmax(masked).item())
            remaining[idx] = 0.0
            card_indices.append(idx)
        entropy = 0.0

    return (mode, count, card_indices), float(log_prob.item()), float(entropy)
