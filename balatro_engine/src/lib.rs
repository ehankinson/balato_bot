mod calculation;
mod consts;
mod core;

use pyo3::prelude::*;

use crate::core::blind::BlindKind;
use crate::core::consumable::ConsumableTarget;
use crate::core::game::{ActionKind, GameAction};

#[pyclass(name = "Game")]
struct PyGame {
    game: crate::core::game::Game,
}

#[pymethods]
impl PyGame {
    fn next_step(&mut self) -> u8 {
        self.game.next_step().code()
    }

    #[getter]
    fn ante(&self) -> u8 {
        self.game.state().counters.ante
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

    #[pyo3(signature = (action, index=None, positions=None, selections=None))]
    fn apply_action(
        &mut self,
        action: ActionKind,
        index: Option<usize>,
        positions: Option<Vec<u8>>,
        selections: Option<Vec<usize>>,
    ) -> PyResult<u8> {
        let blind_kind = |index: Option<usize>| -> PyResult<BlindKind> {
            let selector = self.game.state().blind_selector;
            match index.unwrap_or(0) {
                0 => Ok(selector.small().kind),
                1 => Ok(selector.big().kind),
                2 => Ok(selector.boss().kind),
                _ => Err(pyo3::exceptions::PyValueError::new_err(
                    "blind index must be 0, 1, or 2",
                )),
            }
        };
        let action = match action {
            ActionKind::SelectBlind => GameAction::SelectBlind(blind_kind(index)?),
            ActionKind::SkipBlind => GameAction::SkipBlind(blind_kind(index)?),
            ActionKind::PlayCards => GameAction::PlayCards(positions.ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err("play actions require positions")
            })?),
            ActionKind::DiscardCards => GameAction::DiscardCards(positions.ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err("discard actions require positions")
            })?),
            ActionKind::BuyShopItem => GameAction::BuyShopItem(index.ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err("shop item actions require an index")
            })?),
            ActionKind::BuyVoucher => GameAction::BuyVoucher(index.ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err("voucher actions require an index")
            })?),
            ActionKind::RerollShop => GameAction::RerollShop,
            ActionKind::BuyPack => GameAction::BuyPack(index.ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err("pack actions require an index")
            })?),
            ActionKind::SelectPackOptions => {
                GameAction::SelectPackOptions(selections.ok_or_else(|| {
                    pyo3::exceptions::PyValueError::new_err("pack choices require selections")
                })?)
            }
            ActionKind::UseConsumable => GameAction::UseConsumable {
                index: index.ok_or_else(|| {
                    pyo3::exceptions::PyValueError::new_err("consumable actions require an index")
                })?,
                target: positions
                    .map(|indices| {
                        ConsumableTarget::hand(
                            indices.into_iter().map(usize::from).collect::<Vec<_>>(),
                        )
                    })
                    .unwrap_or_default(),
            },
            ActionKind::SellJoker => GameAction::SellJoker(index.ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err("selling a Joker requires an index")
            })?),
            ActionKind::SellConsumable => GameAction::SellConsumable(index.ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err("selling a consumable requires an index")
            })?),
            ActionKind::LeaveShop => GameAction::LeaveShop,
        };

        self.game
            .apply_action(action)
            .map(|point| point.code())
            .map_err(|error| pyo3::exceptions::PyValueError::new_err(format!("{error:?}")))
    }
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
    use super::new;

    #[pymodule_export]
    use crate::core::enums::{Decks, Stakes};

    #[pymodule_export]
    use crate::core::game::ActionKind;

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
