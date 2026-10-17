from copy import deepcopy
from types import SimpleNamespace

import numpy as np
import pytest
import torch

import balatro_engine as engine
from simulation.encoder import (
    TABLES,
    ResourceLimits,
    UnsupportedObservation,
    tensor_observation,
)
from simulation.policy import GamePolicy, ModelConfig
from simulation.ppo import compute_gae
from simulation.reward import transition_reward
from simulation.rollout import Episode
from simulation.training_config import TrainingConfig


@pytest.fixture(scope="module", autouse=True)
def single_threaded():
    previous = torch.get_num_threads()
    torch.set_num_threads(1)
    yield
    torch.set_num_threads(previous)


def observation(hand_size=8):
    game = engine.new(0, deck=engine.Deck.Magic)
    game.apply_action(engine.Action.SelectBlind, source=0)
    result = game.observe()
    for suffix in ("cat", "num", "flags", "present", "visible"):
        key = f"hand.{suffix}"
        result[key] = result[key][np.arange(hand_size) % 8].copy()
    result["run.counts"][2] = result["run.counts"][9] = hand_size
    return result


def with_jokers(obs, count=2):
    obs = deepcopy(obs)
    for suffix, array in {
        "cat": np.array([[103, 1, 0]] * count, dtype=np.int32),
        "num": np.zeros((count, 1), dtype=np.float32),
        "flags": np.zeros((count, 5), dtype=np.bool_),
        "visible": np.ones((count, 9), dtype=np.bool_),
        "present": np.ones(count, dtype=np.bool_),
    }.items():
        obs[f"jokers.{suffix}"] = array
    return obs


def model():
    torch.manual_seed(7)
    return GamePolicy(ModelConfig(width=16, heads=2, depth=1)).eval()


@pytest.mark.parametrize("delay", [0, 1, 20])
def test_undiscounted_terminal_loss_return_is_not_improved_by_delaying(delay):
    config = TrainingConfig()
    before = engine.new(0).observe()
    lost = deepcopy(before)
    lost["run.flags"][0] = True
    neutral_reward = transition_reward(before, before, True, config.rewards)
    loss_reward = transition_reward(before, lost, True, config.rewards)
    assert neutral_reward == 0
    assert loss_reward == -25
    episodes = [
        Episode(
            transitions=tuple(
                SimpleNamespace(
                    reward=reward,
                    old_value=2.0,
                    terminated=index == len(rewards) - 1,
                )
                for index, reward in enumerate(rewards)
            ),
            bootstrap_value=999.0,
            end_reason="loss",
            accepted_commands=len(rewards),
        )
        for rewards in ([loss_reward], [neutral_reward] * delay + [loss_reward])
    ]
    # Lambda=1 gives full returns; default GAE targets also depend on the critic.
    _, returns, _ = compute_gae(episodes, gamma=config.gamma, gae_lambda=1.0)
    torch.testing.assert_close(returns, torch.full((delay + 2,), -25.0))


@pytest.mark.parametrize(
    "end_reason, reward, expected",
    [
        ("loss", -25.0, [-23.55, -25.0]),
        ("win", 25.0, [23.95, 25.0]),
        ("horizon", 0.0, [9.7, 10.0]),
        ("rejection_limit", -0.1, [9.605, 9.9]),
    ],
)
def test_undiscounted_default_preserves_gae_lambda_and_cutoff_bootstrap(
    end_reason, reward, expected
):
    config = TrainingConfig()
    terminal = end_reason in {"loss", "win"}
    episode = Episode(
        transitions=(
            SimpleNamespace(reward=0.0, old_value=2.0, terminated=False),
            SimpleNamespace(reward=reward, old_value=4.0, terminated=terminal),
        ),
        bootstrap_value=10.0,
        end_reason=end_reason,
        accepted_commands=1 if end_reason == "rejection_limit" else 2,
    )
    _, returns, _ = compute_gae(
        [episode], gamma=config.gamma, gae_lambda=config.gae_lambda
    )
    torch.testing.assert_close(returns, torch.tensor(expected))


def test_same_weights_keep_individual_rows_for_five_and_sixteen_cards():
    policy = model()
    output = policy([observation(5), observation(16), engine.new(1).observe()])
    assert output.action_logits.shape == (3, 18)
    assert output.value.shape == (3,)
    assert torch.isfinite(output.value).all()
    for index, size in enumerate([5, 16, 0]):
        rows = output.objects(index, "hand")
        assert rows.shape == (size, 16)
        assert policy.selection_logits(rows, policy.query(output, index, 2)).shape == (
            size + 1,
        )
    assert output.objects(2, "jokers").shape == (0, 16)


@pytest.mark.parametrize(
    "device",
    [
        "cpu",
        pytest.param(
            "mps",
            marks=pytest.mark.skipif(
                not torch.backends.mps.is_available(), reason="MPS unavailable"
            ),
        ),
    ],
)
@pytest.mark.parametrize("contents", ["initial", "mixed", "joker_values"])
def test_backward_handles_empty_collections_and_optional_joker_values(device, contents):
    initial = engine.new(1).observe()
    observations = [initial]
    if contents != "initial":
        populated = with_jokers(observation(5))
        if contents == "joker_values":
            populated["joker_values.cat"] = np.array([[0, 0, 1]], dtype=np.int32)
            populated["joker_values.num"] = np.array([[1.0]], dtype=np.float32)
            populated["joker_values.flags"] = np.empty((1, 0), dtype=np.bool_)
            populated["joker_values.present"] = np.ones(1, dtype=np.bool_)
            populated["joker_values.visible"] = np.ones((1, 4), dtype=np.bool_)
        observations.insert(0, populated)
    policy = model().to(device).train()
    before = policy.attention[0].self_attn.in_proj_weight.detach().clone()
    optimizer = torch.optim.Adam(policy.parameters(), lr=1e-3)
    output = policy(observations)
    game = len(observations) - 1
    empty = output.objects(game, "jokers")
    assert empty.shape == (0, policy.config.width)
    stop = policy.selection_logits(empty, policy.query(output, game, 15))
    assert stop.shape == (1,)
    loss = (
        output.value.square().mean()
        + output.action_logits.square().mean()
        + stop.square().sum()
    )
    loss.backward()
    assert all(
        torch.isfinite(parameter.grad).all()
        for parameter in policy.parameters()
        if parameter.grad is not None
    )
    if contents == "initial":
        assert all(
            parameter.grad is None for parameter in policy.encoders["hand"].parameters()
        )
        assert policy.joker_suit.weight.grad is None
    if contents == "joker_values":
        assert policy.joker_value_encoder.features[0].weight.grad is not None
    optimizer.step()
    assert not torch.equal(before, policy.attention[0].self_attn.in_proj_weight)


def test_padding_and_other_games_do_not_change_valid_outputs():
    policy = model()
    short, long = observation(5), observation(16)
    with torch.no_grad():
        alone = policy([short])
        batched = policy([short, long])
        changed = deepcopy(long)
        changed["run.num"][0] += 20
        isolated = policy([short, changed])
    for result in (batched, isolated):
        torch.testing.assert_close(
            alone.action_logits[0], result.action_logits[0], rtol=1e-5, atol=1e-6
        )
        torch.testing.assert_close(
            alone.value[0], result.value[0], rtol=1e-5, atol=1e-6
        )
        for name in TABLES:
            torch.testing.assert_close(
                alone.objects(0, name), result.objects(0, name), rtol=1e-5, atol=1e-6
            )


def test_unknown_values_do_not_leak_and_known_zero_is_distinct():
    policy = model()
    hidden = observation()
    hidden["hand.visible"][0, :8] = False
    altered = deepcopy(hidden)
    altered["hand.cat"][0] = 2**31 - 1
    altered["hand.num"][0] = np.nan
    expected, actual = policy([hidden]), policy([altered])
    torch.testing.assert_close(expected.action_logits, actual.action_logits)
    torch.testing.assert_close(expected.objects(0, "hand"), actual.objects(0, "hand"))
    zero = deepcopy(hidden)
    zero["hand.num"][0, 0] = 0
    zero["hand.visible"][0, 5] = True
    assert not torch.allclose(
        expected.objects(0, "hand"), policy([zero]).objects(0, "hand")
    )


def test_catalogue_multiplicity_matters_but_its_order_is_not_draw_order():
    policy = model()
    obs = observation()
    repeated = deepcopy(obs)
    repeated["deck_multiplicity"][0] = 10
    assert not torch.allclose(
        policy([obs]).action_logits, policy([repeated]).action_logits
    )
    reversed_obs = deepcopy(obs)
    for suffix in ("cat", "num", "flags", "present", "visible"):
        key = f"deck_catalogue.{suffix}"
        reversed_obs[key] = np.ascontiguousarray(reversed_obs[key][::-1])
    reversed_obs["deck_multiplicity"] = reversed_obs["deck_multiplicity"][::-1].copy()
    torch.testing.assert_close(
        policy([obs]).action_logits,
        policy([reversed_obs]).action_logits,
        atol=1e-6,
        rtol=1e-5,
    )


def test_hand_selection_depends_on_inventory_and_context_with_gradient_flow():
    policy = model()
    obs = with_jokers(observation())
    output = policy([obs])
    scores = policy.selection_logits(
        output.objects(0, "hand"), policy.query(output, 0, 2)
    )
    scores[0].backward()
    for name in ["hand", "jokers", "consumables", "run"]:
        assert any(
            parameter.grad is not None and parameter.grad.abs().sum() > 0
            for parameter in policy.encoders[name].parameters()
        ), name
    assert policy.attention[0].self_attn.in_proj_weight.grad.abs().sum() > 0
    assert policy.target_query.weight.grad.abs().sum() > 0
    for key in ["jokers.cat", "consumables.cat", "run.num"]:
        changed = deepcopy(obs)
        if changed[key].ndim == 1:
            changed[key][0] += 1
        else:
            changed[key][0, 0] += 1
        actual = policy([changed])
        new_scores = policy.selection_logits(
            actual.objects(0, "hand"), policy.query(actual, 0, 2)
        )
        assert not torch.allclose(scores, new_scores), key


@pytest.mark.parametrize(
    "parent,collection", [(0, "jokers"), (1, "shop_items"), (2, "pack_options")]
)
def test_linked_joker_values_modify_the_correct_parent_before_attention(
    parent, collection
):
    policy = model()
    obs = with_jokers(observation())
    if collection != "jokers":
        for suffix, array in {
            "cat": np.zeros((2, 10), dtype=np.int32),
            "num": np.zeros((2, 5), dtype=np.float32),
            "flags": np.zeros((2, 5), dtype=np.bool_),
            "present": np.ones(2, dtype=np.bool_),
            "visible": np.ones((2, 20), dtype=np.bool_),
        }.items():
            obs[f"{collection}.{suffix}"] = array
    obs.update(
        {
            "joker_values.cat": np.array([[parent, 0, 1]], dtype=np.int32),
            "joker_values.num": np.array([[3.0]], dtype=np.float32),
            "joker_values.flags": np.empty((1, 0), dtype=np.bool_),
            "joker_values.present": np.ones(1, dtype=np.bool_),
            "joker_values.visible": np.ones((1, 4), dtype=np.bool_),
        }
    )
    inputs = []
    handle = policy.attention[0].register_forward_pre_hook(
        lambda _, args: inputs.append(args[0].detach().clone())
    )
    policy([obs])
    other = deepcopy(obs)
    other["joker_values.cat"][0, 1] = 1
    policy([other])
    handle.remove()
    delta = (inputs[0] - inputs[1]).abs().sum(-1)[0]
    offset = 4
    for name in TABLES:
        if name == collection:
            break
        offset += len(obs[f"{name}.present"])
    assert torch.nonzero(delta).flatten().tolist() == [offset, offset + 1]
    broken = deepcopy(obs)
    broken["joker_values.cat"][0, 1] = 2
    with pytest.raises(UnsupportedObservation, match="parent slot"):
        policy([broken])


def test_roles_and_positions_participate_and_poker_counts_are_attached():
    policy = model()
    obs = with_jokers(observation())
    out = policy([obs])
    # Identical owned Joker properties still have different slot representations.
    assert not torch.allclose(out.objects(0, "jokers")[0], out.objects(0, "jokers")[1])
    out.action_logits.sum().backward()
    assert policy.roles.weight.grad.abs().sum() > 0
    assert policy.positions.weight.grad.abs().sum() > 0
    changed = deepcopy(obs)
    changed["poker_hand_counts"][0] += 10
    assert not torch.allclose(out.action_logits, policy([changed]).action_logits)


def test_adapter_keeps_categorical_dtype_and_does_not_retransform_rust_floats():
    obs = observation()
    rows = tensor_observation(obs)
    assert rows["hand"].cat.dtype == torch.int32
    assert rows["hand"].cat.data_ptr() == obs["hand.cat"].ctypes.data
    np.testing.assert_array_equal(rows["hand"].num.numpy(), obs["hand.num"])
    assert rows["deck_catalogue"].num[0, -1] == pytest.approx(
        np.log1p(obs["deck_multiplicity"][0])
    )


@pytest.mark.parametrize(
    "limits,obs",
    [
        (ResourceLimits(max_collection_rows=60), observation(61)),
        (ResourceLimits(max_tokens=10), observation()),
        (ResourceLimits(max_batch_attention_cells=1), observation()),
    ],
)
def test_resource_limits_report_unsupported_without_truncation(limits, obs):
    policy = GamePolicy(ModelConfig(16, 2, 1), limits)
    with pytest.raises(UnsupportedObservation):
        policy([obs])


def test_bad_schema_and_visible_values_fail_explicitly():
    policy = model()
    obs = observation()
    obs["schema_version"][0] = 2
    with pytest.raises(UnsupportedObservation, match="schema"):
        policy([obs])
    obs = observation()
    obs["hand.cat"][0, 0] = 100
    with pytest.raises(UnsupportedObservation, match="categorical"):
        policy([obs])
    obs = observation()
    obs["hand.num"][0, 0] = np.inf
    with pytest.raises(UnsupportedObservation, match="non-finite"):
        policy([obs])


@pytest.mark.parametrize(
    "key,dtype",
    [
        ("run.cat", np.float32),
        ("blind.counts", np.int32),
        ("hand.num", np.float64),
    ],
)
def test_adapter_rejects_schema_dtype_mismatches_instead_of_coercing(key, dtype):
    obs = observation()
    obs[key] = obs[key].astype(dtype)
    with pytest.raises(UnsupportedObservation, match=key):
        model()([obs])


def test_joker_tooltip_values_remain_associated_with_their_feature_keys():
    policy = model()
    obs = with_jokers(observation(), count=1)
    obs.update(
        {
            "joker_values.cat": np.array([[0, 0, 1], [0, 0, 2]], dtype=np.int32),
            "joker_values.num": np.array([[1.0], [4.0]], dtype=np.float32),
            "joker_values.flags": np.empty((2, 0), dtype=np.bool_),
            "joker_values.present": np.ones(2, dtype=np.bool_),
            "joker_values.visible": np.ones((2, 4), dtype=np.bool_),
        }
    )
    swapped = deepcopy(obs)
    swapped["joker_values.num"] = swapped["joker_values.num"][::-1].copy()
    assert not torch.allclose(
        policy([obs]).objects(0, "jokers"), policy([swapped]).objects(0, "jokers")
    )
