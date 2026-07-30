import pytest
import torch

from core.models import GameState
from simulation.blind_env import ActionMasks
from simulation.blind_trainer import BlindModel
from simulation.decoder import (
    evaluate_actions,
    model_decoder,
    model_decoder_batch,
)
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

    assert calculate_score_progress_reward(0, state) == pytest.approx(0.0425)
    assert calculate_terminal_reward(state) == -10.0
    assert calculate_game_score(state) == -9.75

    state.current_score = 400
    state.hands = 2
    state.hands_played = 2
    assert calculate_score_progress_reward(100, state) == pytest.approx(0.1675)
    assert calculate_terminal_reward(state) == pytest.approx(10.1)
    assert calculate_game_score(state) == pytest.approx(11.1)


def test_hand_cost_accumulates_across_every_play():
    state = GameState(score_to_beat=300)

    state.current_score = 50
    first_weak_play = calculate_score_progress_reward(0, state)

    state.current_score = 100
    second_weak_play = calculate_score_progress_reward(50, state)

    state.current_score = 300
    winning_play = calculate_score_progress_reward(100, state)

    assert first_weak_play == pytest.approx(13 / 600)
    assert second_weak_play == pytest.approx(13 / 600)
    assert winning_play == pytest.approx(11 / 75)
    assert first_weak_play + second_weak_play + winning_play == pytest.approx(
        0.19
    )


@pytest.mark.parametrize(
    ("hands_played", "hands_remaining", "expected_reward"),
    [
        (1, 3, 10.15),
        (2, 2, 10.10),
        (3, 1, 10.05),
        (4, 0, 10.0),
    ],
)
def test_terminal_reward_adds_small_bonus_per_preserved_hand(
    hands_played: int,
    hands_remaining: int,
    expected_reward: float,
):
    state = GameState(score_to_beat=300)
    state.current_score = 300
    state.hands_played = hands_played
    state.hands = hands_remaining

    assert calculate_terminal_reward(state) == pytest.approx(expected_reward)


def test_score_reward_prefers_fewer_plays_for_equal_progress():
    state = GameState(score_to_beat=300)

    previous_score = 0.0
    small_hand_reward = 0.0
    for current_score in (50.0, 100.0, 150.0):
        state.current_score = current_score
        small_hand_reward += calculate_score_progress_reward(previous_score, state)
        previous_score = current_score

    state.current_score = 150.0
    large_hand_reward = calculate_score_progress_reward(0.0, state)

    assert large_hand_reward > small_hand_reward


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


def test_batched_decoder_uses_the_correct_heads_for_each_row():
    play = _outputs(mode=0)
    discard = _outputs(mode=1)
    outputs = {
        key: torch.cat((play[key], discard[key]))
        for key in play
    }
    masks = ActionMasks(
        mode=torch.ones(2, 2),
        count=torch.ones(2, 5),
        card=torch.ones(2, 8),
    )

    modes, counts, cards, valid, _, _ = model_decoder_batch(
        outputs, masks, stochastic=False
    )

    assert modes.tolist() == [0, 1]
    assert counts.tolist() == [2, 5]
    assert cards.tolist() == [[0, 1, -1, -1, -1], [3, 4, 5, 6, 7]]
    assert valid.tolist() == [[1.0, 1.0, 0.0, 0.0, 0.0], [1.0] * 5]


def test_batched_decoder_log_probs_match_ppo_action_evaluation():
    torch.manual_seed(123)
    model = BlindModel(input_size=6, hidden_size=8)
    outputs = model(torch.randn(4, 6))
    masks = ActionMasks(
        mode=torch.ones(4, 2),
        count=torch.ones(4, 5),
        card=torch.ones(4, 8),
    )
    modes, counts, cards, valid, sampled_log_probs, _ = model_decoder_batch(
        outputs, masks, stochastic=True
    )

    evaluated_log_probs, _ = evaluate_actions(
        outputs,
        masks.mode,
        masks.count,
        masks.card,
        modes,
        counts,
        cards,
        valid,
    )

    assert torch.allclose(sampled_log_probs, evaluated_log_probs)
