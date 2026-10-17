"""Joint per-object encoders, shared attention, and conditional pointer heads."""

from dataclasses import dataclass
from math import sqrt

import torch
from torch import nn
from torch.nn.utils.rnn import pad_sequence

from simulation.encoder import (
    CONTEXT,
    TABLES,
    ResourceLimits,
    RowSpec,
    UnsupportedObservation,
    tensor_observation,
)


@dataclass(frozen=True)
class ModelConfig:
    width: int = 64
    heads: int = 4
    depth: int = 2

    def __post_init__(self):
        if (
            self.width <= 0
            or self.heads <= 0
            or self.depth <= 0
            or self.width % self.heads
        ):
            raise ValueError(
                "width/depth/heads must be positive and width divisible by heads"
            )


class ObjectEncoder(nn.Module):
    def __init__(self, spec, width):
        super().__init__()
        self.spec = spec
        self.categories = nn.ModuleList(
            nn.Embedding(size + 2, width, padding_idx=0) for size in spec.categories
        )
        features = spec.numbers + spec.counts + spec.flags
        visibility = len(spec.categories) + features
        self.features = nn.Sequential(
            nn.Linear(features + visibility, width), nn.GELU(), nn.Linear(width, width)
        )
        # Mix identity with values before parent-linked feature rows are summed.
        # Otherwise swapping two tooltip values between keys would be invisible.
        self.output = nn.Sequential(
            nn.LayerNorm(width), nn.Linear(width, width), nn.GELU()
        )

    def prepare(self, rows):
        # Validate on CPU before transfer: no per-game GPU scalar reads.
        cat = rows.cat.long()
        visible = rows.visible & rows.present[:, None]
        c = len(self.categories)
        values = torch.cat((rows.num, rows.flags.float()), -1)
        values = torch.where(visible[:, c:], values, 0.0)
        if not torch.isfinite(values).all():
            raise UnsupportedObservation("non-finite visible features")
        ids = torch.where(visible[:, :c], cat + 1, 0)
        for column, maximum in enumerate(self.spec.categories):
            if ((ids[:, column] < 0) | (ids[:, column] > maximum + 1)).any():
                raise UnsupportedObservation("categorical ID outside schema registry")
        return Features(ids, torch.cat((values, visible.float()), -1))

    def forward(self, inputs):
        encoded = self.features(inputs.values)
        for column, embedding in enumerate(self.categories):
            encoded = encoded + embedding(inputs.ids[:, column])
        return self.output(encoded)


@dataclass
class Features:
    ids: torch.Tensor
    values: torch.Tensor

    def to(self, device):
        return Features(self.ids.to(device), self.values.to(device))


@dataclass
class PreparedBatch:
    features: dict[str, Features]
    positions: dict[str, torch.Tensor]
    present: dict[str, torch.Tensor]
    lengths: list[dict[str, int]]
    joker_features: Features
    joker_suits: torch.Tensor
    joker_hands: torch.Tensor
    links: dict[str, tuple[torch.Tensor, torch.Tensor]]

    def to(self, device):
        return PreparedBatch(
            {name: value.to(device) for name, value in self.features.items()},
            {name: value.to(device) for name, value in self.positions.items()},
            {name: value.to(device) for name, value in self.present.items()},
            self.lengths,
            self.joker_features.to(device),
            self.joker_suits.to(device),
            self.joker_hands.to(device),
            {
                name: (rows.to(device), slots.to(device))
                for name, (rows, slots) in self.links.items()
            },
        )


@dataclass
class PolicyOutput:
    state: torch.Tensor
    action_logits: torch.Tensor
    value: torch.Tensor
    rows: dict[str, torch.Tensor]
    lengths: list[dict[str, int]]

    def objects(self, game, collection):
        return self.rows[collection][game, : self.lengths[game][collection]]


class GamePolicy(nn.Module):
    def __init__(self, config=None, limits=None):
        super().__init__()
        config = config if config is not None else ModelConfig()
        limits = limits if limits is not None else ResourceLimits()
        self.config, self.limits = config, limits
        width = config.width
        self.encoders = nn.ModuleDict(
            {
                name: ObjectEncoder(spec, width)
                for name, spec in (CONTEXT | TABLES).items()
            }
        )
        self.roles = nn.Embedding(len(self.encoders), width)
        self.positions = nn.Embedding(limits.max_collection_rows, width)
        self.joker_value_encoder = ObjectEncoder(
            RowSpec((20,), 1, 13, positioned=False), width
        )
        self.joker_suit = nn.Embedding(5, width, padding_idx=0)
        self.joker_hand = nn.Embedding(14, width, padding_idx=0)
        self.attention = nn.ModuleList(
            nn.TransformerEncoderLayer(
                width,
                config.heads,
                width * 4,
                dropout=0.0,
                batch_first=True,
                norm_first=True,
            )
            for _ in range(config.depth)
        )
        self.norm = nn.LayerNorm(width)
        self.action_head = nn.Linear(width, 18)
        self.value_head = nn.Linear(width, 1)
        self.action_embedding = nn.Embedding(18, width)
        self.source_query = nn.Linear(width, width)
        self.target_query = nn.Linear(width, width)
        self.stop_head = nn.Linear(width, 1)
        self.source_known = nn.Linear(1, 1, bias=False)
        self.history = nn.GRUCell(width, width)

    def prepare(self, observations):
        from simulation.encoder import Rows

        if not observations:
            raise ValueError("a policy batch must contain observations")
        adapted = [
            tensor_observation(observation, self.limits) for observation in observations
        ]
        lengths = [
            {name: len(rows.present) for name, rows in game.items()} for game in adapted
        ]
        maxima = {
            name: max(length[name] for length in lengths) for name in self.encoders
        }
        tokens = sum(maxima.values())
        if (
            tokens > self.limits.max_tokens
            or len(observations) * tokens * tokens
            > self.limits.max_batch_attention_cells
        ):
            raise UnsupportedObservation(
                "padded attention batch exceeds resource limits; reduce minibatch size"
            )
        features, positions, present, offsets = {}, {}, {}, {}
        for name in self.encoders:
            rows = Rows(
                *(
                    torch.cat([getattr(game[name], field) for game in adapted])
                    for field in ("cat", "num", "flags", "visible", "present")
                )
            )
            features[name] = self.encoders[name].prepare(rows)
            starts = [0]
            for length in lengths:
                starts.append(starts[-1] + length[name])
            offsets[name] = starts
            positions[name] = pad_sequence(
                [
                    torch.arange(length[name]) + start
                    for length, start in zip(lengths, starts)
                ],
                batch_first=True,
            )
            present[name] = pad_sequence(
                [game[name].present for game in adapted], batch_first=True
            )
        # Concatenate sparse tooltip rows, but attach to their true parent/slot.
        value_cats, value_nums, value_vis, value_present = [], [], [], []
        links = {name: ([], []) for name in ("jokers", "shop_items", "pack_options")}
        value_offset = 0
        for game_index, observation in enumerate(observations):
            cats = torch.from_numpy(observation["joker_values.cat"]).long()
            nums = torch.from_numpy(observation["joker_values.num"])
            vis = torch.from_numpy(observation["joker_values.visible"])
            exists = torch.from_numpy(observation["joker_values.present"])
            active = exists & vis[:, :3].all(-1)
            if (exists & ~active).any():
                raise UnsupportedObservation("Joker value lacks visible parent/key")
            if ((cats[active, 0] < 0) | (cats[active, 0] > 2)).any():
                raise UnsupportedObservation("invalid Joker parent collection")
            for parent, name in enumerate(links):
                indices = torch.nonzero(active & (cats[:, 0] == parent)).flatten()
                slots = cats[indices, 1]
                if ((slots < 0) | (slots >= lengths[game_index][name])).any():
                    raise UnsupportedObservation(
                        "Joker value references missing parent slot"
                    )
                links[name][0].extend((indices + value_offset).tolist())
                links[name][1].extend((slots + offsets[name][game_index]).tolist())
            value_cats.append(cats)
            value_nums.append(nums)
            value_vis.append(vis)
            value_present.append(exists)
            value_offset += len(cats)
        cats, vals, vis, exists = (
            torch.cat(values)
            for values in (value_cats, value_nums, value_vis, value_present)
        )
        keys = cats[:, 2]
        numerical = torch.where(vis[:, 3:4] & exists[:, None], vals, 0.0)
        categorical = exists & vis[:, 3] & ((keys == 8) | (keys == 9) | (keys == 10))
        category_values = numerical[:, 0][categorical]
        if (
            not torch.isfinite(category_values).all()
            or (category_values != category_values.trunc()).any()
            or ((category_values < 0) | (category_values > 8191)).any()
        ):
            raise UnsupportedObservation("invalid categorical Joker tooltip value")
        raw = torch.where(categorical, numerical[:, 0], 0).long()
        bits = ((raw[:, None] >> torch.arange(13)) & 1).bool() & (
            (keys == 10) & categorical
        )[:, None]
        value_rows = Rows(
            keys[:, None],
            torch.where(categorical[:, None], 0.0, numerical),
            bits,
            torch.cat(
                (
                    vis[:, 2:3],
                    vis[:, 3:4] & ~categorical[:, None],
                    (categorical & (keys == 10))[:, None].expand(-1, 13),
                ),
                -1,
            ),
            exists,
        )
        category_ids = []
        for key, maximum in ((8, 3), (9, 12)):
            selected = categorical & (keys == key)
            if ((raw[selected] < 0) | (raw[selected] > maximum)).any():
                raise UnsupportedObservation("invalid Joker tooltip category")
            category_ids.append(torch.where(selected, raw + 1, 0))
        return PreparedBatch(
            features,
            positions,
            present,
            lengths,
            self.joker_value_encoder.prepare(value_rows),
            *category_ids,
            {
                name: (
                    torch.tensor(rows, dtype=torch.long),
                    torch.tensor(slots, dtype=torch.long),
                )
                for name, (rows, slots) in links.items()
            },
        )

    def forward_batch(self, batch):
        encoded = {
            name: self.encoders[name](features)
            for name, features in batch.features.items()
        }
        values = self.joker_value_encoder(batch.joker_features)
        values = (
            values
            + self.joker_suit(batch.joker_suits)
            + self.joker_hand(batch.joker_hands)
        )
        for name, (rows, slots) in batch.links.items():
            encoded[name] = encoded[name].index_add(0, slots, values[rows])
        chunks, masks, spans = [], [], {}
        offset = 0
        for role, (name, spec) in enumerate((CONTEXT | TABLES).items()):
            indices = batch.positions[name]
            size = indices.shape[1]
            hidden = (
                encoded[name][indices]
                if size
                else encoded[name].reshape(len(batch.lengths), 0, self.config.width)
            )
            hidden = hidden + self.roles.weight[role]
            if spec.positioned:
                hidden = (
                    hidden
                    + self.positions(torch.arange(size, device=hidden.device))[None]
                )
            chunks.append(hidden)
            masks.append(~batch.present[name])
            spans[name] = slice(offset, offset + size)
            offset += size
        hidden, padding = torch.cat(chunks, 1), torch.cat(masks, 1)
        for layer in self.attention:
            hidden = layer(hidden, src_key_padding_mask=padding)
        hidden = self.norm(hidden)
        state = hidden[:, 0]
        return PolicyOutput(
            state,
            self.action_head(state),
            self.value_head(state).squeeze(-1),
            {name: hidden[:, span] for name, span in spans.items()},
            batch.lengths,
        )

    def forward(self, observations):
        return self.forward_batch(
            self.prepare(observations).to(next(self.parameters()).device)
        )

    def query(self, output, game, action):
        return torch.tanh(output.state[game] + self.action_embedding.weight[action])

    def source_logits(self, rows, query, known):
        return (rows * self.source_query(query).unsqueeze(-2)).sum(-1) / sqrt(
            self.config.width
        ) + self.source_known(known[..., None].float()).squeeze(-1)

    def selection_logits(self, rows, query):
        return torch.cat(
            (
                (rows * self.target_query(query).unsqueeze(-2)).sum(-1)
                / sqrt(self.config.width),
                self.stop_head(query),
            ),
            dim=-1,
        )
