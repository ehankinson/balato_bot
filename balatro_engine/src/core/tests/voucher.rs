use super::*;

use crate::core::enums::{Consumable, Edition, Planet, PokerHand, Vouchers};
use crate::core::joker_types::JokerEdition;
use crate::core::voucher::{VoucherError, shop_edition_for_roll, standard_pack_edition_for_roll};

#[test]
fn voucher_redemption_updates_persistent_run_modifiers() {
    let mut state = GameState::new(21);

    state.redeem_voucher(Vouchers::CrystalBall).unwrap();
    state.redeem_voucher(Vouchers::Grabber).unwrap();
    state.redeem_voucher(Vouchers::Wasteful).unwrap();
    state.redeem_voucher(Vouchers::SeedMoney).unwrap();
    state.redeem_voucher(Vouchers::Blank).unwrap();
    state.redeem_voucher(Vouchers::Antimatter).unwrap();

    assert_eq!(state.consumable_capacity(), 3);
    assert_eq!(state.hands_per_blind(), 5);
    assert_eq!(state.discards_per_blind(), 5);
    assert_eq!(state.counters.interest_cap, 10);
    assert_eq!(state.joker_capacity(), 6);
    assert_eq!(state.counters.empty_joker_slots, 6);

    state.begin_blind();
    assert_eq!(state.counters.hands_remaining, 5);
    assert_eq!(state.counters.discards_remaining, 5);
}

#[test]
fn voucher_discount_upgrades_and_duplicate_redemption_are_atomic() {
    let mut state = GameState::new(22);

    state.redeem_voucher(Vouchers::ClearanceSale).unwrap();
    assert_eq!(state.shop_price(4), 3);
    state.redeem_voucher(Vouchers::Liquidation).unwrap();
    assert_eq!(state.shop_price(4), 2);
    state.redeem_voucher(Vouchers::Hone).unwrap();
    state.redeem_voucher(Vouchers::GlowUp).unwrap();
    assert_eq!(state.counters.edition_rate_multiplier, 4);

    assert_eq!(
        state.redeem_voucher(Vouchers::GlowUp),
        Err(VoucherError::AlreadyOwned)
    );
    assert_eq!(state.vouchers.len(), 4);
}

#[test]
fn voucher_ante_and_hand_modifiers_apply_through_game_state() {
    let mut state = GameState::new(23);
    state.redeem_voucher(Vouchers::Hieroglyph).unwrap();

    assert_eq!(state.counters.ante, 0);
    assert_eq!(state.hands_per_blind(), 3);

    state.redeem_voucher(Vouchers::Petroglpyh).unwrap();
    state.redeem_voucher(Vouchers::PaintBrush).unwrap();
    state.redeem_voucher(Vouchers::Palette).unwrap();
    assert_eq!(state.counters.ante, 0);
    assert_eq!(state.hand_size(), 9);
}

#[test]
fn observatory_scales_planet_upgrades() {
    let mut state = GameState::new(24);
    state.redeem_voucher(Vouchers::Telescope).unwrap();
    state.redeem_voucher(Vouchers::Observatory).unwrap();
    state
        .add_consumable(Consumable::Planet(Planet::Saturn))
        .unwrap();

    assert!(state.always_show_most_played_hand);
    assert_eq!(state.planet_mult_multiplier(PokerHand::Straight), 1.5);
    assert_eq!(state.planet_mult_multiplier(PokerHand::Flush), 1.0);

    state
        .use_consumable(0, &mut Hand::new(), ConsumableTarget::default())
        .unwrap();
    assert_eq!(state.planet_mult_multiplier(PokerHand::Straight), 1.0);
}

#[test]
fn voucher_prerequisites_are_enforced() {
    let mut state = GameState::new(25);
    assert_eq!(
        state.redeem_voucher(Vouchers::Antimatter),
        Err(VoucherError::PrerequisiteMissing)
    );
    state.redeem_voucher(Vouchers::Blank).unwrap();
    state.redeem_voucher(Vouchers::Antimatter).unwrap();
}

#[test]
fn telescope_and_consumable_slot_vouchers_update_game_state() {
    let mut state = GameState::new(26);
    assert!(!state.always_show_most_played_hand);
    state.redeem_voucher(Vouchers::Telescope).unwrap();
    assert!(state.always_show_most_played_hand);

    state.redeem_voucher(Vouchers::CrystalBall).unwrap();
    state.redeem_voucher(Vouchers::OmenGlobe).unwrap();
    assert_eq!(state.consumable_capacity(), 4);
}

#[test]
fn shop_edition_rates_scale_positive_editions_but_not_negative() {
    assert_eq!(shop_edition_for_roll(0, 1), JokerEdition::Negative);
    assert_eq!(shop_edition_for_roll(30, 1), JokerEdition::Polychrome);
    assert_eq!(shop_edition_for_roll(59, 1), JokerEdition::Polychrome);
    assert_eq!(shop_edition_for_roll(60, 1), JokerEdition::Holographic);
    assert_eq!(shop_edition_for_roll(199, 1), JokerEdition::Holographic);
    assert_eq!(shop_edition_for_roll(200, 1), JokerEdition::Foil);
    assert_eq!(shop_edition_for_roll(399, 1), JokerEdition::Foil);

    assert_eq!(shop_edition_for_roll(30, 2), JokerEdition::Polychrome);
    assert_eq!(shop_edition_for_roll(89, 2), JokerEdition::Polychrome);
    assert_eq!(shop_edition_for_roll(90, 2), JokerEdition::Holographic);
    assert_eq!(shop_edition_for_roll(369, 2), JokerEdition::Holographic);
    assert_eq!(shop_edition_for_roll(370, 2), JokerEdition::Foil);
    assert_eq!(shop_edition_for_roll(769, 2), JokerEdition::Foil);
    assert_eq!(shop_edition_for_roll(30, 4), JokerEdition::Polychrome);
    assert_eq!(shop_edition_for_roll(149, 4), JokerEdition::Polychrome);
    assert_eq!(shop_edition_for_roll(150, 4), JokerEdition::Holographic);
    assert_eq!(shop_edition_for_roll(709, 4), JokerEdition::Holographic);
    assert_eq!(shop_edition_for_roll(710, 4), JokerEdition::Foil);
    assert_eq!(shop_edition_for_roll(1509, 4), JokerEdition::Foil);
}

#[test]
fn standard_pack_edition_rates_use_the_pack_table_and_replace_hone_with_glow_up() {
    assert_eq!(standard_pack_edition_for_roll(0, 1), Edition::Polychrome);
    assert_eq!(standard_pack_edition_for_roll(119, 1), Edition::Polychrome);
    assert_eq!(standard_pack_edition_for_roll(120, 1), Edition::Holographic);
    assert_eq!(standard_pack_edition_for_roll(399, 1), Edition::Holographic);
    assert_eq!(standard_pack_edition_for_roll(400, 1), Edition::Foil);
    assert_eq!(standard_pack_edition_for_roll(799, 1), Edition::Foil);
    assert_eq!(standard_pack_edition_for_roll(800, 1), Edition::None);

    assert_eq!(standard_pack_edition_for_roll(1599, 2), Edition::Foil);
    assert_eq!(standard_pack_edition_for_roll(1600, 2), Edition::None);
    assert_eq!(standard_pack_edition_for_roll(3199, 4), Edition::Foil);
    assert_eq!(standard_pack_edition_for_roll(3200, 4), Edition::None);
}
