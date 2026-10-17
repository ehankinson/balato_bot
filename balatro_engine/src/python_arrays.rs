//! NumPy transport only. Game rules and visibility stay in core.
use crate::core::action::ActionConstraints;
use crate::core::observation::{Observation, ObservationError, Table};
use numpy::{Element, PyArray1, PyArrayMethods, PyUntypedArrayMethods};
use pyo3::exceptions::{PyMemoryError, PyNotImplementedError, PyOverflowError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;

pub(crate) enum Block {
    I32(Vec<usize>, Vec<i32>),
    I64(Vec<usize>, Vec<i64>),
    F32(Vec<usize>, Vec<f32>),
    Bool(Vec<usize>, Vec<bool>),
}
trait Numeric: Element + Clone + Default {
    fn block(shape: Vec<usize>, data: Vec<Self>) -> Block;
}
macro_rules! numeric {
    ($ty:ty, $variant:ident) => {
        impl Numeric for $ty {
            fn block(shape: Vec<usize>, data: Vec<Self>) -> Block {
                Block::$variant(shape, data)
            }
        }
    };
}
numeric!(i32, I32);
numeric!(i64, I64);
numeric!(f32, F32);
numeric!(bool, Bool);
type Bundle = Vec<(String, Block)>;
fn vector<T: Numeric>(bundle: &mut Bundle, key: &str, data: Vec<T>) {
    bundle.push((key.to_owned(), T::block(vec![data.len()], data)));
}
fn matrix<T: Numeric>(bundle: &mut Bundle, key: &str, data: Vec<T>, rows: usize, columns: usize) {
    bundle.push((key.to_owned(), T::block(vec![rows, columns], data)));
}
fn table<const C: usize, const N: usize, const F: usize>(
    bundle: &mut Bundle,
    name: &str,
    data: Table<C, N, F>,
) {
    let rows = data.len();
    matrix(
        bundle,
        &format!("{name}.cat"),
        data.cat.into_flattened(),
        rows,
        C,
    );
    matrix(
        bundle,
        &format!("{name}.num"),
        data.num.into_flattened(),
        rows,
        N,
    );
    matrix(
        bundle,
        &format!("{name}.flags"),
        data.flags.into_flattened(),
        rows,
        F,
    );
    vector(bundle, &format!("{name}.present"), data.present);
    matrix(
        bundle,
        &format!("{name}.visible"),
        data.visible,
        rows,
        C + N + F,
    );
}
fn export_array<T: Element>(
    dict: &Bound<'_, PyDict>,
    name: &str,
    shape: Vec<usize>,
    data: Vec<T>,
) -> PyResult<()> {
    dict.set_item(
        name,
        PyArray1::from_vec(dict.py(), data).reshape(numpy::ndarray::IxDyn(&shape))?,
    )
}
pub(crate) fn export(py: Python<'_>, bundle: Bundle) -> PyResult<Bound<'_, PyDict>> {
    let dict = PyDict::new(py);
    for (name, block) in bundle {
        match block {
            Block::I32(s, d) => export_array(&dict, &name, s, d)?,
            Block::I64(s, d) => export_array(&dict, &name, s, d)?,
            Block::F32(s, d) => export_array(&dict, &name, s, d)?,
            Block::Bool(s, d) => export_array(&dict, &name, s, d)?,
        }
    }
    Ok(dict)
}
impl Block {
    fn shape(&self) -> &[usize] {
        match self {
            Self::I32(s, _) | Self::I64(s, _) | Self::F32(s, _) | Self::Bool(s, _) => s,
        }
    }
}
fn stack<T: Numeric>(shape: &[usize], parts: Vec<Vec<T>>, lengths: &[usize]) -> PyResult<Block> {
    let rows = lengths.iter().copied().max().unwrap_or(0);
    let width = shape[1..]
        .iter()
        .try_fold(1usize, |a, b| a.checked_mul(*b))
        .ok_or_else(|| PyOverflowError::new_err("batch shape overflow"))?;
    let size = parts
        .len()
        .checked_mul(rows)
        .and_then(|x| x.checked_mul(width))
        .ok_or_else(|| PyOverflowError::new_err("batch shape overflow"))?;
    let mut data = Vec::new();
    data.try_reserve_exact(size)
        .map_err(|_| PyMemoryError::new_err("batch allocation failed"))?;
    data.resize(size, T::default());
    for (index, part) in parts.into_iter().enumerate() {
        let start = index * rows * width;
        data[start..start + part.len()].clone_from_slice(&part);
    }
    let mut output_shape = vec![lengths.len(), rows];
    output_shape.extend_from_slice(&shape[1..]);
    Ok(T::block(output_shape, data))
}
pub(crate) fn padded<'py>(
    py: Python<'py>,
    bundles: Vec<Bundle>,
    length_fields: &[&str],
) -> PyResult<Bound<'py, PyDict>> {
    let mut columns: Vec<Vec<(String, Block)>> = (0..bundles[0].len()).map(|_| vec![]).collect();
    for bundle in bundles {
        for (column, value) in columns.iter_mut().zip(bundle) {
            column.push(value);
        }
    }
    let mut output = vec![];
    for column in columns {
        let name = column[0].0.clone();
        let shape = column[0].1.shape().to_vec();
        let lengths: Vec<usize> = column.iter().map(|(_, block)| block.shape()[0]).collect();
        if length_fields.contains(&name.as_str()) {
            vector(
                &mut output,
                &format!("lengths.{}", name.trim_end_matches(".present")),
                lengths.iter().map(|n| i64::try_from(*n).unwrap()).collect(),
            );
        }
        macro_rules! combine {
            ($variant:ident) => {{
                let parts = column
                    .into_iter()
                    .map(|(_, block)| match block {
                        Block::$variant(_, data) => data,
                        _ => unreachable!("schema dtype"),
                    })
                    .collect();
                stack(&shape, parts, &lengths)?
            }};
        }
        let block = match &column[0].1 {
            Block::I32(_, _) => combine!(I32),
            Block::I64(_, _) => combine!(I64),
            Block::F32(_, _) => combine!(F32),
            Block::Bool(_, _) => combine!(Bool),
        };
        output.push((name, block));
    }
    export(py, output)
}
pub(crate) fn observation(py: Python<'_>, data: Observation) -> PyResult<Bound<'_, PyDict>> {
    export(py, observation_blocks(data))
}
pub(crate) fn observation_blocks(data: Observation) -> Bundle {
    let mut bundle = vec![];
    vector(&mut bundle, "schema_version", data.schema_version.to_vec());
    vector(&mut bundle, "sections", data.sections.to_vec());
    vector(&mut bundle, "run.cat", data.run.cat.to_vec());
    vector(&mut bundle, "run.counts", data.run.counts.to_vec());
    vector(&mut bundle, "run.num", data.run.num.to_vec());
    vector(&mut bundle, "run.extra", data.run.extra.to_vec());
    vector(
        &mut bundle,
        "run.used_planets",
        data.run.used_planets.to_vec(),
    );
    vector(&mut bundle, "run.flags", data.run.flags.to_vec());
    vector(&mut bundle, "run.visible", data.run.visible.to_vec());
    vector(&mut bundle, "blind.cat", data.blind.cat.to_vec());
    vector(&mut bundle, "blind.counts", data.blind.counts.to_vec());
    vector(&mut bundle, "blind.num", data.blind.num.to_vec());
    vector(&mut bundle, "blind.flags", data.blind.flags.to_vec());
    vector(&mut bundle, "blind.visible", data.blind.visible.to_vec());
    vector(
        &mut bundle,
        "blind.played_types",
        data.blind.played_types.to_vec(),
    );
    vector(&mut bundle, "shop.num", data.shop.num.to_vec());
    vector(&mut bundle, "shop.visible", data.shop.visible.to_vec());
    vector(&mut bundle, "pack.cat", data.pack.cat.to_vec());
    vector(&mut bundle, "pack.counts", data.pack.counts.to_vec());
    vector(&mut bundle, "pack.visible", data.pack.visible.to_vec());
    table(&mut bundle, "jokers", data.jokers);
    table(&mut bundle, "joker_values", data.joker_values);
    table(&mut bundle, "consumables", data.consumables);
    table(&mut bundle, "last_consumable", data.last_consumable);
    table(&mut bundle, "deck_catalogue", data.deck_catalogue);
    table(&mut bundle, "poker_hands", data.poker_hands);
    table(&mut bundle, "vouchers", data.vouchers);
    table(&mut bundle, "tags", data.tags);
    table(&mut bundle, "blind_offers", data.blind_offers);
    table(&mut bundle, "hand", data.hand);
    table(&mut bundle, "shop_items", data.shop_items);
    table(&mut bundle, "shop_vouchers", data.shop_vouchers);
    table(&mut bundle, "shop_packs", data.shop_packs);
    table(&mut bundle, "pack_options", data.pack_options);
    vector(&mut bundle, "deck_multiplicity", data.deck_multiplicity);
    matrix(
        &mut bundle,
        "poker_hand_counts",
        data.poker_hand_counts.into_flattened(),
        12,
        2,
    );
    vector(&mut bundle, "action_kinds", data.action_kinds.to_vec());
    bundle
}
pub(crate) fn constraints(py: Python<'_>, data: ActionConstraints) -> PyResult<Bound<'_, PyDict>> {
    export(py, constraint_blocks(data))
}
pub(crate) fn constraint_blocks(data: ActionConstraints) -> Bundle {
    let mut bundle = vec![];
    vector(&mut bundle, "source_mask", data.source_mask);
    vector(&mut bundle, "source_known", data.source_known);
    vector(&mut bundle, "target_mask", data.target_mask);
    vector(&mut bundle, "required_targets", data.required_targets);
    vector(&mut bundle, "count_mask", data.count_mask);
    vector(
        &mut bundle,
        "target_collection",
        vec![data.target_collection as i32],
    );
    vector(
        &mut bundle,
        "permutation_collection",
        vec![data.permutation_collection as i32],
    );
    vector(&mut bundle, "ordered", vec![data.ordered]);
    vector(&mut bundle, "can_submit", vec![data.can_submit]);
    bundle
}

pub(crate) fn observation_error(error: ObservationError) -> PyErr {
    match error {
        ObservationError::AllocationFailed => {
            PyMemoryError::new_err("observation allocation failed")
        }
        ObservationError::CollectionTooLarge | ObservationError::IntegerOverflow => {
            PyOverflowError::new_err(format!("observation: {error:?}"))
        }
        ObservationError::UnsupportedPhase => {
            PyNotImplementedError::new_err("unsupported observation phase")
        }
        _ => PyValueError::new_err(format!("observation: {error:?}")),
    }
}

/// No coercion or copy of a strided view: the selected-command boundary is
/// exactly a native-endian, aligned, C-contiguous one-dimensional int32 array.
pub(crate) fn selected_indices(value: &Bound<'_, PyAny>) -> PyResult<Vec<usize>> {
    let array = value.cast::<PyArray1<i32>>()?;
    if !array.is_c_contiguous() || !array.is_aligned() {
        return Err(PyValueError::new_err(
            "indices must be aligned and C-contiguous",
        ));
    }
    let values = array
        .try_readonly()
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    values
        .as_slice()
        .map_err(|error| PyValueError::new_err(error.to_string()))?
        .iter()
        .map(|index| {
            usize::try_from(*index)
                .map_err(|_| PyValueError::new_err("indices must be nonnegative"))
        })
        .collect()
}
