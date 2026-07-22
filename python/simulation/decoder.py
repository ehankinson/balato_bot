import torch
from torch.distributions import Categorical

from core.enums import HandAction
from core.models import Card, GameState
from simulation.blind_env import ActionMasks


def build_mask(
    game_state: GameState, hand: list[Card], device: torch.device
) -> ActionMasks:
    mode = torch.tensor(
        [1.0 if game_state.hands > 0 else 0.0, 1.0 if game_state.discards > 0 else 0.0],
        dtype=torch.float32,
    )
    count = torch.ones(5, dtype=torch.float32)

    # We do this since if our hand size is to srink, we will set the
    # excess to 0s while the hand_size = 1
    card = torch.zeros(8, dtype=torch.float32)
    card[: len(hand)] = 1.0

    # Move our mask down to the GPU making sure that we don't cause any errors
    # like playing a hand when there are no more hands left
    return ActionMasks(
        mode=mode.to(device), count=count.to(device), card=card.to(device)
    )


def model_decoder(
    values: dict[str, torch.Tensor],
    masks: ActionMasks,
    device: torch.device,
    stochastic: bool,
) -> tuple[HandAction, int, list[int], torch.Tensor, torch.Tensor]:
    mode_logits = values["mode_logits"][0]

    mode_dist = Categorical(
        logits=mode_logits.masked_fill(masks.mode == 0, float("-inf"))
    )

    entropy = torch.tensor(0.0).to(device)
    log_prob = torch.tensor(0.0).to(device)

    if stochastic:
        mode = HandAction(int(mode_dist.sample().item()))
        log_prob = mode_dist.log_prob(torch.tensor(mode).to(device))
        entropy = mode_dist.entropy()

    else:
        mode = HandAction(int(torch.argmax(mode_dist.logits).item()))

    if mode == HandAction.DISCARD:
        count_logits = values["discard_count_logits"][0]
        card_logits = values["discard_card_logits"][0]
    else:
        count_logits = values["play_count_logits"][0]
        card_logits = values["play_card_logits"][0]

    count_dist = Categorical(
        logits=count_logits.masked_fill(masks.count == 0, float("-inf"))
    )
    if stochastic:
        count = int(count_dist.sample().item()) + 1
        log_prob = log_prob + count_dist.log_prob(torch.tensor(count - 1).to(device))
        entropy = entropy + count_dist.entropy()
    else:
        count = int(torch.argmax(count_dist.logits).item()) + 1

    card_indices: list[int] = []
    card_mask = masks.card.clone()

    for _ in range(count):
        card_dist = Categorical(
            logits=card_logits.masked_fill(card_mask == 0, float("-inf"))
        )
        if stochastic:
            idx = int(card_dist.sample().item())
            log_prob = log_prob + card_dist.log_prob(torch.tensor(idx).to(device))
            entropy = entropy + card_dist.entropy()
        else:
            idx = int(torch.argmax(card_dist.logits).item())

        card_mask[idx] = 0.0
        card_indices.append(idx)

    return mode, count, card_indices, log_prob, entropy


def evaluate_actions(
    values: dict[str, torch.Tensor],
    mode_masks: torch.Tensor,
    count_masks: torch.Tensor,
    card_masks: torch.Tensor,
    modes: torch.Tensor,
    counts: torch.Tensor,
    card_indices: torch.Tensor,
    card_valid: torch.Tensor,
) -> tuple[torch.Tensor, torch.Tensor]:
    """Re-compute log_prob + entropy for STORED actions through CURRENT model outputs.

    Used in the PPO update phase: we replay each stored action through the
    current policy to get new_log_prob, then compute the PPO ratio against
    old_log_prob. Returns (log_probs [B], entropies [B]).
    """
    mode_logits = values["mode_logits"]
    is_discard = modes.bool().unsqueeze(1)
    count_logits = torch.where(
        is_discard,
        values["discard_count_logits"],
        values["play_count_logits"],
    )
    card_logits = torch.where(
        is_discard,
        values["discard_card_logits"],
        values["play_card_logits"],
    )

    mode_dist = Categorical(
        logits=mode_logits.masked_fill(mode_masks == 0, float("-inf"))
    )
    count_dist = Categorical(
        logits=count_logits.masked_fill(count_masks == 0, float("-inf"))
    )

    mode_log = mode_dist.log_prob(modes)
    count_log = count_dist.log_prob(counts - 1)
    mode_ent = mode_dist.entropy()
    count_ent = count_dist.entropy()

    B, K = card_indices.shape
    remaining = card_masks.clone()
    card_log_probs = []
    card_ents = []
    for k in range(K):
        masked = card_logits.masked_fill(remaining == 0, float("-inf"))
        dist = Categorical(logits=masked)
        idx = card_indices[:, k].clamp(min=0)
        valid = card_valid[:, k]
        log_p = torch.where(
            valid > 0, dist.log_prob(idx), torch.zeros(B, device=idx.device)
        )
        ent = torch.where(
            valid > 0, dist.entropy(), torch.zeros(B, device=idx.device)
        )
        card_log_probs.append(log_p)
        card_ents.append(ent)
        scatter_idx = torch.where(
            valid.unsqueeze(1) > 0,
            idx.unsqueeze(1),
            torch.zeros_like(idx.unsqueeze(1)),
        )
        remaining = remaining.clone().scatter_(1, scatter_idx, 0.0)

    log_probs = mode_log + count_log + torch.stack(card_log_probs).sum(0)
    entropies = mode_ent + count_ent + torch.stack(card_ents).sum(0)
    return log_probs, entropies
