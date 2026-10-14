use chacha20::ChaCha12Rng;
use rand::prelude::{IndexedRandom, IteratorRandom, RngExt, SliceRandom};

use crate::core::blind::BlindState;
use crate::core::card::Card;
use crate::core::deck::Deck;
use crate::core::enums::{ALL_RANKS, ALL_SUITS, BossBlinds, Enhancement, PokerHand, Rank, Suit};
use crate::core::game::{GameCounters, GameState};
use crate::core::joker::{Joker, JokerData, JokerStructure};
use crate::core::joker_types::JokerUpdate::*;
use crate::core::joker_types::{
    CopyType, GameStateModifications, GenerateType, JokerEdition, JokerKind, JokerModifiers,
    JokerRarity, JokerTrigger, JokerUpdate, RetriggerTarget,
};

pub(crate) struct JokerContext<'a> {
    pub(crate) blind: &'a mut BlindState,
    pub(crate) deck: &'a mut Deck,
    pub(crate) money: &'a mut i16,
    pub(crate) counters: &'a mut GameCounters,
    pub(crate) most_played_hand: Option<PokerHand>,
    pub(crate) rng: &'a mut ChaCha12Rng,
    joker_count: usize,
    joker_capacity: usize,
    consumable_slots: usize,
    pub(crate) modifiers: JokerModifiers,
    owned: OwnedCardCounts,
}

#[derive(Default)]
struct OwnedCardCounts {
    count: usize,
    steel: usize,
    stone: usize,
    enhanced: usize,
    nines: usize,
}
impl OwnedCardCounts {
    fn from_cards<'a>(cards: impl Iterator<Item = &'a Card>) -> Self {
        let mut counts = Self::default();
        for card in cards {
            counts.count += 1;
            counts.steel += usize::from(card.enhancement() == Enhancement::Steel);
            counts.stone += usize::from(card.enhancement() == Enhancement::Stone);
            counts.enhanced += usize::from(card.enhancement() != Enhancement::None);
            counts.nines +=
                usize::from(card.enhancement() != Enhancement::Stone && card.rank() == Rank::Nine);
        }
        counts
    }
}
impl<'a> JokerContext<'a> {
    pub(crate) fn new(
        state: &'a mut GameState,
        joker_count: usize,
        joker_capacity: usize,
        modifiers: JokerModifiers,
    ) -> Self {
        let most_played_hand = state.most_played_hand();
        let consumable_slots = state
            .consumable_capacity()
            .saturating_sub(state.consumables.len());
        Self::from_parts(
            &mut state.blind,
            &mut state.deck,
            &mut state.money,
            &mut state.counters,
            most_played_hand,
            &mut state.rng,
            joker_count,
            joker_capacity,
            consumable_slots,
            modifiers,
        )
    }

    pub(crate) fn from_parts(
        blind: &'a mut BlindState,
        deck: &'a mut Deck,
        money: &'a mut i16,
        counters: &'a mut GameCounters,
        most_played_hand: Option<PokerHand>,
        rng: &'a mut ChaCha12Rng,
        joker_count: usize,
        joker_capacity: usize,
        consumable_slots: usize,
        modifiers: JokerModifiers,
    ) -> Self {
        Self {
            blind,
            deck,
            money,
            counters,
            most_played_hand,
            rng,
            joker_count,
            joker_capacity,
            consumable_slots,
            modifiers,
            owned: OwnedCardCounts::default(),
        }
    }

    fn empty_joker_slots(&self) -> usize {
        self.joker_capacity.saturating_sub(self.joker_count)
    }
}

/// One-shot events. All payloads borrow the current game objects so dispatch
/// does not clone cards or hands.
pub enum TriggerEvent<'a> {
    HandPlayed {
        played_cards: &'a [Card],
        scoring_card_indices: &'a [usize],
        hand_type: PokerHand,
    },
    PlayedCard {
        card: &'a Card,
        played_cards: &'a [Card],
        held_cards: &'a [Card],
        card_index: usize,
        scoring_card_indices: &'a [usize],
        hand_type: Option<PokerHand>,
        hands_remaining: u8,
    },
    HeldCard {
        card: &'a Card,
        held_cards: &'a [Card],
        card_index: usize,
    },
    Discard {
        cards: &'a [Card],
        is_discard_action: bool,
    },
    BeforePlayedCards {
        played_cards: &'a [Card],
        held_cards: &'a [Card],
    },
    AfterHand {
        played_cards: &'a [Card],
        scoring_card_indices: &'a [usize],
        held_cards: &'a [Card],
        hand_type: Option<PokerHand>,
        hands_remaining: u8,
        discards_remaining: u8,
        joker_count: u8,
    },
    BossBlindAbilityTriggered {
        boss: BossBlinds,
        during_hand_scoring: bool,
    },
}

/// Persistent events. `JokerUpdate` is the compact bitmask discriminant; this
/// enum adds the payload required by conditional updates.
pub enum UpdateEvent<'a> {
    JokerOrderChanged,
    BlindSelected {
        is_boss: bool,
    },
    DuringBlind,
    CardEnhancementAdded {
        card: &'a Card,
    },
    BlindCompleted {
        is_boss: bool,
    },
    HandCompleted {
        played_cards: &'a [Card],
        scoring_card_indices: &'a [usize],
        hand_type: Option<PokerHand>,
        hands_remaining: u8,
    },
    DeckChanged {
        cards_added: u16,
    },
    PlanetCardUsed,
    BoosterPackSkipped,
    BoosterPackOpened,
    TarotCardUsed,
    JokerSold {
        kind: JokerKind,
    },
    CardsDiscarded {
        cards: &'a [Card],
    },
    AfterCardSold,
    AfterPlayerDeath {
        chips_scored: u64,
        required_chips: u64,
    },
    CardDestroyed {
        card: &'a Card,
    },
    RoundStarted,
    RoundCompleted {
        held_cards: &'a [Card],
    },
    DiscardActionCompleted,
    LuckyCardSucceeded,
    ShopRerolled,
    BossBlindCompleted,
    ShopClosed,
}

impl UpdateEvent<'_> {
    fn kind(&self) -> JokerUpdate {
        match self {
            Self::JokerOrderChanged => JokerUpdate::JokerOrderChanged,
            Self::BlindSelected { .. } => JokerUpdate::BlindSelected,
            Self::DuringBlind => JokerUpdate::DuringBlind,
            Self::CardEnhancementAdded { .. } => JokerUpdate::CardEnhancementAdded,
            Self::BlindCompleted { .. } => JokerUpdate::BlindCompleted,
            Self::HandCompleted { .. } => JokerUpdate::HandCompleted,
            Self::DeckChanged { .. } => JokerUpdate::DeckChanged,
            Self::PlanetCardUsed => JokerUpdate::PlanetCardUsed,
            Self::BoosterPackSkipped => JokerUpdate::BoosterPackSkipped,
            Self::BoosterPackOpened => JokerUpdate::BoosterPackOpened,
            Self::TarotCardUsed => JokerUpdate::TarotCardUsed,
            Self::JokerSold { .. } => JokerUpdate::JokerSold,
            Self::CardsDiscarded { .. } => JokerUpdate::CardsDiscarded,
            Self::AfterCardSold => JokerUpdate::AfterCardSold,
            Self::AfterPlayerDeath { .. } => JokerUpdate::AfterPlayerDeath,
            Self::CardDestroyed { .. } => JokerUpdate::CardDestroyed,
            Self::RoundStarted => JokerUpdate::RoundStarted,
            Self::RoundCompleted { .. } => JokerUpdate::RoundCompleted,
            Self::DiscardActionCompleted => JokerUpdate::DiscardActionCompleted,
            Self::LuckyCardSucceeded => JokerUpdate::LuckyCardSucceeded,
            Self::ShopRerolled => JokerUpdate::ShopRerolled,
            Self::BossBlindCompleted => JokerUpdate::BossBlindCompleted,
            Self::ShopClosed => JokerUpdate::ShopClosed,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct JokerScoringEffect {
    pub chips: i32,
    pub add_mult: i32,
    pub x_mult: f32,
}

impl Default for JokerScoringEffect {
    fn default() -> Self {
        Self {
            chips: 0,
            add_mult: 0,
            x_mult: 1.0,
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct JokerEconomyEffect {
    pub money: i16,
    pub sell_value_bonus: u8,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct JokerGameStateEffect {
    pub life: i16,
    pub hand_size: i8,
    pub hands: i8,
    pub discards: i8,
    pub remove_right_joker: bool,
    pub remove_random_joker: bool,
    pub remove_self: bool,
    pub disable_boss_blind: bool,
    pub prevent_death: bool,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct JokerRetriggerEffect {
    pub retriggers: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GeneratedConsumable {
    Tarot,
    Planet,
    Spectral,
}

/// Fully specified game operation emitted by a Joker activation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JokerOperation {
    AddPlayedCardChips {
        index: usize,
        chips: i32,
    },
    CopyPlayedCard {
        index: usize,
        count: u8,
    },
    GenerateJoker {
        rarity: JokerRarity,
        count: u8,
    },
    CopyRandomOwnedJoker {
        count: u8,
    },
    GenerateConsumable {
        kind: GeneratedConsumable,
        count: u8,
    },
    GenerateStoneCard {
        count: u8,
    },
    GenerateSealedCard {
        count: u8,
    },
    EnhancePlayedCard {
        index: usize,
        enhancement: Enhancement,
    },
    UpgradeHand {
        hand: PokerHand,
        levels: u8,
    },
    CopyNegativeConsumable {
        count: u8,
    },
    AddDoubleTag {
        count: u8,
    },
}

#[derive(Debug, PartialEq)]
pub struct JokerEffect {
    pub chips: i32,
    pub add_mult: i32,
    pub x_mult: f32,
    pub money: i16,
    pub life: i16,
    pub hand_size: i8,
    pub hands: i8,
    pub discards: i8,
    pub retriggers: u8,
    /// Operations preserve order when an activation requests multiple outputs.
    pub operations: Vec<JokerOperation>,
    pub remove_right_joker: bool,
    pub remove_random_joker: bool,
    pub remove_self: bool,
    pub disable_boss_blind: bool,
    pub prevent_death: bool,
    pub sell_value_bonus: u8,
    pub destroy_discarded_card: bool,
}

impl Default for JokerEffect {
    fn default() -> Self {
        Self {
            chips: 0,
            add_mult: 0,
            x_mult: 1.0,
            money: 0,
            life: 0,
            hand_size: 0,
            hands: 0,
            discards: 0,
            retriggers: 0,
            operations: Vec::new(),
            remove_right_joker: false,
            remove_random_joker: false,
            remove_self: false,
            disable_boss_blind: false,
            prevent_death: false,
            sell_value_bonus: 0,
            destroy_discarded_card: false,
        }
    }
}

impl JokerEffect {
    pub(crate) fn from_scoring(scoring: JokerScoringEffect) -> Self {
        Self {
            chips: scoring.chips,
            add_mult: scoring.add_mult,
            x_mult: scoring.x_mult,
            ..Self::default()
        }
    }

    pub(crate) fn economy(&self) -> JokerEconomyEffect {
        JokerEconomyEffect {
            money: self.money,
            sell_value_bonus: self.sell_value_bonus,
        }
    }

    pub(crate) fn game_state(&self) -> JokerGameStateEffect {
        JokerGameStateEffect {
            life: self.life,
            hand_size: self.hand_size,
            hands: self.hands,
            discards: self.discards,
            remove_right_joker: self.remove_right_joker,
            remove_random_joker: self.remove_random_joker,
            remove_self: self.remove_self,
            disable_boss_blind: self.disable_boss_blind,
            prevent_death: self.prevent_death,
        }
    }

    fn add_operation(&mut self, operation: JokerOperation) {
        self.operations.push(operation);
    }

    fn merge(&mut self, other: Self) {
        self.chips += other.chips;
        self.add_mult += other.add_mult;
        self.x_mult *= other.x_mult;
        self.money += other.money;
        self.life += other.life;
        self.hand_size += other.hand_size;
        self.hands += other.hands;
        self.discards += other.discards;
        self.retriggers += other.retriggers;
        self.operations.extend(other.operations);
        self.remove_right_joker |= other.remove_right_joker;
        self.remove_random_joker |= other.remove_random_joker;
        self.remove_self |= other.remove_self;
        self.destroy_discarded_card |= other.destroy_discarded_card;
        self.disable_boss_blind |= other.disable_boss_blind;
        self.prevent_death |= other.prevent_death;
        self.sell_value_bonus = self.sell_value_bonus.saturating_add(other.sell_value_bonus);
    }
}

pub struct Jokers {
    pub(super) jokers: Vec<Joker>,
    capacity: usize,
}

impl Jokers {
    pub(crate) const CAPACITY: usize = 5;

    pub fn new() -> Jokers {
        Jokers {
            jokers: Vec::with_capacity(Self::CAPACITY),
            capacity: Self::CAPACITY,
        }
    }

    pub fn add(&mut self, joker: Joker) {
        self.jokers.push(joker);
    }

    fn effective_index(&self, index: usize) -> Option<usize> {
        if index >= self.jokers.len() {
            return None;
        }

        let mut current = index;
        for _ in 0..self.jokers.len() {
            if self.jokers[current].is_debuffed() {
                return None;
            }
            let next = match &self.jokers[current].structure {
                JokerStructure::Copy(CopyType::Right) => current.checked_add(1),
                JokerStructure::Copy(CopyType::LeftMost) => (current > 0).then_some(0),
                JokerStructure::Normal(_) => {
                    return (current == index || self.jokers[current].kind.copy_compatible())
                        .then_some(current);
                }
            }?;
            if next >= self.jokers.len() || next == current {
                return None;
            }
            current = next;
        }
        None
    }

    pub(crate) fn trigger_at(
        &mut self,
        index: usize,
        event: &TriggerEvent<'_>,
        context: &mut JokerContext<'_>,
        joker_count: u8,
    ) -> JokerEffect {
        let Some(effective_index) = self.effective_index(index) else {
            return JokerEffect::default();
        };
        if self.jokers[index].is_debuffed() || self.jokers[effective_index].is_debuffed() {
            return JokerEffect::default();
        }
        // Copied scoring abilities read persistent growth; only the actual
        // scaler updates on a played card or in the before-scoring phase.
        if effective_index != index
            && matches!(
                (self.jokers[effective_index].kind, event),
                (JokerKind::Wee, TriggerEvent::PlayedCard { .. })
                    | (JokerKind::Vampire, TriggerEvent::HandPlayed { .. })
            )
        {
            return JokerEffect::default();
        }
        self.jokers[effective_index].trigger(event, context, joker_count)
    }

    pub(crate) fn trigger_retrigger_at(
        &mut self,
        index: usize,
        event: &TriggerEvent<'_>,
        context: &mut JokerContext<'_>,
    ) -> JokerRetriggerEffect {
        let Some(effective_index) = self.effective_index(index) else {
            return JokerRetriggerEffect::default();
        };
        if self.jokers[index].is_debuffed() || self.jokers[effective_index].is_debuffed() {
            return JokerRetriggerEffect::default();
        }
        if !matches!(
            &self.jokers[effective_index].structure,
            JokerStructure::Normal(JokerData::Retrigger(data))
                if !matches!(data.target, RetriggerTarget::HeldCards)
        ) {
            return JokerRetriggerEffect::default();
        }
        self.jokers[effective_index].trigger_retrigger(event, context)
    }

    pub(crate) fn is_played_retrigger(&self, index: usize) -> bool {
        self.effective_index(index).is_some_and(|effective_index| {
            matches!(
                &self.jokers[effective_index].structure,
                JokerStructure::Normal(JokerData::Retrigger(data))
                    if !matches!(data.target, RetriggerTarget::HeldCards)
            )
        })
    }

    pub(crate) fn is_retrigger(&self, index: usize) -> bool {
        self.effective_index(index).is_some_and(|effective_index| {
            matches!(
                &self.jokers[effective_index].structure,
                JokerStructure::Normal(JokerData::Retrigger(_))
            )
        })
    }

    pub(crate) fn mime_count(&self) -> usize {
        (0..self.jokers.len())
            .filter(|&index| {
                self.effective_index(index).is_some_and(|effective_index| {
                    self.jokers[effective_index].kind() == JokerKind::Mime
                })
            })
            .count()
    }

    pub(crate) fn apply_effect_in_context(
        effect: &JokerEffect,
        context: &mut JokerContext<'_>,
        jokers: &mut Jokers,
    ) {
        apply_game_effect(effect, context, jokers);
    }

    pub(crate) fn modifiers(&self) -> JokerModifiers {
        let mut modifiers = JokerModifiers::default();
        for joker in &self.jokers {
            if joker.is_debuffed() {
                continue;
            }
            let JokerStructure::Normal(JokerData::GameState(data)) = &joker.structure else {
                continue;
            };
            modifiers.four_fingers |= joker.kind == JokerKind::FourFingers;
            match data.game_state_modification {
                Some(GameStateModifications::AllFaces) => modifiers.all_faces = true,
                Some(GameStateModifications::PlayAllCards) => modifiers.splash = true,
                Some(GameStateModifications::AllowStraightGaps) => modifiers.straight_gaps = true,
                Some(GameStateModifications::DoubleSuit) => modifiers.smeared_suits = true,
                Some(GameStateModifications::DuplicateJokers) => modifiers.showman = true,
                Some(GameStateModifications::DoubleProbability) => {
                    modifiers.probability_multiplier *= 2.0
                }
                Some(GameStateModifications::FreePlanetCards) => modifiers.free_planets = true,
                Some(GameStateModifications::DisableBossBlind)
                    if joker.kind == JokerKind::Chicot =>
                {
                    modifiers.disable_boss = true
                }
                _ => {}
            }
        }
        modifiers
    }

    pub(crate) fn hand_size_modifier(&self) -> i8 {
        self.jokers
            .iter()
            .filter(|joker| !joker.is_debuffed())
            .map(|joker| match &joker.structure {
                JokerStructure::Normal(JokerData::GameState(data)) => data.hand_size,
                JokerStructure::Normal(JokerData::Scoring(data)) => data.hand_size,
                _ => 0,
            })
            .sum()
    }

    pub(crate) fn capacity(&self) -> usize {
        self.capacity
            + self
                .jokers
                .iter()
                .filter(|joker| joker.edition() == JokerEdition::Negative)
                .count()
    }

    pub(crate) fn can_add(&self, joker: &Joker) -> bool {
        self.jokers.len() < self.capacity() + usize::from(joker.edition() == JokerEdition::Negative)
    }

    pub(crate) fn can_duplicate_after_destroy_others(&self, index: usize) -> bool {
        if index >= self.jokers.len() {
            return false;
        }
        let survivors = self
            .jokers
            .iter()
            .enumerate()
            .filter(|(position, joker)| *position == index || joker.is_eternal())
            .map(|(_, joker)| joker)
            .collect::<Vec<_>>();
        let negative_slots = survivors
            .iter()
            .filter(|joker| joker.edition() == JokerEdition::Negative)
            .count();
        survivors.len() < self.capacity + negative_slots
    }

    pub(crate) fn reorder(state: &mut GameState, order: &[usize]) -> bool {
        if !crate::core::selection::valid_permutation(order, state.jokers.jokers.len()) {
            return false;
        }
        let mut old = std::mem::take(&mut state.jokers.jokers)
            .into_iter()
            .map(Some)
            .collect::<Vec<_>>();
        state.jokers.jokers = order
            .iter()
            .map(|&index| old[index].take().unwrap())
            .collect();
        state.update_jokers(UpdateEvent::JokerOrderChanged);
        true
    }

    pub(crate) fn decrease_capacity(&mut self, amount: usize) {
        self.capacity -= amount;
    }

    pub(crate) fn finish_round_stickers(&mut self) -> i16 {
        self.jokers
            .iter_mut()
            .map(Joker::finish_round_stickers)
            .sum()
    }

    pub(crate) fn increase_capacity(&mut self, amount: usize) {
        self.capacity = self.capacity.saturating_add(amount);
    }

    pub(crate) fn get(&self, index: usize) -> Option<&Joker> {
        self.jokers.get(index)
    }

    pub(crate) fn get_mut(&mut self, index: usize) -> Option<&mut Joker> {
        self.jokers.get_mut(index)
    }

    pub(crate) fn clone_at(&self, index: usize) -> Option<Joker> {
        self.jokers.get(index).cloned()
    }

    pub(crate) fn destroy_other_jokers(&mut self, index: usize) -> bool {
        if index >= self.jokers.len() {
            return false;
        }

        let mut position = 0;
        self.jokers.retain(|entry| {
            let keep = position == index || entry.is_eternal();
            position += 1;
            keep
        });
        true
    }

    pub(crate) fn shuffle(&mut self, rng: &mut ChaCha12Rng) {
        self.jokers.shuffle(rng);
    }

    pub(crate) fn disable_random(&mut self, rng: &mut ChaCha12Rng) -> Option<usize> {
        let index = (0..self.jokers.len())
            .filter(|&index| self.jokers.len() == 1 || !self.jokers[index].is_debuffed())
            .choose(rng)?;
        for joker in &mut self.jokers {
            joker.set_debuffed(false);
        }
        self.jokers[index].set_debuffed(true);
        Some(index)
    }

    pub(crate) fn clear_blind_effects(&mut self) {
        for joker in &mut self.jokers {
            joker.set_debuffed(false);
            joker.set_face_down(false);
        }
    }

    pub(crate) fn total_sell_value(&self) -> u16 {
        self.jokers
            .iter()
            .map(|joker| joker.sell_value() as u16)
            .sum()
    }

    #[cfg(test)]
    pub fn initialize(&mut self, game_state: &mut GameState) {
        self.update(UpdateEvent::RoundStarted, game_state);
        self.update(
            UpdateEvent::BlindCompleted {
                is_boss: game_state.blind.kind.is_boss(),
            },
            game_state,
        );
    }

    pub fn as_slice(&self) -> &[Joker] {
        &self.jokers
    }

    pub(crate) fn as_mut_slice(&mut self) -> &mut [Joker] {
        &mut self.jokers
    }

    pub fn trigger(&mut self, event: TriggerEvent<'_>, game_state: &mut GameState) -> JokerEffect {
        let joker_count = game_state.jokers.as_slice().len();
        let joker_capacity = game_state.jokers.capacity();
        let mut context =
            JokerContext::new(game_state, joker_count, joker_capacity, self.modifiers());
        self.trigger_in_context(event, &mut context)
    }

    pub(crate) fn trigger_in_context(
        &mut self,
        event: TriggerEvent<'_>,
        context: &mut JokerContext<'_>,
    ) -> JokerEffect {
        let mut effect = JokerEffect::default();
        let joker_count = self.jokers.len() as u8;
        for index in 0..self.jokers.len() {
            let joker_effect = self.trigger_at(index, &event, context, joker_count);
            apply_game_effect(&joker_effect, context, self);
            effect.merge(joker_effect);
        }
        effect
    }

    #[cfg(test)]
    pub fn update(&mut self, event: UpdateEvent<'_>, game_state: &mut GameState) -> JokerEffect {
        std::mem::swap(self, &mut game_state.jokers);
        let effect = game_state.update_jokers(event);
        std::mem::swap(self, &mut game_state.jokers);
        effect
    }

    fn update_at(
        &mut self,
        index: usize,
        event: &UpdateEvent<'_>,
        context: &mut JokerContext<'_>,
    ) -> JokerEffect {
        let Some(target) = self.effective_index(index) else {
            return JokerEffect::default();
        };
        if self.jokers[target].update_mask & event.kind().mask() == 0
            || (target != index && !self.jokers[target].copies_update(event))
        {
            return JokerEffect::default();
        }
        // Compatible one-shot updates do not mutate persistent target state.
        // Scalers receive their native update once and copies read the result
        // during scoring.
        self.jokers[target].update(event, context)
    }

    fn apply_update_effect(
        &mut self,
        index: usize,
        effect: &JokerEffect,
        original_end: usize,
        context: &mut JokerContext<'_>,
    ) -> Option<usize> {
        apply_game_effect(effect, context, self);
        if effect.sell_value_bonus > 0 {
            for joker in &mut self.jokers {
                joker.sell_value_bonus = joker
                    .sell_value_bonus
                    .saturating_add(effect.sell_value_bonus);
            }
            context.counters.sell_value_bonus = context
                .counters
                .sell_value_bonus
                .saturating_add(effect.sell_value_bonus);
        }
        let removed = if effect.remove_self {
            Some(index)
        } else if effect.remove_right_joker {
            // Dagger can consume an original neighbor, not a Joker generated
            // during this blind-selection event.
            (index + 1 < original_end).then_some(index + 1)
        } else if effect.remove_random_joker && self.jokers.len() > 1 {
            (0..self.jokers.len())
                .filter(|&victim| victim != index && !self.jokers[victim].is_eternal())
                .choose(context.rng)
        } else {
            None
        };
        let removed = removed.filter(|&victim| !self.jokers[victim].is_eternal());
        if let Some(removed) = removed {
            if effect.remove_right_joker {
                let growth = u32::from(self.jokers[removed].sell_value()) * 2;
                if let JokerStructure::Normal(JokerData::Scoring(data)) =
                    &mut self.jokers[index].structure
                {
                    data.add_mult += growth;
                }
            }
            self.jokers.remove(removed);
        }
        removed
    }

    pub(crate) fn dispatch_updates(state: &mut GameState, event: UpdateEvent<'_>) -> JokerEffect {
        let mut aggregate = JokerEffect::default();
        let mut cursor = 0;
        let mut original_end = state.jokers.as_slice().len();
        // Appends are visible to later handlers but remain outside this event's
        // activation boundary. Only removing an original entry shrinks it.
        while cursor < original_end {
            let index = cursor;
            cursor += 1;
            let (effect, removed) = {
                let joker_count = state.jokers.as_slice().len();
                let joker_capacity = state.jokers.capacity();
                let modifiers = state.jokers.modifiers();
                let most_played_hand = state.most_played_hand();
                let consumable_slots = state
                    .consumable_capacity()
                    .saturating_sub(state.consumables.len());
                let GameState {
                    blind,
                    deck,
                    jokers,
                    money,
                    counters,
                    rng,
                    ..
                } = state;
                let mut context = JokerContext::from_parts(
                    blind,
                    deck,
                    money,
                    counters,
                    most_played_hand,
                    rng,
                    joker_count,
                    joker_capacity,
                    consumable_slots,
                    modifiers,
                );
                let effect = jokers.update_at(index, &event, &mut context);
                let removed =
                    jokers.apply_update_effect(index, &effect, original_end, &mut context);
                (effect, removed)
            };
            if let Some(removed) = removed {
                if removed < original_end {
                    original_end -= 1;
                }
                if removed < cursor {
                    cursor -= 1;
                }
            }
            state.materialize_generated_effects(&effect.operations, &mut [], None);
            aggregate.merge(effect);
        }
        aggregate
    }

    #[cfg(test)]
    pub fn sell(&mut self, index: usize, game_state: &mut GameState) -> JokerEffect {
        std::mem::swap(self, &mut game_state.jokers);
        let effect = Self::dispatch_sale(game_state, index);
        std::mem::swap(self, &mut game_state.jokers);
        effect
    }

    pub(crate) fn dispatch_sale(state: &mut GameState, index: usize) -> JokerEffect {
        let effect = {
            let joker_count = state.jokers.as_slice().len();
            let joker_capacity = state.jokers.capacity();
            let modifiers = state.jokers.modifiers();
            let most_played_hand = state.most_played_hand();
            let consumable_slots = state
                .consumable_capacity()
                .saturating_sub(state.consumables.len());
            let GameState {
                blind,
                deck,
                jokers,
                money,
                counters,
                rng,
                ..
            } = state;
            let mut context = JokerContext::from_parts(
                blind,
                deck,
                money,
                counters,
                most_played_hand,
                rng,
                joker_count,
                joker_capacity,
                consumable_slots,
                modifiers,
            );
            let kind = jokers
                .effective_index(index)
                .map(|target| jokers.jokers[target].kind);
            let effect = kind.map_or_else(JokerEffect::default, |kind| {
                jokers.update_at(index, &UpdateEvent::JokerSold { kind }, &mut context)
            });
            apply_game_effect(&effect, &mut context, jokers);
            jokers.jokers.remove(index);
            context.blind.on_joker_sold();
            context.counters.cards_sold = context.counters.cards_sold.saturating_add(1);
            effect
        };
        // Invisible copies only from the remaining owned collection.
        state.materialize_generated_effects(&effect.operations, &mut [], None);
        effect
    }
}

fn apply_game_effect(effect: &JokerEffect, context: &mut JokerContext<'_>, jokers: &mut Jokers) {
    *context.money += effect.economy().money;
    let game_state_effect = effect.game_state();
    context.counters.life += game_state_effect.life;
    if game_state_effect.disable_boss_blind {
        context.blind.disable_boss_ability();
        jokers.clear_blind_effects();
        context.deck.clear_debuffs();
    }
    context.blind.prevent_death |= game_state_effect.prevent_death;
    context.deck.adjust_hand_size(game_state_effect.hand_size);
    if game_state_effect.hands >= 0 {
        context.blind.hands_remaining = context
            .blind
            .hands_remaining
            .saturating_add(game_state_effect.hands as u8);
    } else {
        context.blind.hands_remaining = context
            .blind
            .hands_remaining
            .saturating_sub(game_state_effect.hands.unsigned_abs());
    }
    if game_state_effect.discards >= 0 {
        context.blind.discards_remaining = context
            .blind
            .discards_remaining
            .saturating_add(game_state_effect.discards as u8);
    } else {
        context.blind.discards_remaining = context
            .blind
            .discards_remaining
            .saturating_sub(game_state_effect.discards.unsigned_abs());
    }
}

impl Joker {
    fn copies_update(&self, event: &UpdateEvent<'_>) -> bool {
        matches!(
            (event, self.kind),
            (
                UpdateEvent::BlindSelected { .. },
                JokerKind::Marble | JokerKind::RiffRaff | JokerKind::Burglar
            ) | (UpdateEvent::RoundStarted, JokerKind::Certificate)
                | (UpdateEvent::BoosterPackOpened, JokerKind::Hallucination)
                | (UpdateEvent::ShopClosed, JokerKind::Perkeo)
                | (
                    UpdateEvent::JokerSold { .. },
                    JokerKind::DietCola | JokerKind::Luchador
                )
        )
    }

    pub(super) fn update_mask(joker_kind: JokerKind) -> u32 {
        let mut mask = 0;
        match joker_kind {
            JokerKind::Glass | JokerKind::Canio => mask |= CardDestroyed.mask(),
            JokerKind::DelayedGratification => mask |= RoundCompleted.mask(),
            JokerKind::Marble
            | JokerKind::RiffRaff
            | JokerKind::Madness
            | JokerKind::CeremonialDagger
            | JokerKind::Burglar => mask |= BlindSelected.mask(),
            JokerKind::HitTheRoad => mask |= CardsDiscarded.mask() | RoundStarted.mask(),
            JokerKind::RideTheBus
            | JokerKind::Runner
            | JokerKind::SquareJoker
            | JokerKind::IceCream
            | JokerKind::Obelisk
            | JokerKind::SpareTrousers => mask |= HandCompleted.mask(),
            JokerKind::Invisible => mask |= RoundCompleted.mask() | JokerSold.mask(),
            JokerKind::Yorick => mask |= CardsDiscarded.mask(),
            JokerKind::TurtleBean
            | JokerKind::Popcorn
            | JokerKind::Egg
            | JokerKind::Cloud9
            | JokerKind::GiftCard
            | JokerKind::ToTheMoon
            | JokerKind::Golden
            | JokerKind::Satellite
            | JokerKind::GrosMichel
            | JokerKind::Cavendish => mask |= RoundCompleted.mask(),
            JokerKind::Constellation => mask |= PlanetCardUsed.mask(),
            JokerKind::FortuneTeller => mask |= TarotCardUsed.mask(),
            JokerKind::Hallucination => mask |= BoosterPackOpened.mask(),
            JokerKind::Green => mask |= HandCompleted.mask() | DiscardActionCompleted.mask(),
            JokerKind::RedCard => mask |= BoosterPackSkipped.mask(),
            JokerKind::Hologram => mask |= DeckChanged.mask(),
            JokerKind::Seltzer => mask |= BlindSelected.mask() | HandCompleted.mask(),
            JokerKind::LuckyCat => mask |= LuckyCardSucceeded.mask(),
            JokerKind::FlashCard => mask |= ShopRerolled.mask(),
            JokerKind::Ramen => mask |= CardsDiscarded.mask(),
            JokerKind::Castle => {
                mask |= CardsDiscarded.mask() | RoundStarted.mask() | RoundCompleted.mask()
            }
            JokerKind::Campfire => mask |= AfterCardSold.mask() | BossBlindCompleted.mask(),
            JokerKind::Ancient => mask |= RoundStarted.mask() | RoundCompleted.mask(),
            JokerKind::Idol | JokerKind::MailInRebate | JokerKind::ToDoList => {
                mask |= BlindCompleted.mask()
            }
            JokerKind::Rocket => mask |= RoundCompleted.mask() | BossBlindCompleted.mask(),
            JokerKind::Certificate => mask |= RoundStarted.mask(),
            JokerKind::Luchador | JokerKind::DietCola => mask |= JokerSold.mask(),
            JokerKind::MrBones => mask |= AfterPlayerDeath.mask(),
            JokerKind::Perkeo => mask |= ShopClosed.mask(),
            JokerKind::Blueprint | JokerKind::Brainstorm => mask |= JokerOrderChanged.mask(),
            _ => {}
        }
        mask
    }

    pub(crate) fn trigger(
        &mut self,
        event: &TriggerEvent<'_>,
        context: &mut JokerContext<'_>,
        joker_count: u8,
    ) -> JokerEffect {
        if self.is_debuffed() {
            return JokerEffect::default();
        }

        let (played, held) = match event {
            TriggerEvent::PlayedCard {
                played_cards,
                held_cards,
                ..
            }
            | TriggerEvent::AfterHand {
                played_cards,
                held_cards,
                ..
            }
            | TriggerEvent::BeforePlayedCards {
                played_cards,
                held_cards,
            } => (*played_cards, *held_cards),
            _ => (&[][..], &[][..]),
        };
        if matches!(
            self.kind,
            JokerKind::Steel | JokerKind::Stone | JokerKind::Erosion | JokerKind::DriversLicense
        ) {
            context.owned = OwnedCardCounts::from_cards(
                context
                    .deck
                    .cards()
                    .iter()
                    .chain(context.deck.discarded_cards())
                    .chain(played)
                    .chain(held),
            );
        }
        match (event, self.trigger) {
            (TriggerEvent::HandPlayed { .. }, _)
                if matches!(self.kind, JokerKind::Vampire | JokerKind::Space) =>
            {
                self.on_hand_played(event, context)
            }
            (TriggerEvent::HandPlayed { .. }, JokerTrigger::OnHandPlayed) => {
                self.on_hand_played(event, context)
            }
            (TriggerEvent::PlayedCard { card, .. }, _) if self.kind == JokerKind::Wee => {
                if !card.is_debuffed()
                    && card.enhancement() != Enhancement::Stone
                    && card.rank() == Rank::Two
                {
                    if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                        data.chips += 8;
                    }
                }
                JokerEffect::default()
            }
            (TriggerEvent::PlayedCard { .. }, JokerTrigger::OnPlayedCard) => {
                self.on_played_card(event, context, joker_count)
            }
            (TriggerEvent::HeldCard { .. }, JokerTrigger::OnHeldCard) => {
                self.on_held_card(event, context)
            }
            (
                TriggerEvent::Discard {
                    cards,
                    is_discard_action,
                },
                JokerTrigger::OnDiscard,
            ) => self.on_discard(cards, *is_discard_action, context),
            (
                TriggerEvent::Discard {
                    cards,
                    is_discard_action,
                },
                _,
            ) if matches!(self.kind, JokerKind::Faceless | JokerKind::TradingCard) => {
                self.on_discard(cards, *is_discard_action, context)
            }
            (TriggerEvent::BeforePlayedCards { .. }, JokerTrigger::BeforePlayedCards) => {
                self.on_before_played_cards(event)
            }
            (TriggerEvent::AfterHand { .. }, JokerTrigger::AfterHand) => {
                self.on_after_hand(event, context, joker_count)
            }
            (TriggerEvent::BossBlindAbilityTriggered { .. }, JokerTrigger::OnBossBlindAbility) => {
                self.on_boss_blind_ability(event)
            }
            _ => JokerEffect::default(),
        }
    }

    pub(crate) fn trigger_scoring(
        &mut self,
        event: &TriggerEvent<'_>,
        context: &mut JokerContext<'_>,
        joker_count: u8,
    ) -> JokerScoringEffect {
        if self.is_debuffed() || !matches!(self.trigger, JokerTrigger::OnPlayedCard) {
            return JokerScoringEffect::default();
        }

        let TriggerEvent::PlayedCard {
            card,
            played_cards,
            held_cards,
            card_index,
            scoring_card_indices,
            hand_type,
            hands_remaining,
        } = event
        else {
            return JokerScoringEffect::default();
        };

        if card.is_debuffed() {
            return JokerScoringEffect::default();
        }

        let JokerStructure::Normal(JokerData::Scoring(data)) = &self.structure else {
            return JokerScoringEffect::default();
        };

        if !scoring_condition(
            self.kind,
            data,
            card,
            played_cards,
            held_cards,
            *hand_type,
            *card_index,
            scoring_card_indices,
            *hands_remaining,
            context,
            joker_count,
        ) {
            return JokerScoringEffect::default();
        }

        let mut effect = JokerScoringEffect::default();
        add_scoring_effect(
            self.kind,
            data,
            card,
            played_cards,
            held_cards,
            *hand_type,
            context,
            joker_count,
            &mut effect,
        );
        effect
    }

    pub(crate) fn trigger_retrigger(
        &mut self,
        event: &TriggerEvent<'_>,
        context: &mut JokerContext<'_>,
    ) -> JokerRetriggerEffect {
        if self.is_debuffed() {
            return JokerRetriggerEffect::default();
        }

        let TriggerEvent::PlayedCard {
            card,
            card_index,
            scoring_card_indices,
            hands_remaining,
            ..
        } = event
        else {
            return JokerRetriggerEffect::default();
        };

        let JokerStructure::Normal(JokerData::Retrigger(data)) = &self.structure else {
            return JokerRetriggerEffect::default();
        };

        if self.kind == JokerKind::Seltzer && self.active_hands == 0 {
            return JokerRetriggerEffect::default();
        }

        if card.is_debuffed()
            || !retrigger_matches(
                data.target,
                card,
                *card_index,
                scoring_card_indices.first().copied(),
                *hands_remaining,
                context.modifiers,
            )
        {
            return JokerRetriggerEffect::default();
        }

        JokerRetriggerEffect {
            retriggers: data.retrigger,
        }
    }

    pub(crate) fn trigger_after_hand_scoring(
        &mut self,
        event: &TriggerEvent<'_>,
        context: &mut JokerContext<'_>,
        joker_count: u8,
    ) -> JokerScoringEffect {
        if self.is_debuffed() || !matches!(self.trigger, JokerTrigger::AfterHand) {
            return JokerScoringEffect::default();
        }

        let TriggerEvent::AfterHand {
            played_cards,
            scoring_card_indices,
            held_cards,
            hand_type,
            hands_remaining,
            ..
        } = event
        else {
            return JokerScoringEffect::default();
        };

        let Some(card) = played_cards.first().or_else(|| held_cards.first()) else {
            return JokerScoringEffect::default();
        };
        let JokerStructure::Normal(JokerData::Scoring(data)) = &self.structure else {
            return JokerScoringEffect::default();
        };

        if !scoring_condition(
            self.kind,
            data,
            card,
            played_cards,
            held_cards,
            *hand_type,
            0,
            scoring_card_indices,
            *hands_remaining,
            context,
            joker_count,
        ) {
            return JokerScoringEffect::default();
        }

        let mut effect = JokerScoringEffect::default();
        add_scoring_effect(
            self.kind,
            data,
            card,
            played_cards,
            held_cards,
            *hand_type,
            context,
            joker_count,
            &mut effect,
        );
        effect
    }

    fn on_played_card(
        &mut self,
        event: &TriggerEvent<'_>,
        context: &mut JokerContext<'_>,
        joker_count: u8,
    ) -> JokerEffect {
        let TriggerEvent::PlayedCard {
            card,
            played_cards: _,
            held_cards: _,
            card_index: _,
            hand_type: _,
            hands_remaining: _,
            ..
        } = event
        else {
            return JokerEffect::default();
        };

        if card.is_debuffed() {
            return JokerEffect::default();
        }

        if self.kind == JokerKind::Hiker {
            let TriggerEvent::PlayedCard { card_index, .. } = event else {
                unreachable!()
            };
            let mut effect = JokerEffect::default();
            effect.add_operation(JokerOperation::AddPlayedCardChips {
                index: *card_index,
                chips: 5,
            });
            return effect;
        }
        match &self.structure {
            JokerStructure::Normal(JokerData::Scoring(_)) => {
                JokerEffect::from_scoring(self.trigger_scoring(event, context, joker_count))
            }
            JokerStructure::Normal(JokerData::Econ(data)) => {
                let mut effect = JokerEffect::default();
                match self.kind {
                    JokerKind::BusinessCard
                        if context.modifiers.is_face(card) && chance(context, data.probability) =>
                    {
                        effect.money += data.money as i16;
                    }
                    JokerKind::GoldenTicket if card.enhancement() == Enhancement::Gold => {
                        effect.money += data.money as i16;
                    }
                    JokerKind::RoughGem
                        if context.modifiers.card_has_suit(card, Suit::Diamonds) =>
                    {
                        effect.money += data.money as i16;
                    }
                    _ => {}
                }
                effect
            }
            JokerStructure::Normal(JokerData::Retrigger(data)) => {
                let _ = data;
                JokerEffect {
                    retriggers: self.trigger_retrigger(event, context).retriggers,
                    ..JokerEffect::default()
                }
            }
            JokerStructure::Normal(JokerData::Generate(data))
                if self.kind == JokerKind::EightBall
                    && card.enhancement() != Enhancement::Stone
                    && data.rank == Some(card.rank())
                    && context.consumable_slots > 0
                    && chance(context, data.probability) =>
            {
                generated_effect(data)
            }
            JokerStructure::Normal(JokerData::Generate(_)) => JokerEffect::default(),
            JokerStructure::Copy(_) => JokerEffect::default(),
            _ => JokerEffect::default(),
        }
    }

    fn on_hand_played(
        &mut self,
        event: &TriggerEvent<'_>,
        context: &mut JokerContext<'_>,
    ) -> JokerEffect {
        let TriggerEvent::HandPlayed {
            played_cards,
            scoring_card_indices,
            hand_type,
        } = event
        else {
            return JokerEffect::default();
        };

        let mut effect = JokerEffect::default();
        if self.kind == JokerKind::ToDoList {
            if let JokerStructure::Normal(JokerData::Econ(data)) = &self.structure {
                if data.poker_hand == Some(*hand_type) {
                    effect.money = data.money as i16;
                }
            }
            return effect;
        }
        if self.kind == JokerKind::Vampire {
            if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                for &index in *scoring_card_indices {
                    let card = &played_cards[index];
                    if !card.is_debuffed() && card.enhancement() != Enhancement::None {
                        data.x_mult += 0.1;
                        effect.add_operation(JokerOperation::EnhancePlayedCard {
                            index,
                            enhancement: Enhancement::None,
                        });
                    }
                }
            }
            return effect;
        }
        if self.kind == JokerKind::Space {
            if let JokerStructure::Normal(JokerData::GameState(data)) = &self.structure {
                if chance(context, data.probability) {
                    effect.add_operation(JokerOperation::UpgradeHand {
                        hand: *hand_type,
                        levels: 1,
                    });
                }
            }
            return effect;
        }
        let JokerStructure::Normal(JokerData::Generate(data)) = &self.structure else {
            return effect;
        };
        match self.kind {
            JokerKind::DNA if played_cards.len() == 1 && context.blind.hands_played == 0 => {
                effect.add_operation(JokerOperation::CopyPlayedCard {
                    index: 0,
                    count: data.amount,
                });
            }
            JokerKind::SixthSense
                if played_cards.len() == 1
                    && context.blind.hands_played == 0
                    && played_cards[0].rank() == Rank::Six =>
            {
                effect = generated_effect(data);
            }
            JokerKind::MidasMask => {
                for (index, card) in played_cards.iter().enumerate() {
                    if scoring_card_indices.contains(&index)
                        && !card.is_debuffed()
                        && context.modifiers.is_face(card)
                    {
                        effect.add_operation(JokerOperation::EnhancePlayedCard {
                            index,
                            enhancement: Enhancement::Gold,
                        });
                    }
                }
            }
            _ => {}
        }
        effect
    }

    fn on_held_card(
        &mut self,
        event: &TriggerEvent<'_>,
        context: &mut JokerContext<'_>,
    ) -> JokerEffect {
        let TriggerEvent::HeldCard {
            card,
            held_cards,
            card_index,
        } = event
        else {
            return JokerEffect::default();
        };

        if card.is_debuffed() {
            return JokerEffect::default();
        }

        match &self.structure {
            JokerStructure::Normal(JokerData::Scoring(data)) => {
                let mut scoring = JokerScoringEffect {
                    x_mult: 1.0,
                    ..JokerScoringEffect::default()
                };
                if scoring_condition(
                    self.kind,
                    data,
                    card,
                    &[],
                    held_cards,
                    None,
                    *card_index,
                    &[],
                    0,
                    context,
                    0,
                ) {
                    add_scoring_effect(
                        self.kind,
                        data,
                        card,
                        &[],
                        held_cards,
                        None,
                        context,
                        0,
                        &mut scoring,
                    );
                }
                JokerEffect::from_scoring(scoring)
            }
            JokerStructure::Normal(JokerData::Econ(data))
                if self.kind == JokerKind::ReservedParking
                    && context.modifiers.is_face(card)
                    && chance(context, data.probability) =>
            {
                JokerEffect {
                    money: data.money as i16,
                    ..JokerEffect::default()
                }
            }
            JokerStructure::Normal(JokerData::Retrigger(data))
                if retrigger_matches(
                    data.target,
                    card,
                    *card_index,
                    None,
                    0,
                    context.modifiers,
                ) =>
            {
                JokerEffect {
                    retriggers: data.retrigger,
                    ..JokerEffect::default()
                }
            }
            _ => JokerEffect::default(),
        }
    }

    fn on_discard(
        &mut self,
        cards: &[Card],
        is_discard_action: bool,
        context: &mut JokerContext<'_>,
    ) -> JokerEffect {
        match self.kind {
            JokerKind::MailInRebate => {
                let JokerStructure::Normal(JokerData::Econ(data)) = &self.structure else {
                    return JokerEffect::default();
                };
                let Some(target) = data.rank.as_ref().and_then(|ranks| ranks.first()) else {
                    return JokerEffect::default();
                };
                let matches = cards.iter().filter(|card| card.rank() == *target).count();
                JokerEffect {
                    money: data.money as i16 * matches as i16,
                    ..JokerEffect::default()
                }
            }
            JokerKind::Faceless
                if cards
                    .iter()
                    .filter(|card| !card.is_debuffed() && context.modifiers.is_face(card))
                    .count()
                    >= 3 =>
            {
                JokerEffect {
                    money: 5,
                    ..JokerEffect::default()
                }
            }
            JokerKind::TradingCard
                if is_discard_action && cards.len() == 1 && context.blind.discards_used == 0 =>
            {
                JokerEffect {
                    money: 3,
                    destroy_discarded_card: true,
                    ..JokerEffect::default()
                }
            }
            JokerKind::Castle => {
                let count = context
                    .blind
                    .target_suit
                    .map(|suit| {
                        cards
                            .iter()
                            .filter(|card| context.modifiers.card_has_suit(card, suit))
                            .count()
                    })
                    .unwrap_or(0);
                JokerEffect {
                    chips: count as i32 * 3,
                    ..JokerEffect::default()
                }
            }
            JokerKind::BurntJoker if is_discard_action && context.blind.discards_used == 0 => {
                let hand_type = crate::calculation::poker::determine_poker_hand(
                    &cards.iter().map(Card::id).collect::<Vec<_>>(),
                    context.modifiers,
                )
                .map(|result| result.hand);
                let Some(hand_type) = hand_type else {
                    return JokerEffect::default();
                };
                let JokerStructure::Normal(JokerData::Generate(data)) = &self.structure else {
                    return JokerEffect::default();
                };
                let mut effect = JokerEffect::default();
                effect.add_operation(JokerOperation::UpgradeHand {
                    hand: hand_type,
                    levels: data.amount,
                });
                effect
            }
            _ => JokerEffect::default(),
        }
    }

    fn on_before_played_cards(&mut self, _event: &TriggerEvent<'_>) -> JokerEffect {
        match &self.structure {
            JokerStructure::Normal(JokerData::Generate(data)) => generated_effect(data),
            _ => JokerEffect::default(),
        }
    }

    fn on_after_hand(
        &mut self,
        event: &TriggerEvent<'_>,
        context: &mut JokerContext<'_>,
        joker_count: u8,
    ) -> JokerEffect {
        let TriggerEvent::AfterHand {
            hand_type,
            played_cards,
            scoring_card_indices,
            ..
        } = event
        else {
            return JokerEffect::default();
        };

        match &mut self.structure {
            JokerStructure::Normal(JokerData::Scoring(_)) => JokerEffect::from_scoring(
                self.trigger_after_hand_scoring(event, context, joker_count),
            ),
            JokerStructure::Normal(JokerData::Econ(_)) => JokerEffect::default(),
            JokerStructure::Normal(JokerData::Generate(data)) => {
                let qualifies = context.consumable_slots > 0
                    && match self.kind {
                        JokerKind::Vagabond => *context.money <= 4,
                        JokerKind::Superposition => {
                            (data.poker_hand == *hand_type
                                || (data.poker_hand == Some(PokerHand::Straight)
                                    && *hand_type == Some(PokerHand::StraightFlush)))
                                && scoring_card_indices.iter().any(|&index| {
                                    let card = &played_cards[index];
                                    card.enhancement() != Enhancement::Stone
                                        && data.rank == Some(card.rank())
                                })
                        }
                        JokerKind::Seance => data.poker_hand == *hand_type,
                        _ => false,
                    };
                if qualifies {
                    generated_effect(data)
                } else {
                    JokerEffect::default()
                }
            }
            _ => JokerEffect::default(),
        }
    }

    fn on_boss_blind_ability(&mut self, event: &TriggerEvent<'_>) -> JokerEffect {
        let TriggerEvent::BossBlindAbilityTriggered {
            boss,
            during_hand_scoring: true,
        } = event
        else {
            return JokerEffect::default();
        };
        if self.kind == JokerKind::Matador
            && matches!(
                boss,
                BossBlinds::Club
                    | BossBlinds::Goad
                    | BossBlinds::Window
                    | BossBlinds::Head
                    | BossBlinds::Plante
                    | BossBlinds::Pillar
                    | BossBlinds::Flint
                    | BossBlinds::Eye
                    | BossBlinds::Mouth
                    | BossBlinds::Psychich
                    | BossBlinds::Arm
                    | BossBlinds::Ox
                    | BossBlinds::VerdantLeaf
            )
        {
            JokerEffect {
                money: 8,
                ..JokerEffect::default()
            }
        } else {
            JokerEffect::default()
        }
    }

    fn update(&mut self, event: &UpdateEvent<'_>, context: &mut JokerContext<'_>) -> JokerEffect {
        if self.is_debuffed() {
            return JokerEffect::default();
        }

        // These abilities normally update before scoring, which a whole-hand
        // boss debuff skips. After-play decay (Ice Cream/Seltzer) still runs.
        if matches!(event, UpdateEvent::HandCompleted { scoring_card_indices, .. } if scoring_card_indices.is_empty())
            && matches!(
                self.kind,
                JokerKind::RideTheBus
                    | JokerKind::Runner
                    | JokerKind::SquareJoker
                    | JokerKind::Green
                    | JokerKind::Obelisk
                    | JokerKind::SpareTrousers
            )
        {
            return JokerEffect::default();
        }

        if let (JokerKind::Cloud9, UpdateEvent::RoundCompleted { held_cards }) = (self.kind, event)
        {
            context.owned = OwnedCardCounts::from_cards(
                context
                    .deck
                    .cards()
                    .iter()
                    .chain(context.deck.discarded_cards())
                    .chain(*held_cards),
            );
        }
        let mut effect = JokerEffect::default();
        match (event, self.kind) {
            (UpdateEvent::BlindSelected { .. }, JokerKind::Marble) => {
                effect.add_operation(JokerOperation::GenerateStoneCard { count: 1 });
            }
            (UpdateEvent::BlindSelected { .. }, JokerKind::RiffRaff) => {
                effect.add_operation(JokerOperation::GenerateJoker {
                    rarity: JokerRarity::Common,
                    count: 2,
                });
            }
            (UpdateEvent::RoundStarted, JokerKind::Certificate) => {
                effect.add_operation(JokerOperation::GenerateSealedCard { count: 1 });
            }
            (UpdateEvent::BlindSelected { .. }, JokerKind::Seltzer) => {
                self.active_hands = 10;
            }
            (UpdateEvent::BlindSelected { .. }, JokerKind::CeremonialDagger) => {
                effect.remove_right_joker = true;
            }
            (UpdateEvent::BlindSelected { .. }, JokerKind::Burglar) => {
                effect.hands = 3;
                effect.discards = -(context.blind.discards_remaining as i8);
            }
            (UpdateEvent::BlindSelected { is_boss: false }, JokerKind::Madness) => {
                if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                    data.x_mult += 0.5;
                    effect.x_mult = data.x_mult;
                    effect.remove_random_joker = true;
                }
            }
            (UpdateEvent::CardDestroyed { card }, JokerKind::Glass)
                if card.enhancement() == Enhancement::Glass =>
            {
                if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                    data.x_mult += 0.75;
                    effect.x_mult = data.x_mult;
                }
            }
            (UpdateEvent::CardDestroyed { card }, JokerKind::Canio)
                if !card.is_debuffed() && context.modifiers.is_face(card) =>
            {
                if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                    data.x_mult += 1.0;
                    effect.x_mult = data.x_mult;
                }
            }
            (UpdateEvent::CardsDiscarded { cards }, JokerKind::HitTheRoad) => {
                if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                    data.x_mult += 0.5
                        * cards
                            .iter()
                            .filter(|card| card.rank() == Rank::Jack)
                            .count() as f32;
                    effect.x_mult = data.x_mult;
                }
            }
            (UpdateEvent::CardsDiscarded { cards }, JokerKind::Ramen) => {
                if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                    data.x_mult = (data.x_mult - 0.01 * cards.len() as f32).max(1.0);
                    effect.x_mult = data.x_mult;
                }
            }
            (UpdateEvent::RoundStarted, JokerKind::HitTheRoad) => {
                if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                    data.x_mult = 1.0;
                }
            }
            (
                UpdateEvent::HandCompleted {
                    played_cards,
                    scoring_card_indices,
                    ..
                },
                JokerKind::RideTheBus,
            ) => {
                if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                    if scoring_card_indices.iter().any(|&index| {
                        let card = &played_cards[index];
                        !card.is_debuffed() && context.modifiers.is_face(card)
                    }) {
                        data.add_mult = 0;
                    } else {
                        data.add_mult += 1;
                    }
                    effect.add_mult = data.add_mult as i32;
                }
            }
            (
                UpdateEvent::HandCompleted {
                    hand_type: Some(PokerHand::Straight),
                    ..
                },
                JokerKind::Runner,
            ) => {
                if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                    data.chips += 15;
                    effect.chips = data.chips as i32;
                }
            }
            (UpdateEvent::HandCompleted { played_cards, .. }, JokerKind::SquareJoker)
                if played_cards.len() == 4 =>
            {
                if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                    data.chips += 4;
                    effect.chips = data.chips as i32;
                }
            }
            (UpdateEvent::HandCompleted { .. }, JokerKind::IceCream) => {
                if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                    data.chips = data.chips.saturating_sub(5);
                    effect.chips = data.chips as i32;
                }
            }
            (UpdateEvent::HandCompleted { .. }, JokerKind::Seltzer) => {
                self.active_hands = self.active_hands.saturating_sub(1);
            }
            (UpdateEvent::RoundCompleted { .. }, JokerKind::Popcorn) => {
                if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                    data.add_mult = data.add_mult.saturating_sub(4);
                    effect.add_mult = data.add_mult as i32;
                }
            }
            (UpdateEvent::RoundCompleted { .. }, JokerKind::DelayedGratification)
                if context.blind.discards_used == 0 =>
            {
                if let JokerStructure::Normal(JokerData::Econ(data)) = &self.structure {
                    effect.money = data.money as i16 * context.blind.discards_remaining as i16;
                }
            }
            (UpdateEvent::RoundCompleted { .. }, JokerKind::TurtleBean) => {
                if let JokerStructure::Normal(JokerData::GameState(data)) = &mut self.structure {
                    data.hand_size -= 1;
                    effect.remove_self = data.hand_size == 0;
                }
            }
            (UpdateEvent::RoundCompleted { .. }, JokerKind::Invisible) => {
                self.rounds_completed = self.rounds_completed.saturating_add(1);
            }
            (UpdateEvent::RoundCompleted { .. }, JokerKind::Egg) => {
                self.sell_value_bonus = self.sell_value_bonus.saturating_add(3);
            }
            (UpdateEvent::RoundCompleted { .. }, JokerKind::Cloud9) => {
                effect.money = context.owned.nines as i16;
            }
            (UpdateEvent::RoundCompleted { .. }, JokerKind::GiftCard) => {
                effect.sell_value_bonus = 1;
            }
            (UpdateEvent::RoundCompleted { .. }, JokerKind::ToTheMoon) => {
                effect.money = (*context.money).max(0) / 5;
            }
            (UpdateEvent::RoundCompleted { .. }, JokerKind::Golden) => {
                effect.money = 4;
            }
            (UpdateEvent::RoundCompleted { .. }, JokerKind::Satellite) => {
                effect.money = context.counters.used_planets.count_ones() as i16;
            }
            (UpdateEvent::RoundCompleted { .. }, JokerKind::Rocket) => {
                if let JokerStructure::Normal(JokerData::Econ(data)) = &self.structure {
                    effect.money = data.money as i16;
                }
            }
            (UpdateEvent::RoundCompleted { .. }, JokerKind::GrosMichel)
                if chance(context, 1.0 / 6.0) =>
            {
                effect.remove_self = true;
            }
            (UpdateEvent::RoundCompleted { .. }, JokerKind::Cavendish)
                if chance(context, 1.0 / 1000.0) =>
            {
                effect.remove_self = true;
            }
            (
                UpdateEvent::RoundStarted | UpdateEvent::RoundCompleted { .. },
                JokerKind::Ancient | JokerKind::Castle,
            ) => {
                let suit = *ALL_SUITS.choose(&mut context.rng).unwrap();
                if self.kind == JokerKind::Castle {
                    context.blind.target_suit = Some(suit);
                }
                if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                    if self.kind == JokerKind::Ancient {
                        data.suit = Some(suit);
                    }
                }
            }
            (UpdateEvent::PlanetCardUsed, JokerKind::Constellation) => {
                if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                    data.x_mult += 0.1;
                    effect.x_mult = data.x_mult;
                }
            }
            (UpdateEvent::TarotCardUsed, JokerKind::FortuneTeller) => {
                if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                    data.add_mult = data.add_mult.saturating_add(1);
                    effect.add_mult = data.add_mult as i32;
                }
            }
            (UpdateEvent::BoosterPackOpened, JokerKind::Hallucination) => {
                let should_generate = match &self.structure {
                    JokerStructure::Normal(JokerData::Generate(data)) => {
                        chance(context, data.probability)
                    }
                    _ => false,
                };
                if should_generate {
                    if let JokerStructure::Normal(JokerData::Generate(data)) = &self.structure {
                        effect.operations.extend(generated_effect(data).operations);
                    }
                }
            }
            (UpdateEvent::HandCompleted { .. }, JokerKind::Green) => {
                if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                    data.add_mult += 1;
                    effect.add_mult = data.add_mult as i32;
                }
            }
            (UpdateEvent::DiscardActionCompleted, JokerKind::Green) => {
                if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                    data.add_mult = data.add_mult.saturating_sub(1);
                    effect.add_mult = data.add_mult as i32;
                }
            }
            (UpdateEvent::BoosterPackSkipped, JokerKind::RedCard) => {
                if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                    data.add_mult += 3;
                    effect.add_mult = data.add_mult as i32;
                }
            }
            (UpdateEvent::DeckChanged { cards_added }, JokerKind::Hologram) if *cards_added > 0 => {
                if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                    data.x_mult += 0.25;
                    effect.x_mult = data.x_mult;
                }
            }
            (UpdateEvent::HandCompleted { hand_type, .. }, JokerKind::Obelisk) => {
                if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                    if context.most_played_hand == *hand_type {
                        data.x_mult = 1.0;
                    } else {
                        data.x_mult += 0.2;
                    }
                    effect.x_mult = data.x_mult;
                }
            }
            (UpdateEvent::LuckyCardSucceeded, JokerKind::LuckyCat) => {
                if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                    data.x_mult += 0.25;
                    effect.x_mult = data.x_mult;
                }
            }
            (UpdateEvent::ShopRerolled, JokerKind::FlashCard) => {
                if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                    data.add_mult += 2;
                    effect.add_mult = data.add_mult as i32;
                }
            }
            (UpdateEvent::AfterCardSold, JokerKind::Campfire) => {
                if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                    data.x_mult += 0.25;
                    effect.x_mult = data.x_mult;
                }
            }
            (UpdateEvent::JokerSold { kind }, JokerKind::Luchador) if *kind == self.kind => {
                effect.disable_boss_blind = true;
            }
            (UpdateEvent::JokerSold { kind }, JokerKind::DietCola) if *kind == self.kind => {
                effect.add_operation(JokerOperation::AddDoubleTag { count: 1 });
            }
            (UpdateEvent::JokerSold { kind }, JokerKind::Invisible)
                if *kind == self.kind && self.rounds_completed >= 2 =>
            {
                effect.add_operation(JokerOperation::CopyRandomOwnedJoker { count: 1 });
            }
            (
                UpdateEvent::AfterPlayerDeath {
                    chips_scored,
                    required_chips,
                },
                JokerKind::MrBones,
            ) if !context.blind.prevent_death
                && chips_scored.saturating_mul(4) >= *required_chips =>
            {
                effect.prevent_death = true;
                effect.remove_self = true;
            }
            (UpdateEvent::ShopClosed, JokerKind::Perkeo) => {
                effect.add_operation(JokerOperation::CopyNegativeConsumable { count: 1 });
            }
            (UpdateEvent::BossBlindCompleted, JokerKind::Campfire) => {
                if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                    data.x_mult = 1.0;
                }
            }
            (UpdateEvent::BossBlindCompleted, JokerKind::Rocket) => {
                if let JokerStructure::Normal(JokerData::Econ(data)) = &mut self.structure {
                    data.money = data.money.saturating_add(2);
                }
            }
            (UpdateEvent::CardsDiscarded { cards }, JokerKind::Yorick) => {
                if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                    data.req_count = data.req_count.saturating_add(cards.len() as u8);
                    if data.req_count >= 23 {
                        data.req_count -= 23;
                        data.x_mult += 1.0;
                        effect.x_mult = data.x_mult;
                    }
                }
            }
            (
                UpdateEvent::HandCompleted {
                    hand_type: Some(PokerHand::TwoPair),
                    ..
                },
                JokerKind::SpareTrousers,
            ) => {
                if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                    data.add_mult += 2;
                    effect.add_mult = data.add_mult as i32;
                }
            }
            (UpdateEvent::CardsDiscarded { cards }, JokerKind::Castle) => {
                if let Some(target_suit) = context.blind.target_suit {
                    if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                        data.chips = data.chips.saturating_add(
                            cards
                                .iter()
                                .filter(|card| context.modifiers.card_has_suit(card, target_suit))
                                .count() as u16
                                * 3,
                        );
                        effect.chips = data.chips as i32;
                    }
                }
            }
            (UpdateEvent::BlindCompleted { .. }, JokerKind::Idol) => {
                let rank = *ALL_RANKS.choose(&mut context.rng).unwrap();
                let suit = *ALL_SUITS.choose(&mut context.rng).unwrap();
                if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                    data.rank = Some(vec![rank]);
                    data.suit = Some(suit);
                }
            }
            (UpdateEvent::BlindCompleted { .. }, JokerKind::MailInRebate) => {
                let rank = *ALL_RANKS.choose(&mut context.rng).unwrap();
                if let JokerStructure::Normal(JokerData::Econ(data)) = &mut self.structure {
                    data.rank = Some(vec![rank]);
                }
            }
            (UpdateEvent::BlindCompleted { .. }, JokerKind::ToDoList) => {
                let poker_hand = *PokerHand::DISCARD_HANDS.choose(&mut context.rng).unwrap();
                if let JokerStructure::Normal(JokerData::Econ(data)) = &mut self.structure {
                    data.poker_hand = Some(poker_hand);
                }
            }
            _ => {}
        }
        effect
    }
}

fn add_scoring_effect(
    kind: JokerKind,
    data: &crate::core::joker::JokerScoring,
    card: &Card,
    _played_cards: &[Card],
    held_cards: &[Card],
    hand_type: Option<PokerHand>,
    context: &mut JokerContext<'_>,
    joker_count: u8,
    effect: &mut JokerScoringEffect,
) {
    effect.chips += data.chips as i32;
    effect.add_mult += data.add_mult as i32;
    if data.x_mult > 1.0 {
        effect.x_mult *= data.x_mult;
    }

    match kind {
        JokerKind::Banner => effect.chips += context.blind.discards_remaining as i32 * 30,
        JokerKind::MysticSummit if context.blind.discards_remaining == 0 => effect.add_mult += 15,
        JokerKind::Abstract => effect.add_mult += joker_count.saturating_sub(1) as i32 * 3,
        JokerKind::Stencil => {
            effect.x_mult *= 1.0 + context.empty_joker_slots() as f32;
        }
        JokerKind::Steel => {
            let steel_cards = context.owned.steel;
            effect.x_mult *= 1.0 + steel_cards as f32 * 0.2;
        }
        JokerKind::RaisedFist => {
            if let Some(lowest) = held_cards
                .iter()
                .min_by_key(|held| held.rank().base_chips())
            {
                if lowest.rank() == card.rank() {
                    effect.add_mult += lowest.rank().base_chips() as i32 * 2;
                }
            }
        }
        JokerKind::Bootstraps => effect.add_mult += ((*context.money).max(0) / 5) as i32 * 2,
        JokerKind::Bull => effect.chips += (*context.money).max(0) as i32 * 2,
        JokerKind::Blue => effect.chips += context.deck.size() as i32 * 2,
        JokerKind::Stone => effect.chips += context.owned.stone as i32 * 25,
        JokerKind::Erosion => {
            effect.add_mult += context
                .deck
                .starting_size()
                .saturating_sub(context.owned.count as u16) as i32
                * 4;
        }
        JokerKind::Throwback => {
            effect.x_mult *= 1.0 + context.counters.blinds_skipped as f32 * 0.25
        }
        JokerKind::Supernova => {
            if context.blind.current_hand == hand_type {
                effect.add_mult += 1;
            }
        }
        JokerKind::Misprint if !context.rng.random_bool(0.5) => {
            effect.chips = 0;
            effect.add_mult = 0;
            effect.x_mult = 1.0;
        }
        JokerKind::Bloodstone if !chance(context, data.probability) => {
            effect.x_mult = 1.0;
        }
        _ => {}
    }
}

fn generated_effect(data: &crate::core::joker::JokerGenerate) -> JokerEffect {
    let mut effect = JokerEffect::default();
    let operation = match data.generate_type {
        GenerateType::None => None,
        GenerateType::StoneCard => Some(JokerOperation::GenerateStoneCard { count: data.amount }),
        GenerateType::Tarot => Some(JokerOperation::GenerateConsumable {
            kind: GeneratedConsumable::Tarot,
            count: data.amount,
        }),
        GenerateType::Spectral => Some(JokerOperation::GenerateConsumable {
            kind: GeneratedConsumable::Spectral,
            count: data.amount,
        }),
        GenerateType::SealCard => Some(JokerOperation::GenerateSealedCard { count: data.amount }),
        GenerateType::FirstPlayedCard | GenerateType::Gold | GenerateType::PlanetLevel => None,
        GenerateType::Joker => Some(JokerOperation::GenerateJoker {
            rarity: JokerRarity::Common,
            count: data.amount,
        }),
        GenerateType::NegativeConsumable => {
            Some(JokerOperation::CopyNegativeConsumable { count: data.amount })
        }
    };
    if let Some(operation) = operation {
        effect.add_operation(operation);
    }
    effect
}

fn chance(context: &mut JokerContext<'_>, probability: f32) -> bool {
    let probability = probability * context.modifiers.probability_multiplier;
    if probability >= 1.0 {
        return true;
    }
    if probability <= 0.0 {
        return false;
    }
    context.rng.random_bool(probability as f64)
}

fn retrigger_matches(
    target: RetriggerTarget,
    card: &Card,
    card_index: usize,
    first_scoring_card: Option<usize>,
    hands_remaining: u8,
    modifiers: JokerModifiers,
) -> bool {
    match target {
        RetriggerTarget::FaceCards => modifiers.is_face(card),
        RetriggerTarget::LowCards => card.is_low_card(),
        RetriggerTarget::FinalHand => hands_remaining == 1,
        RetriggerTarget::FirstCard => Some(card_index) == first_scoring_card,
        RetriggerTarget::PlayedCards | RetriggerTarget::HeldCards => true,
    }
}

fn scoring_condition(
    kind: JokerKind,
    data: &crate::core::joker::JokerScoring,
    card: &Card,
    played_cards: &[Card],
    held_cards: &[Card],
    hand_type: Option<PokerHand>,
    card_index: usize,
    scoring_card_indices: &[usize],
    _hands_remaining: u8,
    context: &JokerContext<'_>,
    _joker_count: u8,
) -> bool {
    if let Some(rank) = &data.rank {
        if card.enhancement() == Enhancement::Stone || !rank.contains(&card.rank()) {
            return false;
        }
    }
    if let Some(suit) = data.suit {
        if !context.modifiers.card_has_suit(card, suit) {
            return false;
        }
    }
    if let Some(required_hand) = data.poker_hand {
        let mut ranks = [0u8; 15];
        for card in played_cards
            .iter()
            .filter(|card| card.enhancement() != Enhancement::Stone)
        {
            ranks[card.rank() as usize] += 1;
        }
        let contains = match required_hand {
            PokerHand::Pair => ranks.iter().any(|&count| count >= 2),
            PokerHand::TwoPair => ranks.iter().filter(|&&count| count >= 2).count() >= 2,
            PokerHand::ThreeOfAKind => ranks.iter().any(|&count| count >= 3),
            PokerHand::FourOfAKind => ranks.iter().any(|&count| count >= 4),
            PokerHand::Straight => {
                let mask = ranks.iter().enumerate().fold(0u16, |mask, (rank, &count)| {
                    if count > 0 { mask | (1 << rank) } else { mask }
                });
                crate::calculation::poker::straight_ranks(
                    mask,
                    context.modifiers.straight_gaps,
                    if context.modifiers.four_fingers { 4 } else { 5 },
                )
                .is_some()
            }
            PokerHand::Flush => ALL_SUITS.iter().copied().any(|suit| {
                played_cards
                    .iter()
                    .filter(|card| context.modifiers.card_has_flush_suit(card, suit))
                    .count()
                    >= if context.modifiers.four_fingers { 4 } else { 5 }
            }),
            _ => hand_type == Some(required_hand),
        };
        if !contains {
            return false;
        }
    }

    match kind {
        JokerKind::Half => played_cards.len() <= 3,
        JokerKind::ScaryFace | JokerKind::SmileyFace => context.modifiers.is_face(card),
        JokerKind::Steel
        | JokerKind::Banner
        | JokerKind::Abstract
        | JokerKind::RaisedFist
        | JokerKind::Blue
        | JokerKind::Hiker
        | JokerKind::Erosion
        | JokerKind::Throwback
        | JokerKind::Bootstraps
        | JokerKind::Bull
        | JokerKind::Supernova
        | JokerKind::GrosMichel
        | JokerKind::Misprint => true,
        JokerKind::MysticSummit => context.blind.discards_remaining == 0,
        JokerKind::Loyalty => context.blind.hands_played > 0 && context.blind.hands_played % 6 == 0,
        JokerKind::Fibonacci => {
            card.enhancement() != Enhancement::Stone
                && matches!(
                    card.rank(),
                    Rank::Ace | Rank::Two | Rank::Three | Rank::Five | Rank::Eight
                )
        }
        JokerKind::FlowerPot => {
            fn covers_suits(
                cards: &[Card],
                indices: &[usize],
                suit: usize,
                used: u8,
                modifiers: JokerModifiers,
            ) -> bool {
                if suit == ALL_SUITS.len() {
                    return true;
                }
                indices.iter().copied().any(|index| {
                    let card = &cards[index];
                    used & (1 << index) == 0
                        && (if card.enhancement() == Enhancement::Wild {
                            modifiers.card_has_suit(card, ALL_SUITS[suit])
                        } else {
                            modifiers.card_has_flush_suit(card, ALL_SUITS[suit])
                        })
                        && covers_suits(cards, indices, suit + 1, used | (1 << index), modifiers)
                })
            }
            covers_suits(played_cards, scoring_card_indices, 0, 0, context.modifiers)
        }
        JokerKind::Blackboard => held_cards.iter().all(|held| {
            context.modifiers.card_has_flush_suit(held, Suit::Clubs)
                || context.modifiers.card_has_flush_suit(held, Suit::Spades)
        }),
        JokerKind::Photograph => {
            scoring_card_indices
                .iter()
                .copied()
                .find(|&index| context.modifiers.is_face(&played_cards[index]))
                == Some(card_index)
        }
        JokerKind::SeeingDouble => scoring_card_indices.iter().copied().any(|index| {
            context
                .modifiers
                .card_has_flush_suit(&played_cards[index], Suit::Clubs)
                && scoring_card_indices.iter().copied().any(|other| {
                    other != index
                        && ALL_SUITS.iter().copied().any(|suit| {
                            !context.modifiers.suits_match(suit, Suit::Clubs)
                                && context
                                    .modifiers
                                    .card_has_flush_suit(&played_cards[other], suit)
                        })
                })
        }),
        JokerKind::SpareTrousers => hand_type == Some(PokerHand::TwoPair),
        JokerKind::DriversLicense => context.owned.enhanced >= 16,
        JokerKind::Obelisk => context.most_played_hand != hand_type,
        JokerKind::CardSharp => {
            hand_type.is_some_and(|hand| context.blind.played_hand_types.contains(&hand))
        }
        _ => true,
    }
}
