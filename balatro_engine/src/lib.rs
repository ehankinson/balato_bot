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

    #[pyo3(signature = (action, source=None, targets=None, order=None))]
    fn apply_action(
        &mut self,
        action: ActionKind,
        source: Option<&Bound<'_, PyAny>>,
        targets: Option<&Bound<'_, PyAny>>,
        order: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<u8> {
        let error = |message: &str| pyo3::exceptions::PyValueError::new_err(message.to_owned());
        let source = source.map(selected_index).transpose()?;
        let targets = targets.map(selected_indices).transpose()?;
        let order = order.map(selected_indices).transpose()?;
        if source.is_some() != action.has_source() {
            return Err(error("action requires exactly its source field"));
        }
        let is_reorder = matches!(action, ActionKind::ReorderHand | ActionKind::ReorderJokers);
        if order.is_some() != is_reorder {
            return Err(error("only reorder actions require order"));
        }
        let uses_targets = matches!(
            action,
            ActionKind::PlayCards
                | ActionKind::DiscardCards
                | ActionKind::UseConsumable
                | ActionKind::UsePackConsumable
        );
        if targets.is_some() && !uses_targets {
            return Err(error("action does not accept targets"));
        }
        if matches!(action, ActionKind::PlayCards | ActionKind::DiscardCards) && targets.is_none() {
            return Err(error("card actions require targets"));
        }
        if matches!(
            action,
            ActionKind::UseConsumable | ActionKind::UsePackConsumable
        ) {
            let card = if action == ActionKind::UseConsumable {
                self.game
                    .state()
                    .consumables
                    .get(source.unwrap())
                    .map(|owned| owned.consumable)
            } else {
                self.game
                    .pending_pack()
                    .and_then(|pending| pending.opening.options().get(source.unwrap()))
                    .and_then(|option| match option {
                        crate::core::pack::PackOption::Consumable(card) => Some(*card),
                        _ => None,
                    })
            };
            if let Some(card) = card {
                if targets.is_some() != (card.targets().maximum > 0) {
                    return Err(error("consumable requires exactly its target fields"));
                }
            }
        }
        let blind_kind = |index: usize| -> PyResult<BlindKind> {
            let selector = self.game.state().blind_selector;
            match index {
                0 => Ok(selector.small().kind),
                1 => Ok(selector.big().kind),
                2 => Ok(selector.boss().kind),
                _ => Err(error("blind source must be 0, 1, or 2")),
            }
        };
        let command = match action {
            ActionKind::SelectBlind => GameAction::SelectBlind(blind_kind(source.unwrap())?),
            ActionKind::SkipBlind => GameAction::SkipBlind(blind_kind(source.unwrap())?),
            ActionKind::PlayCards => GameAction::PlayCards(targets.unwrap()),
            ActionKind::DiscardCards => GameAction::DiscardCards(targets.unwrap()),
            ActionKind::BuyShopItem => GameAction::BuyShopItem(source.unwrap()),
            ActionKind::BuyVoucher => GameAction::BuyVoucher(source.unwrap()),
            ActionKind::RerollShop => GameAction::RerollShop,
            ActionKind::BuyPack => GameAction::BuyPack(source.unwrap()),
            ActionKind::SelectPackOption => GameAction::SelectPackOption(source.unwrap()),
            ActionKind::UseConsumable => GameAction::UseConsumable {
                index: source.unwrap(),
                target: ConsumableTarget::hand(targets.unwrap_or_default()),
            },
            ActionKind::SellJoker => GameAction::SellJoker(source.unwrap()),
            ActionKind::SellConsumable => GameAction::SellConsumable(source.unwrap()),
            ActionKind::LeaveShop => GameAction::LeaveShop,
            ActionKind::SkipPack => GameAction::SkipPack,
            ActionKind::UsePackConsumable => GameAction::UsePackConsumable {
                index: source.unwrap(),
                target: ConsumableTarget::hand(targets.unwrap_or_default()),
            },
            ActionKind::ReorderJokers => GameAction::ReorderJokers(order.unwrap()),
            ActionKind::ReorderHand => GameAction::ReorderHand(order.unwrap()),
            ActionKind::RerollBoss => GameAction::RerollBoss,
        };
        self.game
            .apply_action(command)
            .map(|point| point.code())
            .map_err(|error| pyo3::exceptions::PyValueError::new_err(format!("{error:?}")))
    }
}

// Stage 2 keeps selected values as strict Python ints/lists. Stage 4 supplies
// the contiguous int32-array boundary without moving validation into Python.
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

fn selected_indices(value: &Bound<'_, PyAny>) -> PyResult<Vec<usize>> {
    value
        .cast::<pyo3::types::PyList>()?
        .iter()
        .map(|value| selected_index(&value))
        .collect()
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
