use super::*;

use crate::core::enums::{Consumable, Planet, PokerHand};
use crate::core::pack::{
    BoosterPackKind, BoosterPackSize, PackError, PackOption, SHOP_PACK_WEIGHTS, ShopPack,
};
use crate::core::shop::ShopState;

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
    let opening = pack.open(&mut state);

    assert_eq!(opening.options().len(), 3);
    assert!(matches!(opening.options()[0], PackOption::PlayingCard(_)));
    state.choose_pack(opening, &[0]).unwrap();

    assert_eq!(state.deck_size(), starting_size + 1);
}

#[test]
fn pack_selection_is_atomic_when_the_selection_is_invalid() {
    let mut state = GameState::new(26);
    let pack = ShopPack::new(BoosterPackKind::Buffoon, BoosterPackSize::Mega, 8);
    let opening = pack.open(&mut state);

    assert_eq!(
        state.choose_pack(opening, &[0]),
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
fn telescope_uses_the_most_played_hand_for_celestial_pack_planets() {
    let mut state = GameState::new(28);
    state.most_played_hand = Some(PokerHand::Pair);
    state.shop.most_played_planet_in_pack = true;

    let pack = ShopPack::new(BoosterPackKind::Celestial, BoosterPackSize::Normal, 4);
    let opening = pack.open(&mut state);

    assert!(opening.options().iter().all(|option| matches!(
        option,
        PackOption::Consumable(Consumable::Planet(Planet::Mercury))
    )));
}
