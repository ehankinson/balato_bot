use super::*;

use crate::core::enums::{Consumable, Planet, PokerHand, Spectral, Tarot};
use crate::core::joker::Joker;
use crate::core::joker_types::JokerKind;
use crate::core::pack::{
    BoosterPackKind, BoosterPackSize, PackError, PackOption, SHOP_PACK_WEIGHTS,
    SOUL_PACK_ROLL_DENOMINATOR, ShopPack, consumable_available_in_pack, joker_available_in_pack,
    soul_pack_roll_succeeds,
};
use crate::core::shop::{ShopItem, ShopState};

#[test]
fn pack_size_controls_option_and_selection_counts() {
    let standard = ShopPack::new(BoosterPackKind::Standard, BoosterPackSize::Normal, 4);
    let mega = ShopPack::new(BoosterPackKind::Standard, BoosterPackSize::Mega, 8);
    let buffoon = ShopPack::new(BoosterPackKind::Buffoon, BoosterPackSize::Jumbo, 6);

    assert_eq!(standard.option_count(), 3);
    assert_eq!(standard.choices(), 1);
    assert_eq!(mega.option_count(), 5);
    assert_eq!(mega.choices(), 2);
    assert_eq!(buffoon.option_count(), 4);
    assert_eq!(buffoon.choices(), 1);
}

#[test]
fn opening_and_choosing_a_standard_pack_adds_a_card_to_the_deck() {
    let mut state = GameState::new(25);
    let starting_size = state.deck_size();
    let pack = ShopPack::new(BoosterPackKind::Standard, BoosterPackSize::Normal, 4);
    let mut opening = pack.open(&mut state);

    assert_eq!(opening.options().len(), 3);
    assert!(matches!(opening.options()[0], PackOption::PlayingCard(_)));
    state.select_pack_options(&mut opening, &[0]).unwrap();

    assert_eq!(state.deck_size(), starting_size + 1);
}

#[test]
fn pack_selection_is_atomic_when_the_selection_is_invalid() {
    let mut state = GameState::new(26);
    let pack = ShopPack::new(BoosterPackKind::Buffoon, BoosterPackSize::Mega, 8);
    let mut opening = pack.open(&mut state);

    assert_eq!(
        state.select_pack_options(&mut opening, &[0, 0]),
        Err(PackError::InvalidSelection)
    );
    assert!(state.jokers.as_slice().is_empty());
}

#[test]
fn buying_and_opening_a_pack_charges_the_discounted_price() {
    let mut state = GameState::new(27);
    state.money = 10;
    state
        .redeem_voucher(crate::core::enums::Vouchers::ClearanceSale)
        .unwrap();
    let mut shop = ShopState::new();
    shop.add_pack(ShopPack::new(
        BoosterPackKind::Arcana,
        BoosterPackSize::Normal,
        4,
    ))
    .unwrap();

    let opening = shop.buy_and_open_pack(0, &mut state).unwrap();

    assert_eq!(state.money, 7);
    assert_eq!(shop.packs.len(), 0);
    assert_eq!(opening.options().len(), 3);
    assert!(matches!(opening.options()[0], PackOption::Consumable(_)));
}

#[test]
fn shop_pack_weights_match_the_configured_distribution() {
    let weights = SHOP_PACK_WEIGHTS.map(|choice| choice.3);

    assert_eq!(
        weights,
        [
            400, 200, 50, 400, 200, 50, 400, 200, 50, 120, 60, 15, 60, 30, 7,
        ]
    );
    assert_eq!(weights.into_iter().sum::<u32>(), 2242);
}

#[test]
fn telescope_forces_the_first_planet_even_when_already_owned() {
    let mut state = GameState::new(28);
    state.counters.poker_hands_played[PokerHand::Pair as usize - 1] = 1;
    state
        .add_consumable(Consumable::Planet(Planet::Mercury))
        .unwrap();
    state
        .redeem_voucher(crate::core::enums::Vouchers::Telescope)
        .unwrap();
    let pack = ShopPack::new(BoosterPackKind::Celestial, BoosterPackSize::Normal, 4);
    let opening = pack.open(&mut state);
    assert!(matches!(
        opening.options()[0],
        PackOption::Consumable(Consumable::Planet(Planet::Mercury))
    ));
    assert!(opening.options()[1..].iter().all(|option| !matches!(
        option,
        PackOption::Consumable(Consumable::Planet(Planet::Mercury))
    )));
}

#[test]
fn arcana_and_spectral_packs_exclude_shop_and_owned_cards_without_showman() {
    let mut state = GameState::new(29);
    state.shop.add_item(ShopItem::Consumable {
        consumable: Consumable::Tarot(Tarot::Fool),
        price: 3,
    });
    state
        .add_consumable(Consumable::Tarot(Tarot::Magician))
        .unwrap();

    let arcana = ShopPack::new(BoosterPackKind::Arcana, BoosterPackSize::Mega, 8).open(&mut state);
    assert!(arcana.options().iter().all(|option| {
        !matches!(
            option,
            PackOption::Consumable(Consumable::Tarot(Tarot::Fool | Tarot::Magician))
        )
    }));

    assert!(!consumable_available_in_pack(
        &state,
        Consumable::Tarot(Tarot::Fool),
        &[]
    ));
    assert!(!consumable_available_in_pack(
        &state,
        Consumable::Tarot(Tarot::Magician),
        &[]
    ));
}

#[test]
fn celestial_packs_exclude_shop_and_owned_planets_without_showman() {
    let mut state = GameState::new(32);
    state.shop.add_item(ShopItem::Consumable {
        consumable: Consumable::Planet(Planet::Mercury),
        price: 3,
    });
    state
        .add_consumable(Consumable::Planet(Planet::Venus))
        .unwrap();

    let opening =
        ShopPack::new(BoosterPackKind::Celestial, BoosterPackSize::Mega, 8).open(&mut state);

    assert!(opening.options().iter().all(|option| {
        !matches!(
            option,
            PackOption::Consumable(Consumable::Planet(Planet::Mercury | Planet::Venus))
        )
    }));
}

#[test]
fn showman_allows_pack_duplicates_and_cards_seen_elsewhere() {
    let mut state = GameState::new(30);
    state.jokers.add(Joker::create_joker(
        crate::core::joker_types::JokerKind::Showman,
    ));
    state.shop.add_item(ShopItem::Consumable {
        consumable: Consumable::Tarot(Tarot::Fool),
        price: 3,
    });
    state
        .add_consumable(Consumable::Tarot(Tarot::Magician))
        .unwrap();

    assert!(consumable_available_in_pack(
        &state,
        Consumable::Tarot(Tarot::Fool),
        &[Consumable::Tarot(Tarot::Fool)]
    ));
    assert!(consumable_available_in_pack(
        &state,
        Consumable::Tarot(Tarot::Magician),
        &[Consumable::Tarot(Tarot::Magician)]
    ));
    assert!(consumable_available_in_pack(
        &state,
        Consumable::Planet(Planet::Mercury),
        &[Consumable::Planet(Planet::Mercury)]
    ));
}

#[test]
fn showman_allows_duplicates_for_every_pack_family() {
    let mut state = GameState::new(34);
    state.consumable_capacity = 4;
    state.jokers.add(Joker::create_joker(JokerKind::Showman));
    state.jokers.add(Joker::create_joker(JokerKind::Joker));
    state
        .add_consumable(Consumable::Tarot(Tarot::Fool))
        .unwrap();
    state
        .add_consumable(Consumable::Planet(Planet::Mercury))
        .unwrap();
    state
        .add_consumable(Consumable::Spectral(Spectral::Aura))
        .unwrap();

    assert!(consumable_available_in_pack(
        &state,
        Consumable::Tarot(Tarot::Fool),
        &[Consumable::Tarot(Tarot::Fool)]
    ));
    assert!(consumable_available_in_pack(
        &state,
        Consumable::Planet(Planet::Mercury),
        &[Consumable::Planet(Planet::Mercury)]
    ));
    assert!(consumable_available_in_pack(
        &state,
        Consumable::Spectral(Spectral::Aura),
        &[Consumable::Spectral(Spectral::Aura)]
    ));
    assert!(joker_available_in_pack(
        &state,
        JokerKind::Joker,
        &[JokerKind::Joker]
    ));

    let standard =
        ShopPack::new(BoosterPackKind::Standard, BoosterPackSize::Normal, 4).open(&mut state);
    assert!(
        standard
            .options()
            .iter()
            .all(|option| matches!(option, PackOption::PlayingCard(_)))
    );
}

#[test]
fn packs_do_not_repeat_consumables_without_showman() {
    let state = GameState::new(31);
    let card = Consumable::Spectral(Spectral::Aura);

    assert!(!consumable_available_in_pack(&state, card, &[card]));
}

#[test]
fn buffoon_packs_do_not_repeat_owned_or_visible_jokers_without_showman() {
    let mut state = GameState::new(33);
    state.jokers.add(Joker::create_joker(JokerKind::Joker));
    state.shop.add_item(ShopItem::Joker {
        joker: Joker::create_joker(JokerKind::Greedy),
        price: 4,
    });

    let opening =
        ShopPack::new(BoosterPackKind::Buffoon, BoosterPackSize::Mega, 8).open(&mut state);

    assert!(opening.options().iter().all(|option| {
        !matches!(
            option,
            PackOption::Joker(joker)
                if matches!(joker.kind(), JokerKind::Joker | JokerKind::Greedy)
        )
    }));
}

#[test]
fn soul_pack_probability_uses_the_0_3_percent_boundary() {
    assert!(soul_pack_roll_succeeds(0));
    assert!(soul_pack_roll_succeeds(2));
    assert!(!soul_pack_roll_succeeds(3));
    assert!(!soul_pack_roll_succeeds(SOUL_PACK_ROLL_DENOMINATOR - 1));
}
