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
use rand::prelude::{IndexedRandom, IteratorRandom, RngExt};
use rand::rngs::StdRng;

pub(crate) const DEFAULT_CONSUMABLE_SLOTS: usize = 2;

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
    pub(crate) deck_indices: Vec<usize>,
    pub(crate) joker_index: Option<usize>,
}

impl ConsumableTarget {
    pub(crate) fn hand(indices: impl Into<Vec<usize>>) -> ConsumableTarget {
        ConsumableTarget {
            hand_indices: indices.into(),
            ..ConsumableTarget::default()
        }
    }

    pub(crate) fn deck(indices: impl Into<Vec<usize>>) -> ConsumableTarget {
        ConsumableTarget {
            deck_indices: indices.into(),
            ..ConsumableTarget::default()
        }
    }

    pub(crate) fn joker(index: usize) -> ConsumableTarget {
        ConsumableTarget {
            joker_index: Some(index),
            ..ConsumableTarget::default()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PokerHandLevel {
    pub(crate) level: u16,
    pub(crate) chips: u32,
    pub(crate) mult: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PokerHandLevels {
    levels: [PokerHandLevel; 12],
}

impl PokerHandLevels {
    pub(crate) fn new() -> PokerHandLevels {
        let initial_level = PokerHandLevel {
            level: 1,
            chips: 0,
            mult: 0,
        };
        PokerHandLevels {
            levels: [initial_level; 12],
        }
    }

    pub(crate) fn get(&self, hand: PokerHand) -> PokerHandLevel {
        self.levels[hand_index(hand)]
    }

    pub(crate) fn upgrade(&mut self, hand: PokerHand, chips: u32, mult: u32) {
        let entry = &mut self.levels[hand_index(hand)];
        entry.level = entry.level.saturating_add(1);
        entry.chips = entry.chips.saturating_add(chips);
        entry.mult = entry.mult.saturating_add(mult);
    }

    pub(crate) fn upgrade_all(&mut self) {
        for hand in [
            PokerHand::HighCard,
            PokerHand::Pair,
            PokerHand::ThreeOfAKind,
            PokerHand::FourOfAKind,
            PokerHand::FiveOfAKind,
            PokerHand::TwoPair,
            PokerHand::Straight,
            PokerHand::Flush,
            PokerHand::FullHouse,
            PokerHand::StraightFlush,
            PokerHand::FlushHouse,
            PokerHand::FlushFive,
        ] {
            let (chips, mult) = planet_upgrade(hand);
            self.upgrade(hand, chips, mult);
        }
    }
}

fn hand_index(hand: PokerHand) -> usize {
    hand as usize - 1
}

fn planet_upgrade(hand: PokerHand) -> (u32, u32) {
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
    pub(crate) fn apply(
        self,
        state: &mut GameState,
        hand: &mut Hand,
        target: &ConsumableTarget,
    ) -> Result<(), ConsumableUseError> {
        match self {
            Consumable::Tarot(card) => apply_tarot(card, state, hand, target),
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
) -> Result<(), ConsumableUseError> {
    match card {
        Fool => {
            if state.last_consumable.is_none() {
                return Err(ConsumableUseError::NoPreviousConsumable);
            }
            require_consumable_room(state, 1)?;
            state
                .consumables
                .push(state.last_consumable.expect("checked above"));
        }
        Magician => apply_hand_enhancement(state, hand, target, 2, Enhancement::Lucky)?,
        HighPriestess => add_random_consumables(state, 2, ConsumableKind::Planet)?,
        Emperor => add_random_consumables(state, 2, ConsumableKind::Tarot)?,
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
    let (chips, mult) = planet_upgrade(hand);
    state.hand_levels.upgrade(hand, chips, mult);
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
            }
        }
        Grim => {
            destroy_random_hand_card(state, hand)?;
            for _ in 0..2 {
                hand.add_card(random_card(&[Rank::Ace], &mut state.rng));
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
            state.counters.hand_size_modifier = state.counters.hand_size_modifier.saturating_sub(1);
        }
        Ectoplasm => {
            let index = random_joker_index(state)?;
            state
                .jokers
                .get_mut(index)
                .expect("validated Joker target")
                .set_edition(JokerEdition::Negative);
            state.counters.hand_size_modifier = state.counters.hand_size_modifier.saturating_sub(1);
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
            let index = random_joker_index(state)?;
            let mut joker = state
                .jokers
                .clone_at(index)
                .expect("validated Joker target");
            joker.clear_negative_edition();
            state.jokers.replace_with(joker);
        }
        DejaVu => apply_hand_seal(hand, target, Seal::Red)?,
        Hex => {
            let index = random_joker_index(state)?;
            state
                .jokers
                .get_mut(index)
                .expect("validated Joker target")
                .set_edition(JokerEdition::Polychrome);
            state.jokers.keep_only(index);
        }
        Trance => apply_hand_seal(hand, target, Seal::Blue)?,
        Medium => apply_hand_seal(hand, target, Seal::Purple)?,
        Cryptid => {
            let indices = validate_hand_target(hand, target, 1, 1)?;
            let card = hand
                .card(indices[0])
                .cloned()
                .expect("validated hand target");
            hand.add_card(card.clone());
            hand.add_card(card);
        }
        Soul => {
            require_joker_room(state)?;
            let joker = Joker::random_with_rarity(JokerRarity::Legendary, &mut state.rng)
                .expect("the Joker factory has Legendary Jokers");
            state.jokers.add(joker);
        }
        BlackHole => state.hand_levels.upgrade_all(),
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
) -> Result<(), ConsumableUseError> {
    let room = state
        .consumable_capacity()
        .saturating_add(1)
        .saturating_sub(state.consumables.len());
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
        state.consumables.push(consumable);
    }
    Ok(())
}

fn require_consumable_room(state: &GameState, amount: usize) -> Result<(), ConsumableUseError> {
    if state.consumables.len() + amount > state.consumable_capacity().saturating_add(1) {
        Err(ConsumableUseError::NoCapacity)
    } else {
        Ok(())
    }
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
        *index >= hand.hand_size() as usize || indices[..position].contains(index)
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
    if state.rng.random_range(0..4) != 0 {
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

fn random_card(ranks: &[Rank], rng: &mut StdRng) -> Card {
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
    let index = (0..hand.hand_size() as usize)
        .choose(&mut state.rng)
        .ok_or(ConsumableUseError::NoAvailableTarget)?;
    let card = hand
        .remove_card_at(index)
        .expect("validated random hand target");
    state.update_jokers(UpdateEvent::CardDestroyed { card: &card });
    Ok(())
}

fn destroy_hand_cards(state: &mut GameState, hand: &mut Hand, mut indices: Vec<usize>) {
    indices.sort_unstable_by(|left, right| right.cmp(left));
    for index in indices {
        if let Some(card) = hand.remove_card_at(index) {
            state.update_jokers(UpdateEvent::CardDestroyed { card: &card });
        }
    }
}
