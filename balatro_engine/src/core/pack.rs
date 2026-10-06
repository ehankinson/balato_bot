use crate::core::card::Card;
use crate::core::consumable::ConsumableUseError;
use crate::core::enums::{
    ALL_RANKS, ALL_SUITS, Consumable, Edition, Enhancement, Planet, Seal, Spectral, Tarot,
};
use crate::core::game::GameState;
use crate::core::joker::{Joker, UpdateEvent};
use crate::core::voucher::{random_shop_joker_edition, random_standard_pack_edition};
use rand::prelude::{IndexedRandom, RngExt};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum BoosterPackKind {
    Standard,
    Arcana,
    Celestial,
    Buffoon,
    Spectral,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum BoosterPackSize {
    Normal,
    Jumbo,
    Mega,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) struct ShopPack {
    pub(crate) kind: BoosterPackKind,
    pub(crate) size: BoosterPackSize,
    pub(crate) price: u16,
}

impl ShopPack {
    pub(crate) fn new(kind: BoosterPackKind, size: BoosterPackSize, price: u16) -> ShopPack {
        ShopPack { kind, size, price }
    }

    pub(crate) fn option_count(&self) -> usize {
        match (self.kind, self.size) {
            (BoosterPackKind::Standard, BoosterPackSize::Normal)
            | (BoosterPackKind::Arcana, BoosterPackSize::Normal)
            | (BoosterPackKind::Celestial, BoosterPackSize::Normal) => 3,
            (BoosterPackKind::Standard, BoosterPackSize::Jumbo)
            | (BoosterPackKind::Arcana, BoosterPackSize::Jumbo)
            | (BoosterPackKind::Celestial, BoosterPackSize::Jumbo)
            | (BoosterPackKind::Standard, BoosterPackSize::Mega)
            | (BoosterPackKind::Arcana, BoosterPackSize::Mega)
            | (BoosterPackKind::Celestial, BoosterPackSize::Mega) => 5,
            (BoosterPackKind::Buffoon, BoosterPackSize::Normal)
            | (BoosterPackKind::Spectral, BoosterPackSize::Normal) => 2,
            (BoosterPackKind::Buffoon, BoosterPackSize::Jumbo)
            | (BoosterPackKind::Spectral, BoosterPackSize::Jumbo)
            | (BoosterPackKind::Buffoon, BoosterPackSize::Mega)
            | (BoosterPackKind::Spectral, BoosterPackSize::Mega) => 4,
        }
    }

    pub(crate) fn choices(&self) -> usize {
        usize::from(self.size == BoosterPackSize::Mega) + 1
    }

    pub(crate) fn open(&self, state: &mut GameState) -> PackOpening {
        let options = (0..self.option_count())
            .map(|_| random_option(self.kind, state))
            .collect();
        PackOpening {
            options,
            choices: self.choices(),
        }
    }
}

#[derive(Clone)]
pub(crate) enum PackOption {
    Joker(Joker),
    Consumable(Consumable),
    PlayingCard(Card),
}

pub(crate) struct PackOpening {
    options: Vec<PackOption>,
    choices: usize,
}

impl PackOpening {
    pub(crate) fn options(&self) -> &[PackOption] {
        &self.options
    }

    pub(crate) fn choices(&self) -> usize {
        self.choices
    }

    pub(crate) fn choose(
        self,
        selections: &[usize],
        state: &mut GameState,
    ) -> Result<(), PackError> {
        validate_selections(&self, selections, state)?;

        for index in selections {
            match self.options.get(*index).expect("validated pack selection") {
                PackOption::Joker(joker) => {
                    state.jokers.add(joker.clone());
                    state.update_jokers(UpdateEvent::JokerOrderChanged);
                }
                PackOption::Consumable(consumable) => state
                    .add_consumable(*consumable)
                    .map_err(PackError::Consumable)?,
                PackOption::PlayingCard(card) => {
                    state.deck.add_card_to_deck(card.clone());
                    state.update_jokers(UpdateEvent::DeckChanged { cards_added: 1 });
                }
            }
        }

        Ok(())
    }
}

pub(crate) const SHOP_PACK_WEIGHTS: [(BoosterPackKind, BoosterPackSize, u16, u32); 15] = [
    (BoosterPackKind::Standard, BoosterPackSize::Normal, 4, 400),
    (BoosterPackKind::Standard, BoosterPackSize::Jumbo, 6, 200),
    (BoosterPackKind::Standard, BoosterPackSize::Mega, 8, 50),
    (BoosterPackKind::Arcana, BoosterPackSize::Normal, 4, 400),
    (BoosterPackKind::Arcana, BoosterPackSize::Jumbo, 6, 200),
    (BoosterPackKind::Arcana, BoosterPackSize::Mega, 8, 50),
    (BoosterPackKind::Celestial, BoosterPackSize::Normal, 4, 400),
    (BoosterPackKind::Celestial, BoosterPackSize::Jumbo, 6, 200),
    (BoosterPackKind::Celestial, BoosterPackSize::Mega, 8, 50),
    (BoosterPackKind::Buffoon, BoosterPackSize::Normal, 4, 120),
    (BoosterPackKind::Buffoon, BoosterPackSize::Jumbo, 6, 60),
    (BoosterPackKind::Buffoon, BoosterPackSize::Mega, 8, 15),
    (BoosterPackKind::Spectral, BoosterPackSize::Normal, 4, 60),
    (BoosterPackKind::Spectral, BoosterPackSize::Jumbo, 6, 30),
    (BoosterPackKind::Spectral, BoosterPackSize::Mega, 8, 7),
];

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PackError {
    InvalidSelection,
    InventoryFull,
    Consumable(ConsumableUseError),
}

fn validate_selections(
    opening: &PackOpening,
    selections: &[usize],
    state: &GameState,
) -> Result<(), PackError> {
    if selections.len() != opening.choices
        || selections
            .iter()
            .any(|index| *index >= opening.options.len())
        || selections
            .iter()
            .enumerate()
            .any(|(position, index)| selections[..position].contains(index))
    {
        return Err(PackError::InvalidSelection);
    }

    let joker_count = selections
        .iter()
        .filter(|index| matches!(opening.options[**index], PackOption::Joker(_)))
        .count();
    if state.jokers.as_slice().len() + joker_count > state.joker_capacity() {
        return Err(PackError::InventoryFull);
    }

    let consumable_count = selections
        .iter()
        .filter(|index| matches!(opening.options[**index], PackOption::Consumable(_)))
        .count();
    if state.consumables.len() + consumable_count > state.consumable_capacity() {
        return Err(PackError::InventoryFull);
    }

    Ok(())
}

pub(crate) fn random_shop_pack(state: &mut GameState) -> ShopPack {
    let total_weight = SHOP_PACK_WEIGHTS.iter().map(|choice| choice.3).sum();
    let mut roll = state.rng.random_range(0..total_weight);
    for (kind, size, price, weight) in SHOP_PACK_WEIGHTS {
        if roll < weight {
            return ShopPack::new(kind, size, price);
        }
        roll -= weight;
    }
    unreachable!()
}

fn random_option(kind: BoosterPackKind, state: &mut GameState) -> PackOption {
    match kind {
        BoosterPackKind::Standard => PackOption::PlayingCard(random_playing_card(state)),
        BoosterPackKind::Arcana => PackOption::Consumable(random_arcana_card(state)),
        BoosterPackKind::Celestial => PackOption::Consumable(random_planet_card(state)),
        BoosterPackKind::Buffoon => {
            let mut joker = Joker::random_shop_joker(&mut state.rng);
            joker.set_edition(random_shop_joker_edition(state));
            PackOption::Joker(joker)
        }
        BoosterPackKind::Spectral => PackOption::Consumable(random_spectral_card(state)),
    }
}

pub(crate) fn random_playing_card(state: &mut GameState) -> Card {
    let enhancement = if state.rng.random_range(0..100u8) < 40 {
        [
            Enhancement::Bonus,
            Enhancement::Mult,
            Enhancement::Wild,
            Enhancement::Glass,
            Enhancement::Steel,
            Enhancement::Stone,
            Enhancement::Gold,
            Enhancement::Lucky,
        ]
        .choose(&mut state.rng)
        .copied()
        .expect("enhancement choices are non-empty")
    } else {
        Enhancement::None
    };
    let seal = if state.rng.random_range(0..100u8) < 20 {
        [Seal::Red, Seal::Blue, Seal::Gold, Seal::Purple]
            .choose(&mut state.rng)
            .copied()
            .expect("seal choices are non-empty")
    } else {
        Seal::None
    };

    Card::new(
        *ALL_RANKS
            .choose(&mut state.rng)
            .expect("rank choices are non-empty"),
        *ALL_SUITS
            .choose(&mut state.rng)
            .expect("suit choices are non-empty"),
        enhancement,
        random_standard_pack_edition(state),
        seal,
    )
}

pub(crate) fn random_shop_playing_card(state: &mut GameState) -> Card {
    let edition = match state.rng.random_range(0..100u8) {
        0..10 => Edition::Foil,
        10..17 => Edition::Holographic,
        17..20 => Edition::Polychrome,
        _ => Edition::None,
    };
    let enhancement = if state.rng.random_range(0..100u8) < 40 {
        [
            Enhancement::Bonus,
            Enhancement::Mult,
            Enhancement::Wild,
            Enhancement::Lucky,
            Enhancement::Glass,
            Enhancement::Steel,
            Enhancement::Stone,
            Enhancement::Gold,
        ]
        .choose(&mut state.rng)
        .copied()
        .expect("enhancement choices are non-empty")
    } else {
        Enhancement::None
    };
    Card::new(
        *ALL_RANKS
            .choose(&mut state.rng)
            .expect("rank choices are non-empty"),
        *ALL_SUITS
            .choose(&mut state.rng)
            .expect("suit choices are non-empty"),
        enhancement,
        edition,
        Seal::None,
    )
}

fn random_arcana_card(state: &mut GameState) -> Consumable {
    if state.has_voucher(crate::core::enums::Vouchers::OmenGlobe)
        && state.rng.random_range(0..5u8) == 0
    {
        random_spectral_card(state)
    } else {
        let tarot = Tarot::ALL
            .choose(&mut state.rng)
            .copied()
            .expect("tarot choices are non-empty");
        Consumable::Tarot(tarot)
    }
}

fn random_planet_card(state: &mut GameState) -> Consumable {
    let planet = Planet::ALL
        .choose(&mut state.rng)
        .copied()
        .expect("planet choices are non-empty");
    Consumable::Planet(planet)
}

fn random_spectral_card(state: &mut GameState) -> Consumable {
    let spectral = Spectral::ALL
        .choose(&mut state.rng)
        .copied()
        .expect("spectral choices are non-empty");
    Consumable::Spectral(spectral)
}

pub(crate) fn planet_for_hand(hand: crate::core::enums::PokerHand) -> Planet {
    match hand {
        crate::core::enums::PokerHand::HighCard => Planet::Pluto,
        crate::core::enums::PokerHand::Pair => Planet::Mercury,
        crate::core::enums::PokerHand::TwoPair => Planet::Uranus,
        crate::core::enums::PokerHand::ThreeOfAKind => Planet::Venus,
        crate::core::enums::PokerHand::Straight => Planet::Saturn,
        crate::core::enums::PokerHand::Flush => Planet::Jupiter,
        crate::core::enums::PokerHand::FullHouse => Planet::Earth,
        crate::core::enums::PokerHand::FourOfAKind => Planet::Mars,
        crate::core::enums::PokerHand::StraightFlush => Planet::Neptune,
        crate::core::enums::PokerHand::FiveOfAKind => Planet::PlanetX,
        crate::core::enums::PokerHand::FlushHouse => Planet::Ceres,
        crate::core::enums::PokerHand::FlushFive => Planet::Eris,
    }
}
