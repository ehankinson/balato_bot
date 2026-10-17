"""Conditional PyTorch distributions, with Rust masks saved for exact replay."""

from dataclasses import dataclass

import numpy as np
import torch
from torch.distributions import Categorical

import balatro_engine as engine

ACTION_NAMES = (
    "SelectBlind",
    "SkipBlind",
    "PlayCards",
    "DiscardCards",
    "BuyShopItem",
    "BuyVoucher",
    "RerollShop",
    "BuyPack",
    "SelectPackOption",
    "UseConsumable",
    "SellJoker",
    "SellConsumable",
    "LeaveShop",
    "SkipPack",
    "UsePackConsumable",
    "ReorderJokers",
    "ReorderHand",
    "RerollBoss",
)
ACTIONS = tuple(getattr(engine.Action, name) for name in ACTION_NAMES)
SOURCE_COLLECTIONS = {
    0: "blind_offers",
    1: "blind_offers",
    4: "shop_items",
    5: "shop_vouchers",
    7: "shop_packs",
    8: "pack_options",
    9: "consumables",
    10: "jokers",
    11: "consumables",
    14: "pack_options",
}


@dataclass(frozen=True)
class Command:
    kind: int
    source: int | None = None
    targets: tuple[int, ...] | None = None
    order: tuple[int, ...] | None = None

    def __post_init__(self):
        if type(self.kind) is not int:
            raise TypeError("command kind must be a Python integer")
        if not 0 <= self.kind < len(ACTIONS):
            raise ValueError("unknown command kind")
        if self.targets is not None and self.order is not None:
            raise ValueError("command cannot contain both targets and order")
        fields = [] if self.source is None else [self.source]
        for indices in (self.targets, self.order):
            if indices is not None:
                if not isinstance(indices, tuple):
                    raise TypeError(
                        "command selections must be immutable integer tuples"
                    )
                fields.extend(indices)
        if any(type(index) is not int for index in fields):
            raise TypeError("command indices must be Python integers")
        if any(not 0 <= index <= 2**31 - 1 for index in fields):
            raise ValueError("command indices must fit nonnegative int32")

    def apply(self, game):
        fields = {}
        if self.source is not None:
            fields["source"] = self.source
        if self.targets is not None:
            fields["targets"] = np.array(self.targets, dtype=np.int32)
        if self.order is not None:
            fields["order"] = np.array(self.order, dtype=np.int32)
        return game.apply_action(ACTIONS[self.kind], **fields)


@dataclass(frozen=True)
class Choice:
    stage: str
    collection: str
    selected: int
    mask: np.ndarray
    prefix: tuple[int, ...] = ()
    source_known: np.ndarray | None = None


@dataclass(frozen=True)
class ActionTrace:
    command: Command
    choices: tuple[Choice, ...]
    source_known: bool


@dataclass
class DecodedAction:
    trace: ActionTrace
    log_prob: torch.Tensor
    entropy: torch.Tensor


def _snapshot(array):
    array = np.array(array, dtype=np.bool_, copy=True)
    array.setflags(write=False)
    return array


def _distribution(logits, mask):
    mask = torch.tensor(mask, dtype=torch.bool, device=logits.device)
    if mask.shape != logits.shape or not mask.any():
        raise ValueError("empty or mismatched conditional action distribution")
    return Categorical(logits=logits.masked_fill(~mask, float("-inf")))


def decode_action(model, output, game, observation, stochastic=True, game_index=0):
    choices, log_probs, entropies = [], [], []

    def choose(stage, collection, logits, mask, prefix=(), known=None):
        distribution = _distribution(logits, mask)
        selected = int(
            distribution.sample() if stochastic else distribution.logits.argmax()
        )
        token = torch.tensor(selected, device=logits.device)
        log_probs.append(distribution.log_prob(token))
        entropies.append(distribution.entropy())
        choices.append(
            Choice(
                stage,
                collection,
                selected,
                _snapshot(mask),
                tuple(prefix),
                _snapshot(known) if known is not None else None,
            )
        )
        return selected

    kind = choose(
        "kind", "", output.action_logits[game_index], observation["action_kinds"]
    )
    query = model.query(output, game_index, kind)
    action = ACTIONS[kind]
    constraint = game.action_constraints(action)
    source, source_known = None, True
    if kind in SOURCE_COLLECTIONS:
        collection = SOURCE_COLLECTIONS[kind]
        rows = output.objects(game_index, collection)
        known = torch.tensor(constraint["source_known"], device=query.device)
        source = choose(
            "source",
            collection,
            model.source_logits(rows, query, known),
            constraint["source_mask"],
            known=constraint["source_known"],
        )
        source_known = bool(constraint["source_known"][source])
        query = model.history(rows[source], query)
        constraint = game.action_constraints(action, source=source)
    permutation = int(constraint["permutation_collection"][0])
    targets = int(constraint["target_collection"][0])
    if permutation or targets:
        collection = "jokers" if permutation == 2 else "hand"
        rows = output.objects(game_index, collection)
        prefix = []
        # Rust permits at most one occurrence of each index. STOP is allowed only
        # when its conditional can_submit says the completed command is valid.
        for _ in range(len(rows) + 1):
            constraint = game.selection_constraints(
                action, np.array(prefix, dtype=np.int32), source=source
            )
            mask = np.concatenate((constraint["target_mask"], constraint["can_submit"]))
            selected = choose(
                "selection",
                collection,
                model.selection_logits(rows, query),
                mask,
                prefix,
            )
            if selected == len(rows):
                break
            prefix.append(selected)
            query = model.history(rows[selected], query)
        else:
            raise RuntimeError("Rust constraints did not allow a completed selection")
        if permutation:
            command = Command(kind, source, order=tuple(prefix))
        else:
            # Keep the sampled sequence in the trace. The engine receives set
            # selections in current slot order; directed consumables keep order.
            selected = prefix if constraint["ordered"][0] else sorted(prefix)
            command = Command(kind, source, targets=tuple(selected))
    else:
        if not constraint["can_submit"][0]:
            raise RuntimeError("feasible action has no submittable command")
        command = Command(kind, source)
    return DecodedAction(
        ActionTrace(command, tuple(choices), source_known),
        torch.stack(log_probs).sum(),
        torch.stack(entropies).sum(),
    )


def evaluate_actions(model, output, traces):
    """Teacher-force the actual latent selection sequence with its saved masks."""
    logs, entropies = [], []
    for game_index, trace in enumerate(traces):
        query = model.query(output, game_index, trace.command.kind)
        log_parts, entropy_parts = [], []
        for choice in trace.choices:
            if choice.stage == "kind":
                logits = output.action_logits[game_index]
            else:
                rows = output.objects(game_index, choice.collection)
                if choice.stage == "source":
                    known = torch.tensor(choice.source_known, device=query.device)
                    logits = model.source_logits(rows, query, known)
                else:
                    logits = model.selection_logits(rows, query)
            distribution = _distribution(logits, choice.mask)
            token = torch.tensor(choice.selected, device=query.device)
            log_parts.append(distribution.log_prob(token))
            entropy_parts.append(distribution.entropy())
            if choice.stage != "kind" and choice.selected < len(rows):
                query = model.history(rows[choice.selected], query)
        logs.append(torch.stack(log_parts).sum())
        entropies.append(torch.stack(entropy_parts).sum())
    return torch.stack(logs), torch.stack(entropies)


@dataclass
class BatchDecoded:
    commands: np.ndarray
    traces: tuple[ActionTrace | None, ...]
    log_probs: torch.Tensor
    entropies: torch.Tensor


def pack_commands(commands):
    width = max(
        (
            len(command.targets or command.order or ())
            for command in commands
            if command is not None
        ),
        default=0,
    )
    packed = np.full((len(commands), 4 + width), -1, dtype=np.int32)
    for index, command in enumerate(commands):
        if command is None:
            continue
        packed[index, :2] = [
            command.kind,
            command.source if command.source is not None else -1,
        ]
        selection = command.targets if command.targets is not None else command.order
        if selection is not None:
            packed[index, 2 if command.targets is not None else 3] = len(selection)
            packed[index, 4 : 4 + len(selection)] = selection
    return packed


def _collection_rows(output, collections, device):
    """Gather different action collections into one pointer batch without scalar GPU reads."""
    width = max((output.rows[name].shape[1] for name in collections if name), default=0)
    result = output.state.new_zeros((len(collections), width, output.state.shape[-1]))
    lengths = np.zeros(len(collections), dtype=np.int32)
    for name in sorted(set(collections) - {""}):
        slots = [
            index for index, collection in enumerate(collections) if collection == name
        ]
        indices = torch.tensor(slots, device=device)
        rows = output.rows[name][indices]
        result[indices, : rows.shape[1]] = rows
        for slot in slots:
            lengths[slot] = output.lengths[slot][name]
    return result, lengths


def decode_action_batch(model, output, games, observations, active, stochastic=True):
    """One vector transfer per conditional stage; never one GPU .item() per game."""
    active = np.asarray(active, dtype=np.bool_)
    n = len(active)
    device = output.state.device
    choices = [[] for _ in range(n)]
    logs = output.value.new_zeros(n)
    entropy = output.value.new_zeros(n)
    batch_indices = torch.arange(n, device=device)

    def sample(logits, mask, used):
        nonlocal logs, entropy
        mask_cpu = np.asarray(mask, dtype=np.bool_)
        if not mask_cpu.any(-1).all() or tuple(mask_cpu.shape) != tuple(logits.shape):
            raise ValueError("empty or mismatched batch distribution")
        distribution = Categorical(
            logits=logits.masked_fill(
                ~torch.tensor(mask_cpu, device=device), float("-inf")
            )
        )
        selected = (
            distribution.sample() if stochastic else distribution.logits.argmax(-1)
        )
        used_tensor = torch.tensor(used, device=device)
        logs = logs + torch.where(used_tensor, distribution.log_prob(selected), 0)
        entropy = entropy + torch.where(used_tensor, distribution.entropy(), 0)
        # Rust needs these chosen values to supply the next conditional masks.
        return selected, selected.detach().cpu().numpy()

    kind_mask = np.stack(
        [observation["action_kinds"] for observation in observations]
    ).copy()
    kind_mask[~active] = False
    kind_mask[~active, 0] = True
    sampled_kind, kinds = sample(output.action_logits, kind_mask, active)
    kinds = kinds.astype(np.int32, copy=True)
    kinds[~active] = -1
    query = model.query(output, batch_indices, sampled_kind)
    for index in np.flatnonzero(active):
        choices[index].append(
            Choice("kind", "", int(kinds[index]), _snapshot(kind_mask[index]))
        )
    sources = np.full(n, -1, dtype=np.int32)
    prefixes = np.zeros((n, 1), dtype=np.int32)
    constraints = games.constraints(kinds, sources, prefixes)
    collections = [SOURCE_COLLECTIONS.get(int(kind), "") for kind in kinds]
    has_source = np.array([bool(name) for name in collections]) & active
    source_known = np.ones(n, dtype=np.bool_)
    if has_source.any():
        rows, source_lengths = _collection_rows(output, collections, device)
        width = rows.shape[1]
        known = np.zeros((n, width), dtype=np.bool_)
        source_mask = np.zeros((n, width + 1), dtype=np.bool_)
        mask_width = constraints["source_mask"].shape[1]
        known[:, :mask_width] = constraints["source_known"]
        source_mask[:, :mask_width] = constraints["source_mask"]
        source_mask[~has_source] = False
        source_mask[~has_source, -1] = True
        logits = torch.cat(
            (
                model.source_logits(rows, query, torch.tensor(known, device=device)),
                query.new_zeros((n, 1)),
            ),
            -1,
        )
        sampled_source, selected = sample(logits, source_mask, has_source)
        sources[has_source] = selected[has_source]
        for index in np.flatnonzero(has_source):
            length = int(source_lengths[index])
            source_known[index] = known[index, sources[index]]
            choices[index].append(
                Choice(
                    "source",
                    collections[index],
                    int(sources[index]),
                    _snapshot(source_mask[index, :length]),
                    source_known=_snapshot(known[index, :length]),
                )
            )
        selected_rows = rows[batch_indices, sampled_source.clamp(max=width - 1)]
        query = torch.where(
            torch.tensor(has_source, device=device)[:, None],
            model.history(selected_rows, query),
            query,
        )
        constraints = games.constraints(kinds, sources, prefixes)
    permutation = constraints["permutation_collection"][:, 0]
    targeted = active & (
        (permutation != 0) | (constraints["target_collection"][:, 0] != 0)
    )
    ordered = constraints["ordered"][:, 0].copy()
    target_collections = [
        "jokers" if permutation[index] == 2 else "hand" if targeted[index] else ""
        for index in range(n)
    ]
    sequences = [[] for _ in range(n)]
    if targeted.any():
        rows, lengths = _collection_rows(output, target_collections, device)
        width = rows.shape[1]
        pending = targeted.copy()
        # Empty permutations select STOP directly. All other cases have distinct
        # slots, so a finite width+1 bound follows from Rust's prefix constraints.
        for _ in range(width + 1):
            if not pending.any():
                break
            query_kinds = kinds.copy()
            query_sources = sources.copy()
            query_kinds[~pending] = -1
            query_sources[~pending] = -1
            max_prefix = max(
                (len(sequences[index]) for index in np.flatnonzero(pending)), default=0
            )
            prefixes = np.full((n, max_prefix + 1), -1, dtype=np.int32)
            prefixes[:, 0] = 0
            for index in np.flatnonzero(pending):
                prefixes[index, 0] = len(sequences[index])
                prefixes[index, 1 : 1 + len(sequences[index])] = sequences[index]
            conditional = games.constraints(query_kinds, query_sources, prefixes)
            mask = np.zeros((n, width + 1), dtype=np.bool_)
            mask[:, : conditional["target_mask"].shape[1]] = conditional["target_mask"]
            mask[:, -1] = conditional["can_submit"][:, 0]
            mask[~pending, -1] = True
            sampled, selected = sample(
                model.selection_logits(rows, query), mask, pending
            )
            extending = pending & (selected != width)
            for index in np.flatnonzero(pending):
                length = int(lengths[index])
                local_mask = np.concatenate((mask[index, :length], mask[index, -1:]))
                local_choice = (
                    length if selected[index] == width else int(selected[index])
                )
                choices[index].append(
                    Choice(
                        "selection",
                        target_collections[index],
                        local_choice,
                        _snapshot(local_mask),
                        tuple(sequences[index]),
                    )
                )
                if extending[index]:
                    sequences[index].append(int(selected[index]))
            if width:
                selected_rows = rows[batch_indices, sampled.clamp(max=width - 1)]
                query = torch.where(
                    torch.tensor(extending, device=device)[:, None],
                    model.history(selected_rows, query),
                    query,
                )
            pending = extending
        if pending.any():
            raise RuntimeError("Rust did not permit a completed batch selection")
    commands, traces = [], []
    for index in range(n):
        if not active[index]:
            commands.append(None)
            traces.append(None)
            continue
        source = int(sources[index]) if has_source[index] else None
        if targeted[index]:
            selection = tuple(
                sequences[index] if ordered[index] else sorted(sequences[index])
            )
            command = (
                Command(int(kinds[index]), source, order=selection)
                if permutation[index]
                else Command(int(kinds[index]), source, targets=selection)
            )
        else:
            if not constraints["can_submit"][index, 0]:
                raise RuntimeError("feasible batch action has no submittable command")
            command = Command(int(kinds[index]), source)
        commands.append(command)
        traces.append(
            ActionTrace(command, tuple(choices[index]), bool(source_known[index]))
        )
    return BatchDecoded(pack_commands(commands), tuple(traces), logs, entropy)
