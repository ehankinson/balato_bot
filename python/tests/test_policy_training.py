import torch

from core.models import GameState
from simulation.blind_env import ActionMasks
from simulation.blind_trainer import BlindModel
from simulation.decoder import evaluate_actions, model_decoder
from simulation.reward import (
    calculate_game_score,
    calculate_score_progress_reward,
    calculate_terminal_reward,
)


def _outputs(mode: int) -> dict[str, torch.Tensor]:
    mode_logits = [-10.0, -10.0]
    mode_logits[mode] = 10.0
    return {
        "mode_logits": torch.tensor([mode_logits]),
        "play_count_logits": torch.tensor([[0.0, 10.0, 0.0, 0.0, 0.0]]),
        "discard_count_logits": torch.tensor([[0.0, 0.0, 0.0, 0.0, 10.0]]),
        "play_card_logits": torch.tensor(
            [[10.0, 9.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]]
        ),
        "discard_card_logits": torch.tensor(
            [[0.0, 0.0, 0.0, 10.0, 9.0, 8.0, 7.0, 6.0]]
        ),
    }


def _masks() -> ActionMasks:
    return ActionMasks(
        mode=torch.ones(2),
        count=torch.ones(5),
        card=torch.ones(8),
    )


def test_decoder_uses_play_specific_count_and_card_heads():
    mode, count, cards, _, _ = model_decoder(
        _outputs(mode=0), _masks(), torch.device("cpu"), stochastic=False
    )

    assert int(mode) == 0
    assert count == 2
    assert cards == [0, 1]


def test_decoder_uses_discard_specific_count_and_card_heads():
    mode, count, cards, _, _ = model_decoder(
        _outputs(mode=1), _masks(), torch.device("cpu"), stochastic=False
    )

    assert int(mode) == 1
    assert count == 5
    assert cards == [3, 4, 5, 6, 7]


def test_reward_values_progress_and_wins_without_rewarding_discard_usage():
    state = GameState(score_to_beat=400)
    state.current_score = 100
    state.discards_used = 3
    state.discards = 0

    assert calculate_score_progress_reward(0, state) == 0.25
    assert calculate_terminal_reward(state) == -1.0
    assert calculate_game_score(state) == -0.75

    state.current_score = 400
    state.hands = 2
    assert calculate_score_progress_reward(100, state) == 0.75
    assert calculate_terminal_reward(state) == 5.5
    assert calculate_game_score(state) == 6.5


def test_ppo_evaluation_updates_both_mode_specific_heads():
    model = BlindModel(input_size=6, hidden_size=8)
    outputs = model(torch.randn(2, 6))
    log_probs, entropies = evaluate_actions(
        outputs,
        mode_masks=torch.ones(2, 2),
        count_masks=torch.ones(2, 5),
        card_masks=torch.ones(2, 8),
        modes=torch.tensor([0, 1]),
        counts=torch.tensor([2, 3]),
        card_indices=torch.tensor([[0, 1, -1, -1, -1], [2, 3, 4, -1, -1]]),
        card_valid=torch.tensor(
            [[1.0, 1.0, 0.0, 0.0, 0.0], [1.0, 1.0, 1.0, 0.0, 0.0]]
        ),
    )

    assert torch.isfinite(log_probs).all()
    assert torch.isfinite(entropies).all()
    loss = -(log_probs.mean() + 0.01 * entropies.mean())
    loss.backward()
    assert model.play_count_head.weight.grad is not None
    assert model.discard_count_head.weight.grad is not None
    assert model.play_card_head.weight.grad is not None
    assert model.discard_card_head.weight.grad is not None
