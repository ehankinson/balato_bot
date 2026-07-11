from dataclasses import fields

import torch
import torch.nn.functional as F

from core.enums import PokerHand
from core.models import Card, FinalScoringResults, GameState, HandScoring

RANK_FEATURES = 13
SUIT_FEATURES = 4
POKER_HAND_FEATURES = 12
SCORING_ROLE_FEATURE = 4
CARD_FEATURES = RANK_FEATURES + SUIT_FEATURES + SCORING_ROLE_FEATURE


def encode_card(card: Card, role: int) -> torch.Tensor:
    rank = F.one_hot(torch.tensor(int(card.rank)), num_classes=RANK_FEATURES)
    suit = F.one_hot(torch.tensor(int(card.suit)), num_classes=SUIT_FEATURES)
    scoring_role = F.one_hot(torch.tensor(role), num_classes=SCORING_ROLE_FEATURE)

    return torch.cat((rank, suit, scoring_role)).float()


def encode_cards(hand_size: int, hand_scoring: HandScoring) -> torch.Tensor:
    encoded_cards = [
        encode_card(card, role - 1)
        for role, field in enumerate(fields(hand_scoring))
        for card in getattr(hand_scoring, field.name)
        if field.name != "hand_stats"
    ]

    padding = hand_size - len(encoded_cards)
    encoded_cards.extend(torch.zeros(CARD_FEATURES) for _ in range(padding))
    return torch.stack(encoded_cards)


def build_discard_bitmask(hand: list[Card], discard_list: list[Card]) -> torch.Tensor:
    bit_mask = [1 if card in discard_list else 0 for card in hand]
    return torch.tensor(bit_mask, dtype=torch.float32)


def encode_game_state(
    hand: list[Card],
    game_state: GameState,
    best_hand: FinalScoringResults,
    discards: dict[PokerHand, dict[str, int | float | list[Card]]],
):
    best_hand_score = best_hand.best_hand.chips * best_hand.best_hand.worst_case_mult
    projected_best_score = best_hand_score + game_state.current_score

    card_feature = encode_cards(game_state.hand_size, best_hand.hand_scoring)
    hand_stats_feature = F.one_hot(
        torch.tensor(best_hand.hand_scoring.hand_stats.name),
        num_classes=POKER_HAND_FEATURES,
    )
    game_state_feature = torch.tensor(
        [
            game_state.current_score / game_state.score_to_beat,
            game_state.hands_played / game_state.hands,
            game_state.discards_used / game_state.discards,
            projected_best_score / game_state.score_to_beat,
        ]
    )

    discard_features = []
    for discard_info in discards.values():
        info = torch.Tensor(
            [
                discard_info["probability"],
                discard_info["value"],
            ]
        )
        feature = torch.cat(
            (info, build_discard_bitmask(hand, discard_info["discard"]))
        ).float()
        discard_features.append(feature)

    pass
