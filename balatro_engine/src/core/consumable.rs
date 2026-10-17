use crate::core::card::Card;
use crate::core::enums::{
    ALL_RANKS, ALL_SUITS, Consumable, Edition, Enhancement, Planet, PokerHand, Rank, Seal,
    Spectral, Suit, Tarot,
};
use crate::core::enums::{Spectral::*, Tarot::*};
use crate::core::game::GameState;
use crate::core::hand::Hand;
use crate::core::joker::{Joker, UpdateEvent};
use crate::core::joker_types::{JokerEdition, JokerRarity};
use chacha20::ChaCha12Rng;
use rand::prelude::{IndexedRandom, IteratorRandom, RngExt};

pub(crate) const DEFAULT_CONSUMABLE_SLOTS: usize = 2;
pub(crate) const DEFAULT_CONSUMABLE_PRICE: u16 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OwnedConsumable {
    pub(crate) consumable: Consumable,
    pub(crate) original_price: u16,
    pub(crate) current_price: u16,
    pub(crate) negative: bool,
}

impl OwnedConsumable {
    pub(crate) fn offer_sell_value(price: u16, discount: u8) -> u16 {
        Self::discounted_price(price, discount) / 2
    }
    pub(crate) fn new(consumable: Consumable, original_price: u16) -> OwnedConsumable {
        OwnedConsumable {
            consumable,
            original_price,
            current_price: original_price,
            negative: false,
        }
    }

    pub(crate) fn negative(consumable: Consumable, original_price: u16) -> OwnedConsumable {
        OwnedConsumable {
            negative: true,
            ..OwnedConsumable::new(consumable, original_price)
        }
    }

    pub(crate) fn apply_discount(&mut self, discount_percent: u8) {
        self.current_price = Self::discounted_price(self.original_price, discount_percent);
    }

    fn discounted_price(price: u16, discount_percent: u8) -> u16 {
        let remaining_percent = 100u32.saturating_sub(discount_percent as u32);
        ((price as u32 * remaining_percent) / 100).max(1) as u16
    }

    pub(crate) fn sell_value(&self) -> u16 {
        self.current_price / 2
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ConsumableUseError {
    ConsumableNotFound,
    InvalidTarget,
    NoAvailableTarget,
    NoCapacity,
    NoPreviousConsumable,
}

#[derive(Default)]
pub(crate) struct ConsumableTarget {
    pub(crate) hand_indices: Vec<usize>,
}

impl ConsumableTarget {
    pub(crate) fn hand(indices: impl Into<Vec<usize>>) -> ConsumableTarget {
        ConsumableTarget {
            hand_indices: indices.into(),
        }
    }
}

/// The source stays owned until success; a normal owned source releases one
/// slot, while a Negative source releases both an object and its extra slot.
#[derive(Clone, Copy, Default)]
pub(crate) struct ConsumableUseSource {
    pub(crate) inventory_index: Option<usize>,
}

impl ConsumableUseSource {
    fn released_slots(self, state: &GameState) -> usize {
        self.inventory_index
            .and_then(|index| state.consumables.get(index))
            .map_or(0, |owned| usize::from(!owned.negative))
    }

    pub(crate) fn room(self, state: &GameState) -> usize {
        state
            .consumable_capacity()
            .saturating_add(self.released_slots(state))
            .saturating_sub(state.consumables.len())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConsumableTargets {
    pub(crate) minimum: usize,
    pub(crate) maximum: usize,
    pub(crate) ordered: bool,
}

pub(crate) fn planet_upgrade_values(hand: PokerHand) -> (u16, u16) {
    match hand {
        PokerHand::HighCard => (10, 1),
        PokerHand::Pair => (15, 1),
        PokerHand::TwoPair => (20, 1),
        PokerHand::ThreeOfAKind => (20, 2),
        PokerHand::Straight => (30, 3),
        PokerHand::Flush => (15, 2),
        PokerHand::FullHouse => (25, 2),
        PokerHand::FourOfAKind => (30, 3),
        PokerHand::StraightFlush => (40, 4),
        PokerHand::FiveOfAKind => (35, 3),
        PokerHand::FlushHouse => (40, 4),
        PokerHand::FlushFive => (50, 3),
    }
}

pub(crate) fn judgement_rarity(roll: u16) -> JokerRarity {
    match roll {
        0..7000 => JokerRarity::Common,
        7000..9500 => JokerRarity::Uncommon,
        _ => JokerRarity::Rare,
    }
}

pub(crate) fn judgement_edition(roll: u16) -> JokerEdition {
    match roll {
        0..30 => JokerEdition::Negative,
        30..60 => JokerEdition::Polychrome,
        60..200 => JokerEdition::Holographic,
        200..400 => JokerEdition::Foil,
        _ => JokerEdition::None,
    }
}

impl Consumable {
    pub(crate) fn targets(self) -> ConsumableTargets {
        let (minimum, maximum, ordered) = match self {
            Consumable::Tarot(Magician | Empress | Hierophant | Strength | HangedMan) => {
                (1, 2, false)
            }
            Consumable::Tarot(Lovers | Chariot | Justice | Devil | Tower)
            | Consumable::Spectral(Talisman | Aura | DejaVu | Trance | Medium | Cryptid) => {
                (1, 1, false)
            }
            Consumable::Tarot(Star | Moon | Sun | World) => (1, 3, false),
            Consumable::Tarot(Death) => (2, 2, true),
            _ => (0, 0, false),
        };
        ConsumableTargets {
            minimum,
            maximum,
            ordered,
        }
    }

    pub(crate) fn validate_use(
        self,
        state: &GameState,
        hand: &Hand,
        target: &ConsumableTarget,
        source: ConsumableUseSource,
    ) -> Result<(), ConsumableUseError> {
        let limits = self.targets();
        validate_hand_target(hand, target, limits.maximum, limits.minimum)?;
        self.validate_availability(state, hand, source)
    }

    /// Read-only rule checks. Random outcomes are never sampled here.
    pub(crate) fn validate_availability(
        self,
        state: &GameState,
        hand: &Hand,
        source: ConsumableUseSource,
    ) -> Result<(), ConsumableUseError> {
        if hand.cards().len() < self.targets().minimum {
            return Err(ConsumableUseError::NoAvailableTarget);
        }
        match self {
            Consumable::Tarot(Fool) => {
                let previous = state
                    .last_consumable
                    .ok_or(ConsumableUseError::NoPreviousConsumable)?;
                if !previous.negative && source.room(state) == 0 {
                    return Err(ConsumableUseError::NoCapacity);
                }
            }
            Consumable::Tarot(HighPriestess | Emperor) if source.room(state) == 0 => {
                return Err(ConsumableUseError::NoCapacity);
            }
            Consumable::Tarot(Judgement) | Consumable::Spectral(Wraith | Soul) => {
                require_joker_room(state)?
            }
            Consumable::Tarot(WheelOfFortune) | Consumable::Spectral(Ectoplasm | Hex) => {
                if state.jokers.as_slice().is_empty() {
                    return Err(ConsumableUseError::NoAvailableTarget);
                }
            }
            Consumable::Spectral(Ankh) => {
                if state.jokers.as_slice().is_empty() {
                    return Err(ConsumableUseError::NoAvailableTarget);
                }
                if !(0..state.jokers.as_slice().len())
                    .any(|index| state.jokers.can_duplicate_after_destroy_others(index))
                {
                    return Err(ConsumableUseError::NoCapacity);
                }
            }
            Consumable::Spectral(Familiar | Grim | Incantation | Sigil | Ouija)
                if hand.cards().is_empty() =>
            {
                return Err(ConsumableUseError::NoAvailableTarget);
            }
            Consumable::Spectral(Immolate) if hand.cards().len() < 5 => {
                return Err(ConsumableUseError::NoAvailableTarget);
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn apply(
        self,
        state: &mut GameState,
        hand: &mut Hand,
        target: &ConsumableTarget,
        source: ConsumableUseSource,
    ) -> Result<(), ConsumableUseError> {
        self.validate_use(state, hand, target, source)?;
        match self {
            Consumable::Tarot(card) => apply_tarot(card, state, hand, target, source),
            Consumable::Planet(card) => apply_planet(card, state),
            Consumable::Spectral(card) => apply_spectral(card, state, hand, target),
        }
    }
}

fn apply_tarot(
    card: Tarot,
    state: &mut GameState,
    hand: &mut Hand,
    target: &ConsumableTarget,
    source: ConsumableUseSource,
) -> Result<(), ConsumableUseError> {
    match card {
        Fool => {
            if state.last_consumable.is_none() {
                return Err(ConsumableUseError::NoPreviousConsumable);
            }
            state
                .consumables
                .push(state.last_consumable.expect("checked above"));
        }
        Magician => apply_hand_enhancement(state, hand, target, 2, Enhancement::Lucky)?,
        HighPriestess => add_random_consumables(state, 2, ConsumableKind::Planet, source)?,
        Emperor => add_random_consumables(state, 2, ConsumableKind::Tarot, source)?,
        Empress => apply_hand_enhancement(state, hand, target, 2, Enhancement::Mult)?,
        Hierophant => apply_hand_enhancement(state, hand, target, 2, Enhancement::Bonus)?,
        Lovers => apply_hand_enhancement(state, hand, target, 1, Enhancement::Wild)?,
        Chariot => apply_hand_enhancement(state, hand, target, 1, Enhancement::Steel)?,
        Justice => apply_hand_enhancement(state, hand, target, 1, Enhancement::Glass)?,
        Hermit => state.money = state.money.saturating_mul(2).min(20),
        WheelOfFortune => apply_wheel_of_fortune(state)?,
        Strength => {
            let indices = validate_hand_target(hand, target, 2, 1)?;
            for index in indices {
                let card = hand.card_mut(index).expect("validated hand target");
                card.set_rank(next_rank(card.rank()));
            }
        }
        HangedMan => {
            let indices = validate_hand_target(hand, target, 2, 1)?;
            destroy_hand_cards(state, hand, indices);
        }
        Death => {
            let indices = validate_hand_target(hand, target, 2, 2)?;
            if indices[0] == indices[1] {
                return Err(ConsumableUseError::InvalidTarget);
            }
            let right = hand
                .card(indices[1])
                .cloned()
                .expect("validated hand target");
            hand.card_mut(indices[0])
                .expect("validated hand target")
                .copy_properties_from(&right);
        }
        Temperance => {
            let money = state.jokers.total_sell_value().min(50) as i16;
            state.money = state.money.saturating_add(money);
        }
        Devil => apply_hand_enhancement(state, hand, target, 1, Enhancement::Gold)?,
        Tower => apply_hand_enhancement(state, hand, target, 1, Enhancement::Stone)?,
        Star => apply_hand_suit(hand, target, 3, Suit::Diamonds)?,
        Moon => apply_hand_suit(hand, target, 3, Suit::Clubs)?,
        Sun => apply_hand_suit(hand, target, 3, Suit::Hearts)?,
        Judgement => {
            require_joker_room(state)?;
            let rarity = judgement_rarity(state.rng.random_range(0..10_000));
            let mut joker = Joker::random_with_rarity(rarity, &mut state.rng)
                .expect("the Joker factory has every Judgement rarity");
            let edition = judgement_edition(state.rng.random_range(0..10_000));
            joker.set_edition(edition);
            state.jokers.add(joker);
        }
        World => apply_hand_suit(hand, target, 3, Suit::Spades)?,
    }

    Ok(())
}

fn apply_planet(card: Planet, state: &mut GameState) -> Result<(), ConsumableUseError> {
    let hand = planet_hand(card);
    let (chips, mult) = planet_upgrade_values(hand);
    state.upgrade_hand_score(hand, chips, mult);
    Ok(())
}

pub(crate) fn planet_hand(card: Planet) -> PokerHand {
    match card {
        Planet::Pluto => PokerHand::HighCard,
        Planet::Mercury => PokerHand::Pair,
        Planet::Uranus => PokerHand::TwoPair,
        Planet::Venus => PokerHand::ThreeOfAKind,
        Planet::Saturn => PokerHand::Straight,
        Planet::Jupiter => PokerHand::Flush,
        Planet::Earth => PokerHand::FullHouse,
        Planet::Mars => PokerHand::FourOfAKind,
        Planet::Neptune => PokerHand::StraightFlush,
        Planet::PlanetX => PokerHand::FiveOfAKind,
        Planet::Ceres => PokerHand::FlushHouse,
        Planet::Eris => PokerHand::FlushFive,
    }
}

fn apply_spectral(
    card: Spectral,
    state: &mut GameState,
    hand: &mut Hand,
    target: &ConsumableTarget,
) -> Result<(), ConsumableUseError> {
    match card {
        Familiar => {
            destroy_random_hand_card(state, hand)?;
            for _ in 0..3 {
                hand.add_card(random_card(
                    &[Rank::Jack, Rank::Queen, Rank::King],
                    &mut state.rng,
                ));
                state.update_jokers(UpdateEvent::DeckChanged { cards_added: 1 });
            }
        }
        Grim => {
            destroy_random_hand_card(state, hand)?;
            for _ in 0..2 {
                hand.add_card(random_card(&[Rank::Ace], &mut state.rng));
                state.update_jokers(UpdateEvent::DeckChanged { cards_added: 1 });
            }
        }
        Incantation => {
            destroy_random_hand_card(state, hand)?;
            for _ in 0..4 {
                hand.add_card(random_card(
                    &[
                        Rank::Two,
                        Rank::Three,
                        Rank::Four,
                        Rank::Five,
                        Rank::Six,
                        Rank::Seven,
                        Rank::Eight,
                        Rank::Nine,
                        Rank::Ten,
                    ],
                    &mut state.rng,
                ));
                state.update_jokers(UpdateEvent::DeckChanged { cards_added: 1 });
            }
        }
        Talisman => apply_hand_seal(hand, target, Seal::Gold)?,
        Aura => {
            let indices = validate_hand_target(hand, target, 1, 1)?;
            let edition = edition_for_roll(state.rng.random_range(0..100));
            hand.card_mut(indices[0])
                .expect("validated hand target")
                .set_edition(edition);
        }
        Wraith => {
            require_joker_room(state)?;
            let joker = Joker::random_with_rarity(JokerRarity::Rare, &mut state.rng)
                .expect("the Joker factory has rare Jokers");
            state.jokers.add(joker);
            state.money = 0;
        }
        Sigil => {
            if hand.hand_size() == 0 {
                return Err(ConsumableUseError::NoAvailableTarget);
            }
            let suit = ALL_SUITS
                .choose(&mut state.rng)
                .copied()
                .expect("suit choices are non-empty");
            for card in hand.cards_mut() {
                card.set_suit(suit);
            }
        }
        Ouija => {
            if hand.hand_size() == 0 {
                return Err(ConsumableUseError::NoAvailableTarget);
            }
            let rank = ALL_RANKS
                .choose(&mut state.rng)
                .copied()
                .expect("rank choices are non-empty");
            for card in hand.cards_mut() {
                card.set_rank(rank);
            }
            state.deck.adjust_hand_size(-1);
        }
        Ectoplasm => {
            let index = random_joker_index(state)?;
            state
                .jokers
                .get_mut(index)
                .expect("validated Joker target")
                .set_edition(JokerEdition::Negative);
            state.deck.adjust_hand_size(-1);
        }
        Immolate => {
            if hand.hand_size() < 5 {
                return Err(ConsumableUseError::NoAvailableTarget);
            }
            for _ in 0..5 {
                destroy_random_hand_card(state, hand)?;
            }
            state.money = state.money.saturating_add(20);
        }
        Ankh => {
            // No effects have been applied yet. Rewind only this random choice
            // if its survivor capacity check rejects the command.
            let position = state.rng.get_word_pos();
            let index = (0..state.jokers.as_slice().len())
                .choose(&mut state.rng)
                .ok_or(ConsumableUseError::NoAvailableTarget)?;
            if !state.jokers.can_duplicate_after_destroy_others(index) {
                state.rng.set_word_pos(position);
                return Err(ConsumableUseError::NoCapacity);
            }
            let mut joker = state
                .jokers
                .clone_at(index)
                .expect("validated Joker target");
            joker.clear_negative_edition();
            state.jokers.destroy_other_jokers(index);
            state.jokers.add(joker);
        }
        DejaVu => apply_hand_seal(hand, target, Seal::Red)?,
        Hex => {
            let index = random_joker_index(state)?;
            state
                .jokers
                .get_mut(index)
                .expect("validated Joker target")
                .set_edition(JokerEdition::Polychrome);
            state.jokers.destroy_other_jokers(index);
        }
        Trance => apply_hand_seal(hand, target, Seal::Blue)?,
        Medium => apply_hand_seal(hand, target, Seal::Purple)?,
        Cryptid => {
            let indices = validate_hand_target(hand, target, 1, 1)?;
            let card = hand
                .card(indices[0])
                .cloned()
                .expect("validated hand target");
            for card in [card.clone(), card] {
                hand.add_card(card);
                state.update_jokers(UpdateEvent::DeckChanged { cards_added: 1 });
            }
        }
        Soul => {
            require_joker_room(state)?;
            let joker = Joker::random_with_rarity(JokerRarity::Legendary, &mut state.rng)
                .expect("the Joker factory has Legendary Jokers");
            state.jokers.add(joker);
        }
        BlackHole => state.upgrade_all_hand_scores(),
    }

    Ok(())
}

enum ConsumableKind {
    Tarot,
    Planet,
}

fn add_random_consumables(
    state: &mut GameState,
    maximum: usize,
    kind: ConsumableKind,
    source: ConsumableUseSource,
) -> Result<(), ConsumableUseError> {
    let room = source.room(state);
    if room == 0 {
        return Err(ConsumableUseError::NoCapacity);
    }

    let amount = maximum.min(room);
    for _ in 0..amount {
        let consumable = match kind {
            ConsumableKind::Tarot => Consumable::Tarot(
                [
                    Tarot::Fool,
                    Tarot::Magician,
                    Tarot::HighPriestess,
                    Tarot::Emperor,
                    Tarot::Empress,
                    Tarot::Hierophant,
                    Tarot::Lovers,
                    Tarot::Chariot,
                    Tarot::Justice,
                    Tarot::Hermit,
                    Tarot::WheelOfFortune,
                    Tarot::Strength,
                    Tarot::HangedMan,
                    Tarot::Death,
                    Tarot::Temperance,
                    Tarot::Devil,
                    Tarot::Tower,
                    Tarot::Star,
                    Tarot::Moon,
                    Tarot::Sun,
                    Tarot::Judgement,
                    Tarot::World,
                ]
                .choose(&mut state.rng)
                .copied()
                .expect("tarot choices are non-empty"),
            ),
            ConsumableKind::Planet => Consumable::Planet(
                [
                    Planet::Pluto,
                    Planet::Mercury,
                    Planet::Uranus,
                    Planet::Venus,
                    Planet::Saturn,
                    Planet::Jupiter,
                    Planet::Earth,
                    Planet::Mars,
                    Planet::Neptune,
                    Planet::PlanetX,
                    Planet::Ceres,
                    Planet::Eris,
                ]
                .choose(&mut state.rng)
                .copied()
                .expect("planet choices are non-empty"),
            ),
        };
        let mut owned = OwnedConsumable::new(consumable, DEFAULT_CONSUMABLE_PRICE);
        owned.apply_discount(state.shop_discount_percent);
        state.consumables.push(owned);
    }
    Ok(())
}

fn require_joker_room(state: &GameState) -> Result<(), ConsumableUseError> {
    if state.joker_has_room() {
        Ok(())
    } else {
        Err(ConsumableUseError::NoCapacity)
    }
}

fn random_joker_index(state: &mut GameState) -> Result<usize, ConsumableUseError> {
    (0..state.jokers.as_slice().len())
        .choose(&mut state.rng)
        .ok_or(ConsumableUseError::NoAvailableTarget)
}

fn validate_hand_target(
    hand: &Hand,
    target: &ConsumableTarget,
    maximum: usize,
    minimum: usize,
) -> Result<Vec<usize>, ConsumableUseError> {
    let indices = &target.hand_indices;
    if indices.len() < minimum || indices.len() > maximum {
        return Err(ConsumableUseError::InvalidTarget);
    }
    if indices.iter().enumerate().any(|(position, index)| {
        *index >= hand.cards().len() || indices[..position].contains(index)
    }) {
        return Err(ConsumableUseError::InvalidTarget);
    }
    Ok(indices.clone())
}

fn apply_hand_enhancement(
    state: &mut GameState,
    hand: &mut Hand,
    target: &ConsumableTarget,
    maximum: usize,
    enhancement: Enhancement,
) -> Result<(), ConsumableUseError> {
    let indices = validate_hand_target(hand, target, maximum, 1)?;
    for index in indices {
        let card = hand.card_mut(index).expect("validated hand target");
        card.set_enhancement(enhancement);
        state.update_jokers(UpdateEvent::CardEnhancementAdded { card });
    }
    Ok(())
}

fn apply_hand_suit(
    hand: &mut Hand,
    target: &ConsumableTarget,
    maximum: usize,
    suit: Suit,
) -> Result<(), ConsumableUseError> {
    let indices = validate_hand_target(hand, target, maximum, 1)?;
    for index in indices {
        hand.card_mut(index)
            .expect("validated hand target")
            .set_suit(suit);
    }
    Ok(())
}

fn apply_hand_seal(
    hand: &mut Hand,
    target: &ConsumableTarget,
    seal: Seal,
) -> Result<(), ConsumableUseError> {
    let indices = validate_hand_target(hand, target, 1, 1)?;
    hand.card_mut(indices[0])
        .expect("validated hand target")
        .set_seal(seal);
    Ok(())
}

fn apply_wheel_of_fortune(state: &mut GameState) -> Result<(), ConsumableUseError> {
    let index = random_joker_index(state)?;
    if state.rng.random_range(0..4) as f32 >= state.jokers.modifiers().probability_multiplier {
        return Ok(());
    }
    let edition = joker_edition_for_roll(state.rng.random_range(0..100));
    state
        .jokers
        .get_mut(index)
        .expect("validated Joker target")
        .set_edition(edition);
    Ok(())
}

pub(crate) fn edition_for_roll(roll: u8) -> Edition {
    match roll {
        0..50 => Edition::Foil,
        50..85 => Edition::Holographic,
        _ => Edition::Polychrome,
    }
}

pub(crate) fn joker_edition_for_roll(roll: u8) -> JokerEdition {
    match edition_for_roll(roll) {
        Edition::Foil => JokerEdition::Foil,
        Edition::Holographic => JokerEdition::Holographic,
        Edition::Polychrome => JokerEdition::Polychrome,
        Edition::None => unreachable!("edition roll always produces an edition"),
    }
}

fn next_rank(rank: Rank) -> Rank {
    match rank {
        Rank::Two => Rank::Three,
        Rank::Three => Rank::Four,
        Rank::Four => Rank::Five,
        Rank::Five => Rank::Six,
        Rank::Six => Rank::Seven,
        Rank::Seven => Rank::Eight,
        Rank::Eight => Rank::Nine,
        Rank::Nine => Rank::Ten,
        Rank::Ten => Rank::Jack,
        Rank::Jack => Rank::Queen,
        Rank::Queen => Rank::King,
        Rank::King => Rank::Ace,
        Rank::Ace => Rank::Two,
    }
}

fn random_card(ranks: &[Rank], rng: &mut ChaCha12Rng) -> Card {
    let rank = ranks
        .choose(rng)
        .copied()
        .expect("rank choices are non-empty");
    let suit = ALL_SUITS
        .choose(rng)
        .copied()
        .expect("suit choices are non-empty");
    let enhancement = [
        Enhancement::Stone,
        Enhancement::Gold,
        Enhancement::Bonus,
        Enhancement::Mult,
        Enhancement::Wild,
        Enhancement::Lucky,
        Enhancement::Glass,
        Enhancement::Steel,
    ]
    .choose(rng)
    .copied()
    .expect("enhancement choices are non-empty");
    Card::new(rank, suit, enhancement, Edition::None, Seal::None)
}

fn destroy_random_hand_card(
    state: &mut GameState,
    hand: &mut Hand,
) -> Result<(), ConsumableUseError> {
    let index = (0..hand.cards().len())
        .choose(&mut state.rng)
        .ok_or(ConsumableUseError::NoAvailableTarget)?;
    remap_destroyed_target(state, &[index]);
    let card = hand
        .remove_card_at(index)
        .expect("validated random hand target");
    state.deck.record_destruction(&card);
    state.update_jokers(UpdateEvent::CardDestroyed { card: &card });
    Ok(())
}

fn destroy_hand_cards(state: &mut GameState, hand: &mut Hand, mut indices: Vec<usize>) {
    remap_destroyed_target(state, &indices);
    indices.sort_unstable_by(|left, right| right.cmp(left));
    for index in indices {
        if let Some(card) = hand.remove_card_at(index) {
            state.deck.record_destruction(&card);
            state.update_jokers(UpdateEvent::CardDestroyed { card: &card });
        }
    }
}

fn remap_destroyed_target(state: &mut GameState, removed: &[usize]) {
    state.blind.forced_card_index = state
        .blind
        .forced_card_index
        .filter(|index| !removed.contains(index))
        .map(|index| index - removed.iter().filter(|&&removed| removed < index).count());
}
