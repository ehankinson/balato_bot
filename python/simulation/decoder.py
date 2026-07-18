import torch
from typing import Any


def blind_decoder(values: dict[str, Any]):
    mode = torch.argmax(values["mode_logits"], dim=-1).item()
    count = int(torch.argmax(values["count_logits"], dim=-1).item() + 1)
    card_indices = torch.topk(values["card_logits"], k=count).indices.tolist()

    return mode, count, card_indices