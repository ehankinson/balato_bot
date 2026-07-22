import torch
import torch.nn.functional as F

from core.enums import PokerHand
from core.models import Card, FinalScoringResults, GameState, HandScoring

RANK_FEATURES = 13
SUIT_FEATURES = 4
POKER_HAND_FEATURES = 12
SCORING_ROLE_FEATURE = 4
CARD_FEATURES = RANK_FEATURES + SUIT_FEATURES + SCORING_ROLE_FEATURE

DISCARD_POKER_HANDS: tuple[PokerHand, ...] = tuple(
    hand for hand in PokerHand if hand != PokerHand.HIGH_CARD
)


def encode_card(card: Card, role: int) -> torch.Tensor:
    rank = F.one_hot(torch.tensor(int(card.rank)), num_classes=RANK_FEATURES)
    suit = F.one_hot(torch.tensor(int(card.suit)), num_classes=SUIT_FEATURES)
    scoring_role = F.one_hot(torch.tensor(role), num_classes=SCORING_ROLE_FEATURE)

    return torch.cat((rank, suit, scoring_role)).float()


def encode_cards(
    hand_size: int, hand: list[Card], hand_scoring: HandScoring
) -> torch.Tensor:
    encoded_cards = []
    for card in hand:
        if card in hand_scoring.scored_played:
            scoring_role = 0
        elif card in hand_scoring.scored_held:
            scoring_role = 1
        elif card in hand_scoring.unscored_played:
            scoring_role = 2
        else:
            scoring_role = 3

        encoded_cards.append(encode_card(card, scoring_role))

    padding = hand_size - len(encoded_cards)
    encoded_cards.extend(torch.zeros(CARD_FEATURES) for _ in range(padding))
    return torch.stack(encoded_cards)


def build_discard_bitmask(hand: list[Card], discard_list: list[Card]) -> torch.Tensor:
    bit_mask = [1 if card in discard_list else 0 for card in hand]
    return torch.tensor(bit_mask, dtype=torch.float32)


def encode_hand_stats(hand_stats_name: int) -> torch.Tensor:
    index = max(0, int(hand_stats_name) - 1)
    return F.one_hot(torch.tensor(index), num_classes=POKER_HAND_FEATURES).float()


def encode_game_state(
    hand: list[Card],
    game_state: GameState,
    best_hand: FinalScoringResults,
    discards: dict[PokerHand, tuple[int, float, list[Card]]],
) -> torch.Tensor:
    best_hand_score = best_hand.best_hand.chips * best_hand.best_hand.worst_case_mult
    projected_best_score = best_hand_score + game_state.current_score

    card_feature = encode_cards(game_state.hand_size, hand, best_hand.hand_scoring)
    hand_stats_feature = encode_hand_stats(best_hand.hand_scoring.hand_stats.name)
    game_state_feature = torch.tensor(
        [
            game_state.current_score / game_state.score_to_beat,
            game_state.hands
            / max(game_state.hands + game_state.hands_played, 1),
            game_state.discards
            / max(game_state.discards + game_state.discards_used, 1),
            projected_best_score / game_state.score_to_beat,
        ],
        dtype=torch.float32,
    )

    discard_features = []
    for poker_hand in DISCARD_POKER_HANDS:
        discard_info = discards.get(poker_hand)
        if discard_info is None:
            info = torch.zeros(1)
            bitmask = torch.zeros(game_state.hand_size)
        else:
            info = torch.tensor([discard_info[1]], dtype=torch.float32)
            bitmask = build_discard_bitmask(hand, discard_info[2])

        feature = torch.cat((info, bitmask)).float()
        discard_features.append(feature)

    state_features = [
        card_feature.flatten(),
        hand_stats_feature,
        game_state_feature,
    ]
    for f in discard_features:
        state_features.append(f.flatten())

    return torch.cat(state_features)


def observation_dim(hand_size: int = 8) -> int:
    card = hand_size * CARD_FEATURES
    hand_stats = POKER_HAND_FEATURES
    game_state = 4
    discards = len(DISCARD_POKER_HANDS) * (1 + hand_size)
    return card + hand_stats + game_state + discards
