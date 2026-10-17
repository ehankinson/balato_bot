mod calculation;
mod consts;
mod core;
mod python_actions;
mod python_arrays;
mod python_batch;

use pyo3::prelude::*;

use crate::core::blind::BlindKind;
use crate::core::game::ActionKind;
use crate::python_arrays::selected_indices;

#[pyclass(name = "Game")]
struct PyGame {
    game: crate::core::game::Game,
}

#[pymethods]
impl PyGame {
    /// Read the current boundary without advancing work or consuming RNG.
    fn observe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, pyo3::types::PyDict>> {
        python_arrays::observation(
            py,
            self.game
                .observe()
                .map_err(python_arrays::observation_error)?,
        )
    }

    #[pyo3(signature = (action, source=None))]
    fn action_constraints<'py>(
        &self,
        py: Python<'py>,
        action: ActionKind,
        source: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Bound<'py, pyo3::types::PyDict>> {
        let source = source.map(selected_index).transpose()?;
        let constraints = self
            .game
            .action_constraints(action, source)
            .map_err(|error| pyo3::exceptions::PyValueError::new_err(format!("{error:?}")))?;
        python_arrays::constraints(py, constraints)
    }

    #[pyo3(signature = (action, prefix, source=None))]
    fn selection_constraints<'py>(
        &self,
        py: Python<'py>,
        action: ActionKind,
        prefix: &Bound<'_, PyAny>,
        source: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Bound<'py, pyo3::types::PyDict>> {
        let source = source.map(selected_index).transpose()?;
        let prefix = selected_indices(prefix)?;
        let constraints = self
            .game
            .selection_constraints(action, source, &prefix)
            .map_err(|error| pyo3::exceptions::PyValueError::new_err(format!("{error:?}")))?;
        python_arrays::constraints(py, constraints)
    }

    fn next_step(&mut self) -> u8 {
        self.game.next_step().code()
    }

    #[getter]
    fn ante(&self) -> u8 {
        self.game.state().counters.ante
    }

    /// Exact chips scored in the current or most recently played blind.
    #[getter]
    fn blind_score(&self) -> u64 {
        self.game.state().blind.chips_scored
    }

    #[getter]
    fn blind_index(&self) -> Option<u8> {
        (self.game.decision_point() == crate::core::game::DecisionPoint::BlindSelection).then(
            || match self.game.available_blind() {
                BlindKind::Small => 0,
                BlindKind::Big => 1,
                BlindKind::Boss(_) => 2,
            },
        )
    }

    #[getter]
    fn decision_point(&self) -> u8 {
        self.game.decision_point().code()
    }

    #[pyo3(signature = (action, source=None, targets=None, order=None))]
    fn apply_action(
        &mut self,
        action: ActionKind,
        source: Option<&Bound<'_, PyAny>>,
        targets: Option<&Bound<'_, PyAny>>,
        order: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<u8> {
        let source = source.map(selected_index).transpose()?;
        let targets = targets.map(selected_indices).transpose()?;
        let order = order.map(selected_indices).transpose()?;
        let command = python_actions::command(&self.game, action, source, targets, order)?;
        self.game
            .apply_action(command)
            .map_err(|error| pyo3::exceptions::PyValueError::new_err(format!("{error:?}")))?;
        Ok(self.game.next_step().code())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blind_score_preserves_the_full_u64_range() {
        let mut game = new(0, core::enums::Decks::Red, core::enums::Stakes::White);
        for score in [0, 1, (1 << 24) + 1, (1 << 53) + 1, (1 << 63) + 1, u64::MAX] {
            game.game.state.blind.chips_scored = score;
            assert_eq!(game.blind_score(), score);
            assert_eq!(game.blind_score(), score);
            assert_eq!(game.game.state.blind.chips_scored, score);
        }
    }
}

// Python scalar indices use the schema's int32 range, without bool/float coercion.
fn selected_index(value: &Bound<'_, PyAny>) -> PyResult<usize> {
    if !value.is_instance_of::<pyo3::types::PyInt>()
        || value.is_instance_of::<pyo3::types::PyBool>()
    {
        return Err(pyo3::exceptions::PyTypeError::new_err(
            "indices must be integers, not bools or floats",
        ));
    }
    let index = value.extract::<i32>()?;
    usize::try_from(index)
        .map_err(|_| pyo3::exceptions::PyValueError::new_err("indices must be nonnegative"))
}

#[pyfunction]
#[pyo3(signature = (seed, deck=crate::core::enums::Decks::Red, stake=crate::core::enums::Stakes::White))]
fn new(seed: u64, deck: crate::core::enums::Decks, stake: crate::core::enums::Stakes) -> PyGame {
    PyGame {
        game: crate::core::game::Game::with_rules(seed, deck, stake),
    }
}

/// A Python module implemented in Rust.
#[pymodule]
mod balatro_engine {
    use pyo3::prelude::*;

    #[pymodule_export]
    use super::PyGame;

    #[pymodule_export]
    use crate::python_batch::{PyGameBatch, new_batch};

    #[pymodule_export]
    use super::new;

    #[pymodule_export]
    use crate::core::enums::{Decks, Stakes};

    #[pymodule_export]
    use crate::core::game::ActionKind;

    #[pyfunction]
    fn build_profile() -> &'static str {
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    }

    #[pyfunction]
    fn generate_discard_table(
        total_cards: usize,
        suit_counts: Vec<usize>,
        suit_scores: Vec<i64>,
        rank_counts: Vec<usize>,
        rank_scores: Vec<i64>,
        suit_rank_counts: Vec<usize>,
        suit_rank_scores: Vec<i64>,
        hand: Vec<(u8, u8, i64)>,
    ) -> PyResult<Vec<(i128, f64, Vec<(u8, u8, i64)>)>> {
        let table = crate::calculation::poker_discards::generate_discard_table(
            total_cards,
            suit_counts,
            suit_scores,
            rank_counts,
            rank_scores,
            suit_rank_counts,
            suit_rank_scores,
            hand,
        );

        Ok(table)
    }
}
