use rand::prelude::{IndexedRandom, IteratorRandom, RngExt};

use crate::core::card::Card;
use crate::core::enums::{ALL_RANKS, ALL_SUITS, Enhancement, PokerHand, Rank, Suit};
use crate::core::game::GameState;
use crate::core::joker::{Joker, JokerData, JokerStructure};
use crate::core::joker_types::JokerUpdate::*;
use crate::core::joker_types::{
    GenerateType, JokerKind, JokerTrigger, JokerUpdate, RetriggerTarget,
};

/// One-shot events. All payloads borrow the current game objects so dispatch
/// does not clone cards or hands.
pub enum TriggerEvent<'a> {
    PlayedCard {
        card: &'a Card,
        played_cards: &'a [Card],
        held_cards: &'a [Card],
        card_index: usize,
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
    },
    BeforePlayedCards {
        played_cards: &'a [Card],
        held_cards: &'a [Card],
    },
    AfterHand {
        played_cards: &'a [Card],
        held_cards: &'a [Card],
        hand_type: Option<PokerHand>,
        hands_remaining: u8,
        discards_remaining: u8,
        joker_count: u8,
    },
    BossBlindAbilityTriggered,
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
    RoundCompleted,
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
            Self::RoundCompleted => JokerUpdate::RoundCompleted,
            Self::DiscardActionCompleted => JokerUpdate::DiscardActionCompleted,
            Self::LuckyCardSucceeded => JokerUpdate::LuckyCardSucceeded,
            Self::ShopRerolled => JokerUpdate::ShopRerolled,
            Self::BossBlindCompleted => JokerUpdate::BossBlindCompleted,
            Self::ShopClosed => JokerUpdate::ShopClosed,
        }
    }
}

/// Fixed-size output for a trigger/update pass. It is intentionally returned
/// by value so the Python layer can encode it without allocating a result list.
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
    pub generated: u8,
    pub generated_type: Option<GenerateType>,
    pub remove_rightmost_joker: bool,
    pub remove_random_joker: bool,
    pub remove_self: bool,
    pub disable_boss_blind: bool,
    pub prevent_death: bool,
    pub sell_value_bonus: u8,
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
            generated: 0,
            generated_type: None,
            remove_rightmost_joker: false,
            remove_random_joker: false,
            remove_self: false,
            disable_boss_blind: false,
            prevent_death: false,
            sell_value_bonus: 0,
        }
    }
}

impl JokerEffect {
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
        self.generated += other.generated;
        if self.generated_type.is_none() {
            self.generated_type = other.generated_type;
        }
        self.remove_rightmost_joker |= other.remove_rightmost_joker;
        self.remove_random_joker |= other.remove_random_joker;
        self.remove_self |= other.remove_self;
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

    pub(crate) fn capacity(&self) -> usize {
        self.capacity
    }

    pub(crate) fn increase_capacity(&mut self, amount: usize) {
        self.capacity = self.capacity.saturating_add(amount);
    }

    pub(crate) fn has_room(&self) -> bool {
        self.jokers.len() < self.capacity
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

    pub(crate) fn replace_with(&mut self, joker: Joker) {
        self.jokers.clear();
        self.jokers.push(joker);
    }

    pub(crate) fn keep_only(&mut self, index: usize) -> bool {
        if index >= self.jokers.len() {
            return false;
        }

        let joker = self.jokers[index].clone();
        self.jokers.clear();
        self.jokers.push(joker);
        true
    }

    pub(crate) fn total_sell_value(&self) -> u16 {
        self.jokers
            .iter()
            .map(|joker| joker.sell_value() as u16)
            .sum()
    }

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

    pub fn trigger(&mut self, event: TriggerEvent<'_>, game_state: &mut GameState) -> JokerEffect {
        let mut effect = JokerEffect::default();
        let joker_count = self.jokers.len() as u8;
        for joker in &mut self.jokers {
            effect.merge(joker.trigger(&event, game_state, joker_count));
        }
        apply_game_effect(&effect, game_state);
        effect
    }

    pub fn update(&mut self, event: UpdateEvent<'_>, game_state: &mut GameState) -> JokerEffect {
        let event_kind = event.kind();
        let mut effect = JokerEffect::default();
        let mut remove_self = Vec::new();

        for (index, joker) in self.jokers.iter_mut().enumerate() {
            if joker.update_mask & event_kind.mask() == 0 {
                continue;
            }
            let joker_effect = joker.update(&event, game_state);
            if joker_effect.remove_self {
                remove_self.push(index);
            }
            effect.merge(joker_effect);
        }

        // Collection mutations happen after dispatch, so left-to-right order
        // is stable and the iterator is never invalidated.
        if effect.remove_rightmost_joker && self.jokers.len() > 1 {
            self.jokers.pop();
        }
        if effect.remove_random_joker && self.jokers.len() > 1 {
            let index = (0..self.jokers.len()).choose(&mut game_state.rng).unwrap();
            self.jokers.remove(index);
        }
        for index in remove_self.into_iter().rev() {
            if index < self.jokers.len() {
                self.jokers.remove(index);
            }
        }
        if effect.sell_value_bonus > 0 {
            for joker in &mut self.jokers {
                joker.sell_value_bonus = joker
                    .sell_value_bonus
                    .saturating_add(effect.sell_value_bonus);
            }
            game_state.counters.sell_value_bonus = game_state
                .counters
                .sell_value_bonus
                .saturating_add(effect.sell_value_bonus);
        }

        apply_game_effect(&effect, game_state);
        effect
    }

    pub fn sell(&mut self, index: usize, game_state: &mut GameState) -> JokerEffect {
        let kind = self.jokers[index].kind;
        let effect = self.update(UpdateEvent::JokerSold { kind }, game_state);
        self.jokers.remove(index);
        game_state.counters.cards_sold = game_state.counters.cards_sold.saturating_add(1);
        effect
    }
}

fn apply_game_effect(effect: &JokerEffect, game_state: &mut GameState) {
    game_state.money += effect.money;
    game_state.counters.life += effect.life;
    game_state.blind.boss_ability_disabled |= effect.disable_boss_blind;
    game_state.blind.prevent_death |= effect.prevent_death;
    game_state.deck.adjust_hand_size(effect.hand_size);
    if effect.hands >= 0 {
        game_state.blind.hands_remaining = game_state
            .blind
            .hands_remaining
            .saturating_add(effect.hands as u8);
    } else {
        game_state.blind.hands_remaining = game_state
            .blind
            .hands_remaining
            .saturating_sub(effect.hands.unsigned_abs());
    }
    if effect.discards >= 0 {
        game_state.blind.discards_remaining = game_state
            .blind
            .discards_remaining
            .saturating_add(effect.discards as u8);
    } else {
        game_state.blind.discards_remaining = game_state
            .blind
            .discards_remaining
            .saturating_sub(effect.discards.unsigned_abs());
    }
}

impl Joker {
    pub(super) fn update_mask(joker_kind: JokerKind) -> u32 {
        let mut mask = 0;
        match joker_kind {
            JokerKind::Glass | JokerKind::Canio => mask |= CardDestroyed.mask(),
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

    fn trigger(
        &mut self,
        event: &TriggerEvent<'_>,
        game_state: &mut GameState,
        joker_count: u8,
    ) -> JokerEffect {
        if self.debuffed {
            return JokerEffect::default();
        }

        match (event, self.trigger) {
            (TriggerEvent::PlayedCard { .. }, JokerTrigger::OnPlayedCard) => {
                self.on_played_card(event, game_state, joker_count)
            }
            (TriggerEvent::HeldCard { .. }, JokerTrigger::OnHeldCard) => {
                self.on_held_card(event, game_state)
            }
            (TriggerEvent::Discard { cards }, JokerTrigger::OnDiscard) => {
                self.on_discard(cards, game_state)
            }
            (TriggerEvent::Discard { cards }, _)
                if matches!(self.kind, JokerKind::Faceless | JokerKind::TradingCard) =>
            {
                self.on_discard(cards, game_state)
            }
            (TriggerEvent::BeforePlayedCards { .. }, JokerTrigger::BeforePlayedCards) => {
                self.on_before_played_cards(event)
            }
            (TriggerEvent::AfterHand { .. }, JokerTrigger::AfterHand) => {
                self.on_after_hand(event, game_state, joker_count)
            }
            (TriggerEvent::BossBlindAbilityTriggered, JokerTrigger::OnBossBlindAbility) => {
                self.on_boss_blind_ability()
            }
            _ => JokerEffect::default(),
        }
    }

    fn on_played_card(
        &mut self,
        event: &TriggerEvent<'_>,
        game_state: &mut GameState,
        joker_count: u8,
    ) -> JokerEffect {
        let TriggerEvent::PlayedCard {
            card,
            played_cards,
            held_cards,
            card_index,
            hand_type,
            hands_remaining,
        } = event
        else {
            return JokerEffect::default();
        };

        match &self.structure {
            JokerStructure::Normal(JokerData::Scoring(data)) => {
                let mut effect = JokerEffect::default();
                if scoring_condition(
                    self.kind,
                    data,
                    card,
                    played_cards,
                    held_cards,
                    *hand_type,
                    *card_index,
                    *hands_remaining,
                    game_state,
                    joker_count,
                ) {
                    add_scoring_effect(
                        self.kind,
                        data,
                        card,
                        played_cards,
                        held_cards,
                        *hand_type,
                        game_state,
                        joker_count,
                        &mut effect,
                    );
                }
                effect
            }
            JokerStructure::Normal(JokerData::Econ(data)) => {
                let mut effect = JokerEffect::default();
                match self.kind {
                    JokerKind::BusinessCard
                        if card.is_face_card() && chance(game_state, data.probability) =>
                    {
                        effect.money += data.money as i16;
                    }
                    JokerKind::GoldenTicket if card.enhancement() == Enhancement::Gold => {
                        effect.money += data.money as i16;
                    }
                    JokerKind::RoughGem if card.suit() == Suit::Diamonds => {
                        effect.money += data.money as i16;
                    }
                    _ => {}
                }
                effect
            }
            JokerStructure::Normal(JokerData::Retrigger(data)) => {
                if self.kind == JokerKind::Seltzer && self.active_hands == 0 {
                    return JokerEffect::default();
                }
                if retrigger_matches(data.target, card, *card_index, *hands_remaining) {
                    JokerEffect {
                        retriggers: data.retrigger,
                        ..JokerEffect::default()
                    }
                } else {
                    JokerEffect::default()
                }
            }
            JokerStructure::Normal(JokerData::Generate(data)) => {
                if matches!(self.kind, JokerKind::DNA | JokerKind::SixthSense) && *card_index == 0 {
                    generated_effect(data)
                } else {
                    JokerEffect::default()
                }
            }
            JokerStructure::Copy(_) => JokerEffect::default(),
            _ => JokerEffect::default(),
        }
    }

    fn on_held_card(
        &mut self,
        event: &TriggerEvent<'_>,
        game_state: &mut GameState,
    ) -> JokerEffect {
        let TriggerEvent::HeldCard {
            card,
            held_cards,
            card_index,
        } = event
        else {
            return JokerEffect::default();
        };

        match &self.structure {
            JokerStructure::Normal(JokerData::Scoring(data)) => {
                let mut effect = JokerEffect::default();
                if scoring_condition(
                    self.kind,
                    data,
                    card,
                    &[],
                    held_cards,
                    None,
                    *card_index,
                    0,
                    game_state,
                    0,
                ) {
                    add_scoring_effect(
                        self.kind,
                        data,
                        card,
                        &[],
                        held_cards,
                        None,
                        game_state,
                        0,
                        &mut effect,
                    );
                }
                effect
            }
            JokerStructure::Normal(JokerData::Econ(data))
                if self.kind == JokerKind::ReservedParking
                    && card.is_face_card()
                    && chance(game_state, data.probability) =>
            {
                JokerEffect {
                    money: data.money as i16,
                    ..JokerEffect::default()
                }
            }
            JokerStructure::Normal(JokerData::Retrigger(data))
                if retrigger_matches(data.target, card, *card_index, 0) =>
            {
                JokerEffect {
                    retriggers: data.retrigger,
                    ..JokerEffect::default()
                }
            }
            _ => JokerEffect::default(),
        }
    }

    fn on_discard(&mut self, cards: &[Card], game_state: &mut GameState) -> JokerEffect {
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
            JokerKind::Faceless if cards.iter().filter(|card| card.is_face_card()).count() >= 3 => {
                JokerEffect {
                    money: 5,
                    ..JokerEffect::default()
                }
            }
            JokerKind::TradingCard if !cards.is_empty() => JokerEffect {
                money: 3,
                ..JokerEffect::default()
            },
            JokerKind::Castle => {
                let count = game_state
                    .blind
                    .target_suit
                    .map(|suit| cards.iter().filter(|card| card.suit() == suit).count())
                    .unwrap_or(0);
                JokerEffect {
                    chips: count as i32 * 3,
                    ..JokerEffect::default()
                }
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
        game_state: &mut GameState,
        joker_count: u8,
    ) -> JokerEffect {
        let TriggerEvent::AfterHand {
            played_cards,
            held_cards,
            hand_type,
            hands_remaining,
            discards_remaining,
            ..
        } = event
        else {
            return JokerEffect::default();
        };

        let fallback_card = played_cards.first().or_else(|| held_cards.first());
        let Some(card) = fallback_card else {
            return JokerEffect::default();
        };

        match &mut self.structure {
            JokerStructure::Normal(JokerData::Scoring(data)) => {
                let mut effect = JokerEffect::default();
                if scoring_condition(
                    self.kind,
                    data,
                    card,
                    played_cards,
                    held_cards,
                    *hand_type,
                    0,
                    *hands_remaining,
                    game_state,
                    joker_count,
                ) {
                    add_scoring_effect(
                        self.kind,
                        data,
                        card,
                        played_cards,
                        held_cards,
                        *hand_type,
                        game_state,
                        joker_count,
                        &mut effect,
                    );
                }
                effect
            }
            JokerStructure::Normal(JokerData::Econ(data)) => match self.kind {
                JokerKind::DelayedGratification if game_state.blind.discards_used == 0 => {
                    JokerEffect {
                        money: data.money as i16,
                        ..JokerEffect::default()
                    }
                }
                JokerKind::ToDoList if data.poker_hand == *hand_type => JokerEffect {
                    money: data.money as i16,
                    ..JokerEffect::default()
                },
                _ => JokerEffect::default(),
            },
            JokerStructure::Normal(JokerData::Generate(data)) => {
                if self.kind == JokerKind::Vagabond && game_state.money <= 4 {
                    generated_effect(data)
                } else {
                    JokerEffect::default()
                }
            }
            _ => {
                let _ = discards_remaining;
                JokerEffect::default()
            }
        }
    }

    fn on_boss_blind_ability(&mut self) -> JokerEffect {
        if self.kind == JokerKind::Matador {
            JokerEffect {
                money: 8,
                ..JokerEffect::default()
            }
        } else {
            JokerEffect::default()
        }
    }

    fn update(&mut self, event: &UpdateEvent<'_>, game_state: &mut GameState) -> JokerEffect {
        if self.debuffed {
            return JokerEffect::default();
        }

        let mut effect = JokerEffect::default();
        match (event, self.kind) {
            (UpdateEvent::BlindSelected { .. }, JokerKind::Marble) => {
                effect.generated = 1;
                effect.generated_type = Some(GenerateType::StoneCard);
            }
            (UpdateEvent::BlindSelected { .. }, JokerKind::RiffRaff) => {
                effect.generated = 2;
                effect.generated_type = Some(GenerateType::Joker);
            }
            (UpdateEvent::RoundStarted, JokerKind::Certificate) => {
                effect.generated = 1;
                effect.generated_type = Some(GenerateType::SealCard);
            }
            (UpdateEvent::BlindSelected { .. }, JokerKind::Seltzer) => {
                self.active_hands = 10;
            }
            (UpdateEvent::BlindSelected { .. }, JokerKind::CeremonialDagger) => {
                effect.add_mult += 2;
                effect.remove_rightmost_joker = true;
            }
            (UpdateEvent::BlindSelected { .. }, JokerKind::Burglar) => {
                effect.hands = 3;
                effect.discards = -4;
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
            (UpdateEvent::CardDestroyed { card }, JokerKind::Canio) if card.is_face_card() => {
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
            (UpdateEvent::HandCompleted { played_cards, .. }, JokerKind::RideTheBus) => {
                if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                    if played_cards.iter().any(Card::is_face_card) {
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
            (UpdateEvent::RoundCompleted, JokerKind::Popcorn) => {
                if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                    data.add_mult = data.add_mult.saturating_sub(4);
                    effect.add_mult = data.add_mult as i32;
                }
            }
            (UpdateEvent::RoundCompleted, JokerKind::TurtleBean) => effect.hand_size = -1,
            (UpdateEvent::RoundCompleted, JokerKind::Invisible) => {
                self.rounds_completed = self.rounds_completed.saturating_add(1);
            }
            (UpdateEvent::RoundCompleted, JokerKind::Egg) => {
                self.sell_value_bonus = self.sell_value_bonus.saturating_add(3);
            }
            (UpdateEvent::RoundCompleted, JokerKind::Cloud9) => {
                effect.money = game_state.nines_in_deck() as i16;
            }
            (UpdateEvent::RoundCompleted, JokerKind::GiftCard) => {
                effect.sell_value_bonus = 1;
            }
            (UpdateEvent::RoundCompleted, JokerKind::ToTheMoon) => {
                effect.money = game_state.money.max(0) / 5;
            }
            (UpdateEvent::RoundCompleted, JokerKind::Golden) => {
                effect.money = 4;
            }
            (UpdateEvent::RoundCompleted, JokerKind::Satellite) => {
                effect.money = game_state.unique_planet_cards_used() as i16;
            }
            (UpdateEvent::RoundCompleted, JokerKind::Rocket) => {
                if let JokerStructure::Normal(JokerData::Econ(data)) = &self.structure {
                    effect.money = data.money as i16;
                }
            }
            (UpdateEvent::RoundCompleted, JokerKind::GrosMichel)
                if chance(game_state, 1.0 / 6.0) =>
            {
                effect.remove_self = true;
            }
            (UpdateEvent::RoundCompleted, JokerKind::Cavendish)
                if chance(game_state, 1.0 / 1000.0) =>
            {
                effect.remove_self = true;
            }
            (
                UpdateEvent::RoundStarted | UpdateEvent::RoundCompleted,
                JokerKind::Ancient | JokerKind::Castle,
            ) => {
                let suit = *ALL_SUITS.choose(&mut game_state.rng).unwrap();
                if self.kind == JokerKind::Castle {
                    game_state.blind.target_suit = Some(suit);
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
                        chance(game_state, data.probability)
                    }
                    _ => false,
                };
                if should_generate {
                    if let JokerStructure::Normal(JokerData::Generate(data)) = &self.structure {
                        effect.generated = data.amount;
                        effect.generated_type = Some(data.generate_type);
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
                    if game_state.most_played_hand == *hand_type {
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
                effect.generated = 1;
                effect.generated_type = Some(GenerateType::DoubleTag);
            }
            (UpdateEvent::JokerSold { kind }, JokerKind::Invisible)
                if *kind == self.kind && self.rounds_completed >= 2 =>
            {
                effect.generated = 1;
                effect.generated_type = Some(GenerateType::Joker);
            }
            (
                UpdateEvent::AfterPlayerDeath {
                    chips_scored,
                    required_chips,
                },
                JokerKind::MrBones,
            ) if chips_scored.saturating_mul(4) >= *required_chips => {
                effect.prevent_death = true;
                effect.remove_self = true;
            }
            (UpdateEvent::ShopClosed, JokerKind::Perkeo) => {
                effect.generated = 1;
                effect.generated_type = Some(GenerateType::NegativeConsumable);
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
                if let Some(target_suit) = game_state.blind.target_suit {
                    if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                        data.chips = data.chips.saturating_add(
                            cards
                                .iter()
                                .filter(|card| card.suit() == target_suit)
                                .count() as u16
                                * 3,
                        );
                        effect.chips = data.chips as i32;
                    }
                }
            }
            (UpdateEvent::BlindCompleted { .. }, JokerKind::Idol) => {
                let rank = *ALL_RANKS.choose(&mut game_state.rng).unwrap();
                let suit = *ALL_SUITS.choose(&mut game_state.rng).unwrap();
                if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                    data.rank = Some(vec![rank]);
                    data.suit = Some(suit);
                }
            }
            (UpdateEvent::BlindCompleted { .. }, JokerKind::MailInRebate) => {
                let rank = *ALL_RANKS.choose(&mut game_state.rng).unwrap();
                if let JokerStructure::Normal(JokerData::Econ(data)) = &mut self.structure {
                    data.rank = Some(vec![rank]);
                }
            }
            (UpdateEvent::BlindCompleted { .. }, JokerKind::ToDoList) => {
                let poker_hand = *PokerHand::DISCARD_HANDS
                    .choose(&mut game_state.rng)
                    .unwrap();
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
    game_state: &mut GameState,
    joker_count: u8,
    effect: &mut JokerEffect,
) {
    effect.chips += data.chips as i32;
    effect.add_mult += data.add_mult as i32;
    if data.x_mult > 1.0 {
        effect.x_mult *= data.x_mult;
    }

    match kind {
        JokerKind::Banner => effect.chips += game_state.blind.discards_remaining as i32 * 30,
        JokerKind::MysticSummit if game_state.blind.discards_remaining == 0 => {
            effect.add_mult += 15
        }
        JokerKind::Abstract => effect.add_mult += joker_count.saturating_sub(1) as i32 * 3,
        JokerKind::Stencil => {
            effect.x_mult *= 1.0 + game_state.empty_joker_slots() as f32;
        }
        JokerKind::Steel => {
            let steel_cards = held_cards
                .iter()
                .filter(|held| held.enhancement() == Enhancement::Steel)
                .count();
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
        JokerKind::Bootstraps => effect.add_mult += (game_state.money.max(0) / 5) as i32 * 2,
        JokerKind::Bull => effect.chips += game_state.money.max(0) as i32 * 2,
        JokerKind::Blue => effect.chips += game_state.deck_size() as i32 / 2,
        JokerKind::Erosion => {
            effect.add_mult += game_state
                .starting_deck_size()
                .saturating_sub(game_state.deck_size()) as i32
                * 4;
        }
        JokerKind::Throwback => {
            effect.x_mult *= 1.0 + game_state.counters.blinds_skipped as f32 * 0.25
        }
        JokerKind::Hiker => effect.chips += 5,
        JokerKind::Supernova => {
            if game_state.blind.current_hand == hand_type {
                effect.add_mult += 1;
            }
        }
        JokerKind::Misprint if !chance(game_state, 0.5) => {
            effect.chips = 0;
            effect.add_mult = 0;
            effect.x_mult = 1.0;
        }
        JokerKind::Bloodstone if !chance(game_state, data.probability) => {
            effect.x_mult = 1.0;
        }
        _ => {}
    }
}

fn generated_effect(data: &crate::core::joker::JokerGenerate) -> JokerEffect {
    JokerEffect {
        generated: data.amount,
        generated_type: Some(data.generate_type),
        ..JokerEffect::default()
    }
}

fn chance(game_state: &mut GameState, probability: f32) -> bool {
    if probability >= 1.0 {
        return true;
    }
    if probability <= 0.0 {
        return false;
    }
    game_state.rng.random_bool(probability as f64)
}

fn retrigger_matches(
    target: RetriggerTarget,
    card: &Card,
    card_index: usize,
    hands_remaining: u8,
) -> bool {
    match target {
        RetriggerTarget::FaceCards => card.is_face_card(),
        RetriggerTarget::LowCards => card.is_low_card(),
        RetriggerTarget::FinalHand => hands_remaining == 1,
        RetriggerTarget::FirstCard => card_index == 0,
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
    _hands_remaining: u8,
    game_state: &GameState,
    _joker_count: u8,
) -> bool {
    if let Some(rank) = &data.rank {
        if !rank.contains(&card.rank()) {
            return false;
        }
    }
    if let Some(suit) = data.suit {
        if card.suit() != suit {
            return false;
        }
    }
    if let Some(required_hand) = data.poker_hand {
        if hand_type != Some(required_hand) {
            return false;
        }
    }

    match kind {
        JokerKind::Half => played_cards.len() <= 3,
        JokerKind::ScaryFace | JokerKind::SmileyFace => card.is_face_card(),
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
        JokerKind::MysticSummit => game_state.blind.discards_remaining == 0,
        JokerKind::Loyalty => {
            game_state.blind.hands_played > 0 && game_state.blind.hands_played % 6 == 0
        }
        JokerKind::Fibonacci => matches!(
            card.rank(),
            Rank::Ace | Rank::Two | Rank::Three | Rank::Five | Rank::Eight
        ),
        JokerKind::FlowerPot => {
            let mut suits = [false; 4];
            for played in played_cards {
                suits[played.suit() as usize] = true;
            }
            suits.iter().all(|present| *present)
        }
        JokerKind::Blackboard => held_cards
            .iter()
            .all(|held| matches!(held.suit(), Suit::Clubs | Suit::Spades)),
        JokerKind::Photograph => card_index == 0 && card.is_face_card(),
        JokerKind::SeeingDouble => {
            played_cards
                .iter()
                .any(|played| played.suit() == Suit::Clubs)
                && played_cards
                    .iter()
                    .any(|played| played.suit() != Suit::Clubs)
        }
        JokerKind::SpareTrousers => hand_type == Some(PokerHand::TwoPair),
        JokerKind::DriversLicense => game_state.enhanced_cards() >= 16,
        JokerKind::Obelisk => game_state.most_played_hand != hand_type,
        JokerKind::CardSharp => game_state.blind.current_hand == hand_type,
        _ => true,
    }
}
