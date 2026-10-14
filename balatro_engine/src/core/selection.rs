/// A selection names distinct, existing occurrences, even when objects are identical.
pub(crate) fn valid_indices(indices: &[usize], length: usize) -> bool {
    indices
        .iter()
        .enumerate()
        .all(|(position, index)| *index < length && !indices[..position].contains(index))
}

/// Each new slot receives the object at the specified old index.
pub(crate) fn valid_permutation(order: &[usize], length: usize) -> bool {
    order.len() == length && valid_indices(order, length)
}
