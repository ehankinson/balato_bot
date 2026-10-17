"""Policy search restrictions, separate from Rust's legal-action contract."""

REORDER_COLLECTIONS = {15: "jokers", 16: "hand"}


def action_kind_mask(observation, reordered=()):
    mask = observation["action_kinds"].copy()
    for kind, collection in REORDER_COLLECTIONS.items():
        mask[kind] &= (
            len(observation[f"{collection}.present"]) > 1 and kind not in reordered
        )
    return mask


def policy_observation(observation, reordered):
    # Retained snapshots and the original Rust arrays must remain unchanged.
    return {**observation, "action_kinds": action_kind_mask(observation, reordered)}


def exclude_identity_completion(mask, prefix, length):
    """Restrict a permutation mask in place without resampling or rewriting actions."""
    # With two slots left, following the identity prefix would force the final
    # slot too. Force a deviation here while there is still a valid completion.
    if (
        length >= 2
        and len(prefix) == length - 2
        and all(slot == index for index, slot in enumerate(prefix))
    ):
        mask[length - 2] = False
