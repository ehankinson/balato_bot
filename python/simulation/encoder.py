"""Schema-v1 tensor adapter. Counts get log1p; Rust float features stay as encoded."""

from dataclasses import dataclass

import numpy as np
import torch

SCHEMA_VERSION = 1


class UnsupportedObservation(ValueError):
    pass


@dataclass(frozen=True)
class ResourceLimits:
    max_collection_rows: int = 512
    max_tokens: int = 1024
    max_batch_attention_cells: int = 4_194_304

    def __post_init__(self):
        if (
            min(
                self.max_collection_rows,
                self.max_tokens,
                self.max_batch_attention_cells,
            )
            <= 0
        ):
            raise ValueError("resource limits must be positive")


@dataclass(frozen=True)
class RowSpec:
    categories: tuple[int, ...]  # maximum valid categorical ID per column
    numbers: int
    flags: int
    counts: int = 0
    positioned: bool = True


TABLES = {
    "hand": RowSpec((14, 3, 8, 3, 4), 3, 3),
    "jokers": RowSpec((150, 3, 4), 1, 5),
    "consumables": RowSpec((3, 22), 1, 1),
    "last_consumable": RowSpec((3, 22), 1, 1),
    "deck_catalogue": RowSpec((14, 3, 8, 3, 4), 3, 0, 1, False),
    "poker_hands": RowSpec((12,), 2, 0, 2, False),
    "vouchers": RowSpec((32,), 0, 0),
    "tags": RowSpec((24, 12), 1, 0),
    "blind_offers": RowSpec((2, 28, 24), 2, 2),
    "shop_items": RowSpec((3, 150, 3, 14, 3, 8, 3, 4, 4, 3), 5, 5),
    "shop_vouchers": RowSpec((32,), 1, 0),
    "shop_packs": RowSpec((4, 2), 1, 0),
    "pack_options": RowSpec((3, 150, 3, 14, 3, 8, 3, 4, 4, 3), 5, 5),
}
CONTEXT = {
    "run": RowSpec((5, 15, 8, 2, 1), 20, 38, positioned=False),
    "blind": RowSpec((2, 28, 12, 12), 7, 15, positioned=False),
    "shop": RowSpec((), 1, 0, positioned=False),
    "pack": RowSpec((4, 2, 5), 1, 0, positioned=False),
}


@dataclass
class Rows:
    cat: torch.Tensor
    num: torch.Tensor
    flags: torch.Tensor
    visible: torch.Tensor
    present: torch.Tensor


def count_features(counts):
    values = counts.to(torch.float32)
    return values.sign() * values.abs().log1p()


def _tensor(observation, key, dtype, shape):
    array = observation[key]
    if (
        not isinstance(array, np.ndarray)
        or array.dtype != dtype
        or array.shape != shape
        or not array.flags.c_contiguous
        or not array.flags.aligned
    ):
        raise UnsupportedObservation(f"{key}: expected contiguous {dtype} {shape}")
    return torch.from_numpy(array)


def validate_observation(observation, limits):
    if "schema_version" not in observation or observation[
        "schema_version"
    ].tolist() != [SCHEMA_VERSION]:
        raise UnsupportedObservation(
            "incompatible observation schema; expected version 1"
        )
    total = len(CONTEXT)
    for name in (*TABLES, "joker_values"):
        length = len(observation[f"{name}.present"])
        if length > limits.max_collection_rows:
            raise UnsupportedObservation(
                f"{name}: {length} exceeds max_collection_rows"
            )
        if name != "joker_values":
            total += length
    if total > limits.max_tokens:
        raise UnsupportedObservation(
            f"{total} tokens exceeds max_tokens={limits.max_tokens}"
        )


FIXED_FIELDS = {
    "schema_version": (np.int32, 1),
    "sections": (np.bool_, 5),
    "run.cat": (np.int32, 5),
    "run.counts": (np.int64, 14),
    "run.num": (np.float32, 3),
    "run.extra": (np.int64, 3),
    "run.flags": (np.bool_, 3),
    "run.visible": (np.bool_, 28),
    "run.used_planets": (np.bool_, 12),
    "action_kinds": (np.bool_, 18),
    "blind.cat": (np.int32, 4),
    "blind.counts": (np.int64, 5),
    "blind.num": (np.float32, 2),
    "blind.flags": (np.bool_, 3),
    "blind.visible": (np.bool_, 14),
    "blind.played_types": (np.bool_, 12),
    "shop.num": (np.float32, 1),
    "shop.visible": (np.bool_, 1),
    "pack.cat": (np.int32, 3),
    "pack.counts": (np.int64, 1),
    "pack.visible": (np.bool_, 4),
}


def tensor_observation(observation, limits=None):
    """Share CPU snapshot storage; do not mutate Rust's independently owned arrays."""
    limits = limits if limits is not None else ResourceLimits()
    validate_observation(observation, limits)
    rows = {}
    for name, spec in TABLES.items():
        n = len(observation[f"{name}.present"])
        c, f = len(spec.categories), spec.flags
        rows[name] = Rows(
            _tensor(observation, f"{name}.cat", np.int32, (n, c)),
            _tensor(observation, f"{name}.num", np.float32, (n, spec.numbers)),
            _tensor(observation, f"{name}.flags", np.bool_, (n, f)),
            _tensor(
                observation, f"{name}.visible", np.bool_, (n, c + spec.numbers + f)
            ),
            _tensor(observation, f"{name}.present", np.bool_, (n,)),
        )
        if name in {"deck_catalogue", "poker_hands"}:
            key = (
                "deck_multiplicity" if name == "deck_catalogue" else "poker_hand_counts"
            )
            shape = (n,) if name == "deck_catalogue" else (n, 2)
            counts = _tensor(observation, key, np.int64, shape).reshape(n, spec.counts)
            # Unknown catalogue properties do not conceal the public multiplicity.
            rows[name].num = torch.cat((rows[name].num, count_features(counts)), -1)
            vis = rows[name].visible
            rows[name].visible = torch.cat(
                (
                    vis[:, : c + spec.numbers],
                    rows[name].present[:, None].expand(-1, spec.counts),
                    vis[:, c + spec.numbers :],
                ),
                -1,
            )
    # Fixed contexts retain their column visibility in the same order as inputs.
    fixed = {
        key: _tensor(observation, key, dtype, (length,))
        for key, (dtype, length) in FIXED_FIELDS.items()
    }
    t = fixed.__getitem__
    rv = t("run.visible")
    run_num = torch.cat(
        (count_features(t("run.counts")), t("run.num"), count_features(t("run.extra")))
    )
    run_flags = torch.cat(
        (t("run.flags"), t("run.used_planets"), t("sections"), t("action_kinds"))
    )
    rows["run"] = Rows(
        t("run.cat")[None],
        run_num[None],
        run_flags[None],
        torch.cat((rv, torch.ones(35, dtype=torch.bool)))[None],
        torch.ones(1, dtype=torch.bool),
    )
    rows["blind"] = Rows(
        t("blind.cat")[None],
        torch.cat((count_features(t("blind.counts")), t("blind.num")))[None],
        torch.cat((t("blind.flags"), t("blind.played_types")))[None],
        torch.cat((t("blind.visible"), t("sections")[1].expand(12)))[None],
        torch.ones(1, dtype=torch.bool),
    )
    rows["shop"] = Rows(
        torch.empty((1, 0), dtype=torch.int32),
        t("shop.num")[None],
        torch.empty((1, 0), dtype=torch.bool),
        t("shop.visible")[None],
        torch.ones(1, dtype=torch.bool),
    )
    rows["pack"] = Rows(
        t("pack.cat")[None],
        count_features(t("pack.counts"))[None],
        torch.empty((1, 0), dtype=torch.bool),
        t("pack.visible")[None],
        torch.ones(1, dtype=torch.bool),
    )
    return rows


BATCH_TABLES = (*TABLES, "joker_values")


def unbatch_observations(batch):
    """Owned batch arrays yield retained CPU views; no padding becomes a slot."""
    count = len(batch["schema_version"])
    observations = []
    for index in range(count):
        observation = {}
        for name, array in batch.items():
            if name.startswith("lengths."):
                continue
            collection = name.split(".")[0]
            if collection in BATCH_TABLES:
                length = int(batch[f"lengths.{collection}"][index])
                observation[name] = array[index, :length]
            elif name == "deck_multiplicity":
                observation[name] = array[
                    index, : int(batch["lengths.deck_catalogue"][index])
                ]
            else:
                observation[name] = array[index]
        observations.append(observation)
    return observations
