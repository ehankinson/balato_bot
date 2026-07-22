from core.enums import PokerHand
from simulation.blind_env import (
    BlindEnv,
    MODE_DISCARD,
    MODE_PLAY,
    observation_dim,
)
from simulation.encoder import encode_hand_stats


def _make_env(seed=0):
    env = BlindEnv()
    env.reset(seed=seed)
    return env


def test_reset_returns_fixed_shape_observation_and_masks():
    env = BlindEnv()
    obs, masks = env.reset(seed=0)

    assert obs.shape == (observation_dim(8),)
    assert obs.dtype.is_floating_point
    assert masks.mode.shape == (2,)
    assert masks.count.shape == (5,)
    assert masks.card.shape == (8,)
    assert env.total_cards() == 52


def test_reset_is_deterministic_with_seed():
    env_a = BlindEnv()
    env_b = BlindEnv()
    obs_a, _ = env_a.reset(seed=42)
    obs_b, _ = env_b.reset(seed=42)
    assert obs_a.tolist() == obs_b.tolist()
    assert [c.card_id for c in env_a.hand] == [c.card_id for c in env_b.hand]


def test_play_action_decrements_hands_and_conserves_cards():
    env = _make_env()
    initial_hands = env.game_state.hands

    result = env.step((MODE_PLAY, 5, [0, 1, 2, 3, 4]))

    assert env.game_state.hands == initial_hands - 1
    assert env.game_state.hands_played == 1
    assert env.game_state.current_score > 0
    assert env.total_cards() == 52
    assert len(env.hand) == 8
    assert result.masks.card[: len(env.hand)].sum() == len(env.hand)


def test_discard_action_decrements_discards_and_refills_hand():
    env = _make_env()
    initial_discards = env.game_state.discards

    result = env.step((MODE_DISCARD, 3, [0, 1, 2]))

    assert env.game_state.discards == initial_discards - 1
    assert env.game_state.discards_used == 1
    assert env.game_state.current_score == 0
    assert env.total_cards() == 52
    assert len(env.hand) == 8
    assert result.terminated is False


def test_win_terminates_with_positive_reward():
    env = BlindEnv(score_to_beat=1, hands=4, discards=3)
    env.reset(seed=0)

    result = env.step((MODE_PLAY, 5, [0, 1, 2, 3, 4]))

    assert result.terminated is True
    assert result.info["won"] is True
    assert result.reward > 0
    assert env.total_cards() == 52


def test_loss_terminates_when_hands_run_out():
    env = BlindEnv(score_to_beat=10_000, hands=1, discards=0)
    obs, masks = env.reset(seed=0)

    result = env.step((MODE_PLAY, 5, [0, 1, 2, 3, 4]))

    assert result.terminated is True
    assert result.info["won"] is False
    assert result.reward > 0


def test_mode_mask_blocks_play_when_no_hands():
    env = BlindEnv(score_to_beat=10_000, hands=1, discards=3)
    env.reset(seed=0)

    env.step((MODE_PLAY, 1, [0]))

    masks = _last_masks(env)
    from simulation.blind_env import _build_masks

    masks = _build_masks(env.game_state, env.hand)
    assert masks.mode[0] == 0.0
    assert masks.mode[1] == 1.0


def test_observation_is_canonical_poker_hand_one_hot():
    stats = encode_hand_stats(PokerHand.FLUSH_FIVE)
    assert stats.sum().item() == 1.0
    assert stats[-1].item() == 1.0

    stats_high = encode_hand_stats(PokerHand.HIGH_CARD)
    assert stats_high[0].item() == 1.0
    assert stats_high.sum().item() == 1.0


def test_greedy_best_hand_playable_and_beats_small_target():
    env = BlindEnv(score_to_beat=1, hands=4, discards=3)
    env.reset(seed=7)
    best_hand = env._best_hand
    scored = best_hand.hand_scoring.scored_played
    assert scored
    indices = [i for i, c in enumerate(env.hand) if c in scored]
    assert 1 <= len(indices) <= 5

    result = env.step((MODE_PLAY, len(indices), indices))
    assert result.terminated is True
    assert result.info["won"] is True


def test_full_short_blind_run_terminates():
    env = BlindEnv(score_to_beat=300)
    env.reset(seed=11)
    terminated = False
    steps = 0
    while not terminated and steps < 50:
        result = env.step((MODE_PLAY, 5, [0, 1, 2, 3, 4]))
        terminated = result.terminated
        steps += 1
    assert terminated is True
    assert env.total_cards() == 52


def _last_masks(env):
    from simulation.blind_env import _build_masks

    return _build_masks(env.game_state, env.hand)