//! Read-only command rules and public selection constraints. No RNG or scoring.
use crate::core::blind::BlindKind;
use crate::core::consumable::ConsumableUseSource;
use crate::core::enums::{BossBlinds, Consumable, Spectral};
use crate::core::game::{ActionError, ActionKind, DecisionError, DecisionPoint, Game, GameAction};
use crate::core::hand::Hand;
use crate::core::pack::{PackError, PackOption};
use crate::core::selection::{valid_indices, valid_permutation};
use crate::core::shop::{ShopError, require_funds};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub(crate) enum TargetCollection {
    None = 0,
    Hand = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub(crate) enum PermutationCollection {
    None = 0,
    Hand = 1,
    Jokers = 2,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ActionConstraints {
    pub(crate) source_mask: Vec<bool>,
    pub(crate) source_known: Vec<bool>,
    pub(crate) target_collection: TargetCollection,
    pub(crate) target_mask: Vec<bool>,
    pub(crate) required_targets: Vec<bool>,
    pub(crate) count_mask: Vec<bool>,
    pub(crate) ordered: bool,
    pub(crate) permutation_collection: PermutationCollection,
    pub(crate) available: bool,
    pub(crate) can_submit: bool,
}

impl ActionKind {
    pub(crate) const ALL: [Self; 18] = [
        Self::SelectBlind,
        Self::SkipBlind,
        Self::PlayCards,
        Self::DiscardCards,
        Self::BuyShopItem,
        Self::BuyVoucher,
        Self::RerollShop,
        Self::BuyPack,
        Self::SelectPackOption,
        Self::UseConsumable,
        Self::SellJoker,
        Self::SellConsumable,
        Self::LeaveShop,
        Self::SkipPack,
        Self::UsePackConsumable,
        Self::ReorderJokers,
        Self::ReorderHand,
        Self::RerollBoss,
    ];

    pub(crate) const fn has_source(self) -> bool {
        matches!(
            self,
            Self::SelectBlind
                | Self::SkipBlind
                | Self::BuyShopItem
                | Self::BuyVoucher
                | Self::BuyPack
                | Self::SelectPackOption
                | Self::UseConsumable
                | Self::UsePackConsumable
                | Self::SellJoker
                | Self::SellConsumable
        )
    }
}

impl Game {
    /// Final rule validation may inspect concealed facts; public masks below may not.
    pub(crate) fn validate_action(&self, action: &GameAction) -> Result<(), ActionError> {
        use GameAction::*;
        if self.decision_point() == DecisionPoint::RunComplete {
            return Err(ActionError::InvalidPhase);
        }
        let empty = Hand::new();
        let hand = self.active_hand().unwrap_or(&empty);
        match action {
            SelectBlind(kind) | SkipBlind(kind) => {
                self.require_phase(DecisionPoint::BlindSelection)?;
                if *kind != self.available_blind() {
                    return Err(ActionError::Decision(DecisionError::InvalidBlindChoice));
                }
                if matches!(action, SkipBlind(_)) && kind.is_boss() {
                    return Err(ActionError::BlindSkip(
                        crate::core::blind::BlindSkipError::BossBlindCannotBeSkipped,
                    ));
                }
            }
            PlayCards(indices) | DiscardCards(indices) => {
                self.require_phase(DecisionPoint::PlayingHand)?;
                if indices.is_empty()
                    || indices.len() > 5
                    || !valid_indices(indices, hand.cards().len())
                {
                    return Err(ActionError::InvalidAction);
                }
                match action {
                    PlayCards(_) => self
                        .state
                        .blind
                        .validate_play(indices)
                        .map_err(|_| ActionError::InvalidAction)?,
                    _ if self.state.blind.discards_remaining == 0 => {
                        return Err(ActionError::InvalidAction);
                    }
                    _ => {}
                }
            }
            BuyShopItem(index) => self
                .active_shop()?
                .items
                .get(*index)
                .ok_or(ActionError::Shop(ShopError::InvalidOffer))?
                .validate_purchase(&self.state)
                .map_err(ActionError::Shop)?,
            BuyVoucher(index) => {
                self.active_shop()?;
                self.state
                    .validate_voucher_purchase(*index)
                    .map_err(ActionError::Shop)?;
            }
            RerollShop => require_funds(&self.state, self.active_shop()?.reroll_cost)
                .map_err(ActionError::Shop)?,
            BuyPack(index) => {
                let pack = self
                    .active_shop()?
                    .packs
                    .get(*index)
                    .ok_or(ActionError::Shop(ShopError::InvalidOffer))?;
                require_funds(&self.state, pack.price(&self.state)).map_err(ActionError::Shop)?;
            }
            SelectPackOption(index) => self
                .active_pack()?
                .validate_select(*index, &self.state)
                .map_err(ActionError::Pack)?,
            SkipPack => {
                if self.active_pack()?.choices() == 0 {
                    return Err(ActionError::InvalidAction);
                }
            }
            UsePackConsumable { index, target } => {
                let PackOption::Consumable(card) = self
                    .active_pack()?
                    .option(*index)
                    .map_err(ActionError::Pack)?
                else {
                    return Err(ActionError::Pack(PackError::InvalidSelection));
                };
                card.validate_use(&self.state, hand, target, ConsumableUseSource::default())
                    .map_err(|error| ActionError::Pack(PackError::Consumable(error)))?;
            }
            UseConsumable { index, target } => {
                self.require_inventory_phase()?;
                let owned = self
                    .state
                    .consumables
                    .get(*index)
                    .ok_or(ActionError::Consumable(
                        crate::core::consumable::ConsumableUseError::ConsumableNotFound,
                    ))?;
                owned
                    .consumable
                    .validate_use(
                        &self.state,
                        hand,
                        target,
                        ConsumableUseSource {
                            inventory_index: Some(*index),
                        },
                    )
                    .map_err(ActionError::Consumable)?;
            }
            SellJoker(index) => {
                self.require_inventory_phase()?;
                if self
                    .state
                    .jokers
                    .get(*index)
                    .is_none_or(|joker| joker.is_eternal())
                {
                    return Err(ActionError::InvalidAction);
                }
            }
            SellConsumable(index) => {
                self.require_inventory_phase()?;
                if *index >= self.state.consumables.len() {
                    return Err(ActionError::InvalidAction);
                }
            }
            LeaveShop => {
                self.active_shop()?;
            }
            ReorderJokers(order) => {
                self.require_inventory_phase()?;
                if !valid_permutation(order, self.state.jokers.as_slice().len()) {
                    return Err(ActionError::InvalidAction);
                }
            }
            ReorderHand(order) => {
                if !self.has_active_hand() {
                    return Err(ActionError::InvalidPhase);
                }
                if !valid_permutation(order, hand.cards().len()) {
                    return Err(ActionError::InvalidAction);
                }
            }
            RerollBoss => self.validate_boss_reroll()?,
        }
        Ok(())
    }

    fn require_phase(&self, phase: DecisionPoint) -> Result<(), ActionError> {
        if self.decision_point() == phase {
            Ok(())
        } else {
            Err(ActionError::InvalidPhase)
        }
    }

    fn require_inventory_phase(&self) -> Result<(), ActionError> {
        if matches!(
            self.decision_point(),
            DecisionPoint::BlindSelection
                | DecisionPoint::PlayingHand
                | DecisionPoint::Shop
                | DecisionPoint::PackSelection
        ) {
            Ok(())
        } else {
            Err(ActionError::InvalidPhase)
        }
    }

    fn active_shop(&self) -> Result<&crate::core::shop::ShopState, ActionError> {
        self.require_phase(DecisionPoint::Shop)?;
        self.shop().ok_or(ActionError::InvalidPhase)
    }

    fn active_pack(&self) -> Result<&crate::core::pack::PackOpening, ActionError> {
        self.require_phase(DecisionPoint::PackSelection)?;
        self.pending_pack()
            .map(|pending| &pending.opening)
            .ok_or(ActionError::InvalidAction)
    }

    pub(crate) fn action_kinds(&self) -> [bool; 18] {
        ActionKind::ALL.map(|kind| {
            self.action_constraints(kind, None)
                .expect("no source query")
                .available
        })
    }

    pub(crate) fn action_constraints(
        &self,
        kind: ActionKind,
        source: Option<usize>,
    ) -> Result<ActionConstraints, ActionError> {
        self.selection_constraints(kind, source, &[])
    }

    pub(crate) fn selection_constraints(
        &self,
        kind: ActionKind,
        source: Option<usize>,
        prefix: &[usize],
    ) -> Result<ActionConstraints, ActionError> {
        use ActionKind::*;
        if source.is_some() && !kind.has_source() {
            return Err(ActionError::InvalidAction);
        }
        let empty = Hand::new();
        let hand = self.active_hand().unwrap_or(&empty);
        let mut result = ActionConstraints {
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
        };
        let source_length = match kind {
            SelectBlind | SkipBlind => 3,
            BuyShopItem => self.shop().map_or(0, |shop| shop.items.len()),
            BuyVoucher => self.shop().map_or(0, |shop| 1 + shop.extra_vouchers.len()),
            BuyPack => self.shop().map_or(0, |shop| shop.packs.len()),
            SelectPackOption | UsePackConsumable => self
                .pending_pack()
                .map_or(0, |pending| pending.opening.options().len()),
            UseConsumable | SellConsumable => self.state.consumables.len(),
            SellJoker => self.state.jokers.as_slice().len(),
            _ => 0,
        };
        if source.is_some_and(|index| index >= source_length) {
            return Err(ActionError::InvalidAction);
        }
        for index in 0..source_length {
            let (feasible, known) = match kind {
                SelectBlind | SkipBlind => {
                    let blind = [
                        self.state.blind_selector.small().kind,
                        self.state.blind_selector.big().kind,
                        self.state.blind_selector.boss().kind,
                    ][index];
                    let action = if kind == SelectBlind {
                        GameAction::SelectBlind(blind)
                    } else {
                        GameAction::SkipBlind(blind)
                    };
                    (self.validate_action(&action).is_ok(), true)
                }
                BuyShopItem => (
                    self.validate_action(&GameAction::BuyShopItem(index))
                        .is_ok(),
                    true,
                ),
                BuyVoucher => (
                    self.validate_action(&GameAction::BuyVoucher(index)).is_ok(),
                    true,
                ),
                BuyPack => (
                    self.validate_action(&GameAction::BuyPack(index)).is_ok(),
                    true,
                ),
                SelectPackOption => (
                    self.validate_action(&GameAction::SelectPackOption(index))
                        .is_ok(),
                    true,
                ),
                SellConsumable => (
                    self.validate_action(&GameAction::SellConsumable(index))
                        .is_ok(),
                    true,
                ),
                SellJoker => {
                    if self.state.jokers.get(index).unwrap().is_face_down() {
                        (self.require_inventory_phase().is_ok(), false)
                    } else {
                        (
                            self.validate_action(&GameAction::SellJoker(index)).is_ok(),
                            true,
                        )
                    }
                }
                UseConsumable | UsePackConsumable => {
                    let card = self.consumable_source(kind, index);
                    let phase_ok = if kind == UseConsumable {
                        self.require_inventory_phase().is_ok()
                    } else {
                        self.active_pack().is_ok()
                    };
                    if phase_ok {
                        card.map_or((false, true), |card| {
                            self.public_consumable_availability(card, hand, kind, index)
                        })
                    } else {
                        (false, true)
                    }
                }
                _ => unreachable!("source action"),
            };
            result.source_mask.push(feasible);
            result.source_known.push(known);
        }
        let source_feasible = source.is_some_and(|index| result.source_mask[index]);
        if kind.has_source() {
            result.available = result.source_mask.iter().any(|feasible| *feasible);
            result.count_mask[0] = source_feasible;
        }
        match kind {
            PlayCards | DiscardCards => {
                let available = self.decision_point() == DecisionPoint::PlayingHand
                    && if kind == PlayCards {
                        self.state.blind.hands_remaining > 0
                    } else {
                        self.state.blind.discards_remaining > 0
                    };
                result.target_collection = TargetCollection::Hand;
                result.target_mask = vec![available; hand.cards().len()];
                result.required_targets = vec![false; hand.cards().len()];
                if available
                    && kind == PlayCards
                    && !self.state.blind.boss_ability_disabled
                    && matches!(
                        self.state.blind.kind,
                        BlindKind::Boss(BossBlinds::CeruleanBell)
                    )
                {
                    if let Some(index) = self.state.blind.forced_card_index {
                        if let Some(required) = result.required_targets.get_mut(index) {
                            *required = true;
                        }
                    }
                }
                result.count_mask = (0..=hand.cards().len())
                    .map(|count| available && (1..=5).contains(&count))
                    .collect();
                result.available = result.count_mask.iter().any(|count| *count);
            }
            UseConsumable | UsePackConsumable => {
                if let Some(card) = source.and_then(|index| self.consumable_source(kind, index)) {
                    let limits = card.targets();
                    result.ordered = limits.ordered;
                    if limits.maximum > 0 {
                        result.target_collection = TargetCollection::Hand;
                        result.target_mask = vec![source_feasible; hand.cards().len()];
                        result.required_targets = vec![false; hand.cards().len()];
                        result.count_mask = (0..=hand.cards().len())
                            .map(|count| {
                                source_feasible
                                    && (limits.minimum..=limits.maximum).contains(&count)
                            })
                            .collect();
                    }
                }
            }
            ReorderHand | ReorderJokers => {
                let (available, length, collection) = if kind == ReorderHand {
                    (
                        self.has_active_hand(),
                        hand.cards().len(),
                        PermutationCollection::Hand,
                    )
                } else {
                    (
                        self.require_inventory_phase().is_ok(),
                        self.state.jokers.as_slice().len(),
                        PermutationCollection::Jokers,
                    )
                };
                result.permutation_collection = collection;
                result.ordered = true;
                result.target_mask = vec![available; length];
                result.required_targets = vec![available; length];
                result.count_mask = vec![false; length + 1];
                result.count_mask[length] = available;
                result.available = available;
            }
            RerollShop | LeaveShop | SkipPack | RerollBoss => {
                let action = match kind {
                    RerollShop => GameAction::RerollShop,
                    LeaveShop => GameAction::LeaveShop,
                    SkipPack => GameAction::SkipPack,
                    _ => GameAction::RerollBoss,
                };
                result.available = self.validate_action(&action).is_ok();
                result.count_mask[0] = result.available;
            }
            _ => {}
        }
        if !valid_indices(prefix, result.target_mask.len())
            || prefix.iter().any(|index| !result.target_mask[*index])
            || !result
                .count_mask
                .iter()
                .enumerate()
                .any(|(count, valid)| *valid && count >= prefix.len())
        {
            // Empty prefixes are allowed for unavailable kinds to return their masks.
            if !prefix.is_empty() {
                return Err(ActionError::InvalidAction);
            }
        }
        result.can_submit = result
            .count_mask
            .get(prefix.len())
            .copied()
            .unwrap_or(false)
            && result
                .required_targets
                .iter()
                .enumerate()
                .all(|(index, required)| !required || prefix.contains(&index));
        // Keep cardinalities as total counts, while masking completions that cannot
        // fit all remaining mandatory targets after this prefix.
        let missing = result
            .required_targets
            .iter()
            .enumerate()
            .filter(|(index, required)| **required && !prefix.contains(index))
            .count();
        for (count, feasible) in result.count_mask.iter_mut().enumerate() {
            *feasible &= count >= prefix.len() + missing;
        }
        if !prefix.is_empty() && !result.count_mask.iter().any(|feasible| *feasible) {
            return Err(ActionError::InvalidAction);
        }
        for index in prefix {
            result.target_mask[*index] = false;
        }
        let can_extend = result
            .count_mask
            .iter()
            .enumerate()
            .any(|(count, valid)| *valid && count > prefix.len());
        if !can_extend {
            result.target_mask.fill(false);
        }
        if missing > 0
            && !result
                .count_mask
                .iter()
                .enumerate()
                .any(|(count, valid)| *valid && count > prefix.len() + missing)
        {
            for (index, selectable) in result.target_mask.iter_mut().enumerate() {
                *selectable &= result.required_targets[index];
            }
        }
        Ok(result)
    }

    fn consumable_source(&self, kind: ActionKind, index: usize) -> Option<Consumable> {
        if kind == ActionKind::UseConsumable {
            self.state
                .consumables
                .get(index)
                .map(|owned| owned.consumable)
        } else {
            match self.pending_pack()?.opening.option(index).ok()? {
                PackOption::Consumable(card) => Some(*card),
                _ => None,
            }
        }
    }

    fn public_consumable_availability(
        &self,
        card: Consumable,
        hand: &Hand,
        kind: ActionKind,
        index: usize,
    ) -> (bool, bool) {
        let source = ConsumableUseSource {
            inventory_index: (kind == ActionKind::UseConsumable).then_some(index),
        };
        if card == Consumable::Spectral(Spectral::Ankh) {
            let jokers = self.state.jokers.as_slice();
            if jokers.iter().any(|joker| joker.is_face_down()) {
                return (!jokers.is_empty(), false);
            }
            let valid = (0..jokers.len())
                .map(|index| self.state.jokers.can_duplicate_after_destroy_others(index))
                .collect::<Vec<_>>();
            return (
                valid.iter().any(|value| *value),
                !valid.iter().any(|value| *value) || valid.iter().all(|value| *value),
            );
        }
        (
            card.validate_availability(&self.state, hand, source)
                .is_ok(),
            true,
        )
    }
}
