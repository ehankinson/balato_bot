use pyo3::prelude::*;
mod calculation;
mod core;

use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[pymodule]
mod balatro_engine {
    use pyo3::prelude::*;

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
    ) -> PyResult<()> {
        // Validate inputs.
        // Construct DeckSummary.
        // Construct Vec<CardInput>.
        // Call the internal Rust calculation.

        crate::calculation::poker_discards::generate_discard_table(
            total_cards,
            suit_counts,
            suit_scores,
            rank_counts,
            rank_scores,
            suit_rank_counts,
            suit_rank_scores,
            hand,
        );

        Ok(())
    }
}
