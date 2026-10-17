//! Typed batch transport; all commands and constraints still use the single-game owners.
use crate::core::action::{ActionConstraints, PermutationCollection, TargetCollection};
use crate::core::batch::GameBatch;
use crate::core::enums::{Decks, Stakes};
use crate::core::game::ActionKind;
use crate::python_arrays::{self, Block};
use numpy::{Element, PyArray1, PyArray2, PyArrayMethods, PyUntypedArrayMethods};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::time::Instant;

fn error(message: &str) -> PyErr {
    PyValueError::new_err(message.to_owned())
}
fn vector<T: Element + Copy>(value: &Bound<'_, PyAny>) -> PyResult<Vec<T>> {
    let array = value.cast::<PyArray1<T>>()?;
    if !array.is_c_contiguous() || !array.is_aligned() {
        return Err(error("array must be aligned and C-contiguous"));
    }
    Ok(array
        .try_readonly()
        .map_err(|e| error(&e.to_string()))?
        .as_slice()
        .map_err(|e| error(&e.to_string()))?
        .to_vec())
}
fn matrix(
    value: &Bound<'_, PyAny>,
    rows: usize,
    minimum_columns: usize,
) -> PyResult<(Vec<i32>, usize)> {
    let array = value.cast::<PyArray2<i32>>()?;
    let shape = array.shape();
    if shape[0] != rows || shape[1] < minimum_columns {
        return Err(error("invalid batch matrix shape"));
    }
    if !array.is_c_contiguous() || !array.is_aligned() {
        return Err(error("array must be aligned and C-contiguous"));
    }
    let columns = shape[1];
    let data = array
        .try_readonly()
        .map_err(|e| error(&e.to_string()))?
        .as_slice()
        .map_err(|e| error(&e.to_string()))?
        .to_vec();
    Ok((data, columns))
}
fn kind(code: i32) -> PyResult<ActionKind> {
    usize::try_from(code)
        .ok()
        .and_then(|index| ActionKind::ALL.get(index))
        .copied()
        .ok_or_else(|| error("invalid action kind"))
}
fn optional_index(index: i32) -> PyResult<Option<usize>> {
    match index {
        -1 => Ok(None),
        0.. => Ok(Some(index as usize)),
        _ => Err(error("invalid optional index")),
    }
}
fn indices(values: &[i32]) -> PyResult<Vec<usize>> {
    values
        .iter()
        .map(|index| usize::try_from(*index).map_err(|_| error("negative index")))
        .collect()
}
fn empty_constraints() -> ActionConstraints {
    ActionConstraints {
        source_mask: vec![],
        source_known: vec![],
        target_collection: TargetCollection::None,
        target_mask: vec![],
        required_targets: vec![],
        count_mask: vec![false],
        ordered: false,
        permutation_collection: PermutationCollection::None,
        available: false,
        can_submit: false,
    }
}
const LENGTHS: [&str; 14] = [
    "jokers.present",
    "joker_values.present",
    "consumables.present",
    "last_consumable.present",
    "deck_catalogue.present",
    "poker_hands.present",
    "vouchers.present",
    "tags.present",
    "blind_offers.present",
    "hand.present",
    "shop_items.present",
    "shop_vouchers.present",
    "shop_packs.present",
    "pack_options.present",
];

#[pyclass(name = "GameBatch")]
pub(crate) struct PyGameBatch {
    batch: GameBatch,
}

impl PyGameBatch {
    fn observations<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let bundles = self
            .batch
            .games
            .iter()
            .map(|game| {
                game.observe()
                    .map(python_arrays::observation_blocks)
                    .map_err(python_arrays::observation_error)
            })
            .collect::<PyResult<Vec<_>>>()?;
        python_arrays::padded(py, bundles, &LENGTHS)
    }
}
#[pymethods]
impl PyGameBatch {
    fn __len__(&self) -> usize {
        self.batch.games.len()
    }
    fn observe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        self.observations(py)
    }

    fn reset(&mut self, indices: &Bound<'_, PyAny>, seeds: &Bound<'_, PyAny>) -> PyResult<()> {
        let slots = vector::<i32>(indices)?;
        let seeds = vector::<u64>(seeds)?;
        if slots.len() != seeds.len() {
            return Err(error("reset lengths differ"));
        }
        let mut seen = std::collections::HashSet::new();
        for index in &slots {
            if *index < 0 || *index as usize >= self.batch.games.len() || !seen.insert(*index) {
                return Err(error("invalid or duplicate reset slot"));
            }
        }
        for (index, seed) in slots.into_iter().zip(seeds) {
            self.batch.reset(index as usize, seed);
        }
        Ok(())
    }

    fn constraints<'py>(
        &self,
        py: Python<'py>,
        kinds: &Bound<'_, PyAny>,
        sources: &Bound<'_, PyAny>,
        prefixes: &Bound<'_, PyAny>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let kinds = vector::<i32>(kinds)?;
        let sources = vector::<i32>(sources)?;
        let n = self.batch.games.len();
        if kinds.len() != n || sources.len() != n {
            return Err(error("constraint lengths differ from batch"));
        }
        let (prefixes, columns) = matrix(prefixes, n, 1)?;
        let mut bundles = vec![];
        for index in 0..n {
            let row = &prefixes[index * columns..(index + 1) * columns];
            if row[0] < 0
                || row[0] as usize > columns - 1
                || row[1 + row[0] as usize..].iter().any(|value| *value != -1)
            {
                return Err(error("invalid prefix count or padding"));
            }
            let constraint = if kinds[index] == -1 {
                if sources[index] != -1 || row[0] != 0 {
                    return Err(error("idle query has fields"));
                }
                empty_constraints()
            } else {
                self.batch.games[index]
                    .selection_constraints(
                        kind(kinds[index])?,
                        optional_index(sources[index])?,
                        &indices(&row[1..1 + row[0] as usize])?,
                    )
                    .map_err(|e| error(&format!("slot {index}: {e:?}")))?
            };
            bundles.push(python_arrays::constraint_blocks(constraint));
        }
        python_arrays::padded(py, bundles, &["source_mask", "target_mask", "count_mask"])
    }

    #[pyo3(signature=(commands, observe=true, profile=false))]
    fn step<'py>(
        &mut self,
        py: Python<'py>,
        commands: &Bound<'_, PyAny>,
        observe: bool,
        profile: bool,
    ) -> PyResult<Bound<'py, PyDict>> {
        let started = Instant::now();
        let n = self.batch.games.len();
        let (commands, columns) = matrix(commands, n, 4)?;
        // Structural errors invalidate the call before any game is changed.
        for row in commands.chunks_exact(columns) {
            if row[0] < -1
                || row[1] < -1
                || row[2] < -1
                || row[3] < -1
                || (row[2] >= 0 && row[3] >= 0)
            {
                return Err(error("invalid command header"));
            }
            let count = row[2].max(row[3]).max(0) as usize;
            if count > columns - 4
                || row[4 + count..].iter().any(|value| *value != -1)
                || row[4..4 + count].iter().any(|value| *value < 0)
            {
                return Err(error("invalid command payload or padding"));
            }
            if row[0] == -1 && row[1..].iter().any(|value| *value != -1) {
                return Err(error("idle command has fields"));
            }
        }
        let parsed = commands
            .chunks_exact(columns)
            .enumerate()
            .map(|(index, row)| {
                if row[0] == -1 {
                    return Ok(None);
                }
                let targets = (row[2] >= 0)
                    .then(|| indices(&row[4..4 + row[2] as usize]))
                    .transpose()?;
                let order = (row[3] >= 0)
                    .then(|| indices(&row[4..4 + row[3] as usize]))
                    .transpose()?;
                crate::python_actions::command(
                    &self.batch.games[index],
                    kind(row[0])?,
                    optional_index(row[1])?,
                    targets,
                    order,
                )
                .map(Some)
            })
            .collect::<Vec<PyResult<_>>>();
        let decoded = started.elapsed().as_secs_f64();
        let engine_started = Instant::now();
        let mut accepted = vec![false; n];
        let attempted: Vec<bool> = commands
            .chunks_exact(columns)
            .map(|row| row[0] != -1)
            .collect();
        let mut errors: Vec<Option<String>> = vec![None; n];
        for (index, command) in parsed.into_iter().enumerate() {
            match command {
                Ok(Some(command)) => match self.batch.games[index].apply_action(command) {
                    Ok(_) => {
                        self.batch.games[index].next_step();
                        accepted[index] = true;
                    }
                    Err(e) => errors[index] = Some(format!("{e:?}")),
                },
                Ok(None) => {}
                Err(e) => errors[index] = Some(e.to_string()),
            }
        }
        let engine_seconds = engine_started.elapsed().as_secs_f64();
        let encoded_started = Instant::now();
        let observations = if observe {
            Some(
                self.batch
                    .games
                    .iter()
                    .map(|game| {
                        game.observe()
                            .map(python_arrays::observation_blocks)
                            .map_err(python_arrays::observation_error)
                    })
                    .collect::<PyResult<Vec<_>>>()?,
            )
        } else {
            None
        };
        let encode_seconds = encoded_started.elapsed().as_secs_f64();
        let export_started = Instant::now();
        let result = python_arrays::export(
            py,
            vec![
                ("accepted".to_owned(), Block::Bool(vec![n], accepted)),
                ("attempted".to_owned(), Block::Bool(vec![n], attempted)),
                (
                    "phases".to_owned(),
                    Block::I32(
                        vec![n],
                        self.batch
                            .games
                            .iter()
                            .map(|game| i32::from(game.decision_point().code()))
                            .collect(),
                    ),
                ),
            ],
        )?;
        // Export every slot, including idle/rejected commands and completed blinds.
        result.set_item(
            "blind_scores",
            PyArray1::from_vec(
                py,
                self.batch
                    .games
                    .iter()
                    .map(|game| game.state().blind.chips_scored)
                    .collect(),
            ),
        )?;
        result.set_item("errors", errors)?;
        result.set_item(
            "observations",
            observations
                .map(|bundles| python_arrays::padded(py, bundles, &LENGTHS))
                .transpose()?,
        )?;
        if profile {
            let timings = PyDict::new(py);
            timings.set_item("command_decode_seconds", decoded)?;
            timings.set_item("engine_step_seconds", engine_seconds)?;
            timings.set_item("observation_encode_seconds", encode_seconds)?;
            timings.set_item(
                "array_export_seconds",
                export_started.elapsed().as_secs_f64(),
            )?;
            result.set_item("timings", timings)?;
        }
        Ok(result)
    }
}

#[pyfunction]
#[pyo3(signature=(seeds,decks=None,stakes=None))]
pub(crate) fn new_batch(
    seeds: &Bound<'_, PyAny>,
    decks: Option<Vec<Decks>>,
    stakes: Option<Vec<Stakes>>,
) -> PyResult<PyGameBatch> {
    let seeds = vector::<u64>(seeds)?;
    if seeds.is_empty() || seeds.len() > i32::MAX as usize {
        return Err(error("batch must contain 1..int32::MAX games"));
    }
    let decks = decks.unwrap_or_else(|| vec![Decks::Red; seeds.len()]);
    let stakes = stakes.unwrap_or_else(|| vec![Stakes::White; seeds.len()]);
    if decks.len() != seeds.len() || stakes.len() != seeds.len() {
        return Err(error("rule lengths differ from seeds"));
    }
    Ok(PyGameBatch {
        batch: GameBatch::new(&seeds, decks.into_iter().zip(stakes).collect()),
    })
}
