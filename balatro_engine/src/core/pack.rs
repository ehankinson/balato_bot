use crate::core::card::Card;
use crate::core::consumable::ConsumableUseError;
use crate::core::enums::{
    ALL_RANKS, ALL_SUITS, Consumable, Edition, Enhancement, Planet, Seal, Spectral, Tarot,
};
use crate::core::game::GameState;
use crate::core::joker::{Joker, UpdateEvent};
use crate::core::joker_types::{JokerKind, JokerRarity};
use crate::core::shop::ShopItem;
use crate::core::voucher::{random_shop_joker_edition, random_standard_pack_edition};
use rand::prelude::{IndexedRandom, IteratorRandom, RngExt};

pub(crate) const SOUL_PACK_ROLL_DENOMINATOR: u16 = 1_000;
pub(crate) const SOUL_PACK_ROLL_THRESHOLD: u16 = 3;

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
        let mut generated_consumables = Vec::with_capacity(self.option_count());
        let mut generated_jokers = Vec::with_capacity(self.option_count());
        let mut options = Vec::with_capacity(self.option_count());
        for _ in 0..self.option_count() {
            let option = random_option(self.kind, state, &generated_consumables, &generated_jokers);
            match &option {
                PackOption::Consumable(consumable) => generated_consumables.push(*consumable),
                PackOption::Joker(joker) => generated_jokers.push(joker.kind()),
                PackOption::PlayingCard(_) => {}
            }
            options.push(option);
        }
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

fn random_option(
    kind: BoosterPackKind,
    state: &mut GameState,
    generated_consumables: &[Consumable],
    generated_jokers: &[JokerKind],
) -> PackOption {
    match kind {
        BoosterPackKind::Standard => PackOption::PlayingCard(random_playing_card(state)),
        BoosterPackKind::Arcana => {
            PackOption::Consumable(random_arcana_card(state, generated_consumables))
        }
        BoosterPackKind::Celestial => {
            PackOption::Consumable(random_planet_card(state, generated_consumables))
        }
        BoosterPackKind::Buffoon => {
            let mut joker = random_pack_joker(state, generated_jokers);
            joker.set_edition(random_shop_joker_edition(state));
            PackOption::Joker(joker)
        }
        BoosterPackKind::Spectral => {
            PackOption::Consumable(random_spectral_card(state, generated_consumables))
        }
    }
}

fn random_pack_joker(state: &mut GameState, generated_jokers: &[JokerKind]) -> Joker {
    let eligible = JokerKind::ALL
        .iter()
        .copied()
        .filter(|kind| Joker::create_joker(*kind).rarity() != JokerRarity::Legendary)
        .filter(|kind| joker_available_in_pack(state, *kind, generated_jokers))
        .collect::<Vec<_>>();
    let kind = eligible
        .choose(&mut state.rng)
        .copied()
        .unwrap_or_else(|| Joker::random_shop_joker(&mut state.rng).kind());

    Joker::create_joker(kind)
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
    let edition = if state.rng.random_range(0..100u8) < state.shop.playing_card_edition_chance {
        match state.rng.random_range(0..100u8) {
            0..50 => Edition::Foil,
            50..85 => Edition::Holographic,
            _ => Edition::Polychrome,
        }
    } else {
        Edition::None
    };
    let enhancement =
        if state.rng.random_range(0..100u8) < state.shop.playing_card_enhancement_chance {
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

fn random_arcana_card(state: &mut GameState, generated_consumables: &[Consumable]) -> Consumable {
    let soul = Consumable::Spectral(Spectral::Soul);
    let soul_roll = state.rng.random_range(0..SOUL_PACK_ROLL_DENOMINATOR);
    if consumable_available_in_pack(state, soul, generated_consumables)
        && soul_pack_roll_succeeds(soul_roll)
    {
        return soul;
    }

    if state.has_voucher(crate::core::enums::Vouchers::OmenGlobe)
        && state.rng.random_range(0..5u8) == 0
    {
        random_spectral_card(state, generated_consumables)
    } else {
        let tarot = available_tarot_cards(state, generated_consumables)
            .choose(&mut state.rng)
            .copied()
            .or_else(|| Tarot::ALL.choose(&mut state.rng).copied())
            .expect("tarot choices are non-empty");
        Consumable::Tarot(tarot)
    }
}

fn random_planet_card(state: &mut GameState, generated_consumables: &[Consumable]) -> Consumable {
    let targeted_planet = state
        .shop
        .most_played_planet_in_pack
        .then(|| state.most_played_hand.map(planet_for_hand))
        .flatten()
        .filter(|planet| {
            consumable_available_in_pack(state, Consumable::Planet(*planet), generated_consumables)
        });

    let planet = targeted_planet
        .or_else(|| {
            available_planets(state, generated_consumables)
                .choose(&mut state.rng)
                .copied()
        })
        .unwrap_or_else(|| random_planet(state));
    Consumable::Planet(planet)
}

fn random_planet(state: &mut GameState) -> Planet {
    Planet::ALL
        .choose(&mut state.rng)
        .copied()
        .expect("planet choices are non-empty")
}

fn random_spectral_card(state: &mut GameState, generated_consumables: &[Consumable]) -> Consumable {
    let soul = Consumable::Spectral(Spectral::Soul);
    let soul_roll = state.rng.random_range(0..SOUL_PACK_ROLL_DENOMINATOR);
    if consumable_available_in_pack(state, soul, generated_consumables)
        && soul_pack_roll_succeeds(soul_roll)
    {
        return soul;
    }

    let spectral = available_spectral_cards(state, generated_consumables)
        .into_iter()
        .filter(|spectral| *spectral != Spectral::Soul)
        .choose(&mut state.rng)
        .or_else(|| {
            Spectral::ALL
                .iter()
                .copied()
                .filter(|spectral| *spectral != Spectral::Soul)
                .choose(&mut state.rng)
        })
        .expect("spectral choices are non-empty");
    Consumable::Spectral(spectral)
}

fn available_tarot_cards(state: &GameState, generated_consumables: &[Consumable]) -> Vec<Tarot> {
    Tarot::ALL
        .iter()
        .copied()
        .filter(|tarot| {
            consumable_available_in_pack(state, Consumable::Tarot(*tarot), generated_consumables)
        })
        .collect()
}

fn available_spectral_cards(
    state: &GameState,
    generated_consumables: &[Consumable],
) -> Vec<Spectral> {
    Spectral::ALL
        .iter()
        .copied()
        .filter(|spectral| {
            consumable_available_in_pack(
                state,
                Consumable::Spectral(*spectral),
                generated_consumables,
            )
        })
        .collect()
}

fn available_planets(state: &GameState, generated_consumables: &[Consumable]) -> Vec<Planet> {
    Planet::ALL
        .iter()
        .copied()
        .filter(|planet| {
            consumable_available_in_pack(state, Consumable::Planet(*planet), generated_consumables)
        })
        .collect()
}

pub(crate) fn consumable_available_in_pack(
    state: &GameState,
    consumable: Consumable,
    generated_consumables: &[Consumable],
) -> bool {
    state.has_showman()
        || (!state
            .consumables
            .iter()
            .any(|owned| owned.consumable == consumable)
            && !state.shop.has_consumable_offer(consumable)
            && !generated_consumables.contains(&consumable))
}

fn joker_available_in_pack(
    state: &GameState,
    kind: JokerKind,
    generated_jokers: &[JokerKind],
) -> bool {
    state.has_showman()
        || (!state
            .jokers
            .as_slice()
            .iter()
            .any(|joker| joker.kind() == kind)
            && !state
                .shop
                .items
                .iter()
                .any(|item| matches!(item, ShopItem::Joker { joker, .. } if joker.kind() == kind))
            && !generated_jokers.contains(&kind))
}

pub(crate) const fn soul_pack_roll_succeeds(roll: u16) -> bool {
    roll < SOUL_PACK_ROLL_THRESHOLD
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
