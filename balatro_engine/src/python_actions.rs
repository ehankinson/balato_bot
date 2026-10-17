use crate::core::blind::BlindKind;
use crate::core::consumable::ConsumableTarget;
use crate::core::game::{ActionKind, Game, GameAction};
use pyo3::prelude::*;

pub(crate) fn command(
    game: &Game,
    action: ActionKind,
    source: Option<usize>,
    targets: Option<Vec<usize>>,
    order: Option<Vec<usize>>,
) -> PyResult<GameAction> {
    let error = |message: &str| pyo3::exceptions::PyValueError::new_err(message.to_owned());
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
            game.state()
                .consumables
                .get(source.unwrap())
                .map(|owned| owned.consumable)
        } else {
            game.pending_pack()
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
        let selector = game.state().blind_selector;
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
    Ok(command)
}
