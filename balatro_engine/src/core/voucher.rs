use crate::core::enums::{Edition, Vouchers};
use crate::core::game::GameState;
use crate::core::joker_types::JokerEdition;
use rand::prelude::RngExt;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum VoucherError {
    AlreadyOwned,
    PrerequisiteMissing,
}

pub(crate) fn redeem(state: &mut GameState, voucher: Vouchers) -> Result<(), VoucherError> {
    if state.vouchers.contains(&voucher) {
        return Err(VoucherError::AlreadyOwned);
    }
    if !can_redeem(state, voucher) {
        return Err(VoucherError::PrerequisiteMissing);
    }

    apply_effect(state, voucher);
    state.vouchers.push(voucher);
    Ok(())
}

pub(crate) fn can_redeem(state: &GameState, voucher: Vouchers) -> bool {
    if state.vouchers.contains(&voucher) {
        return false;
    }

    match voucher {
        Vouchers::OverstockPlus => state.has_voucher(Vouchers::Overstock),
        Vouchers::Liquidation => state.has_voucher(Vouchers::ClearanceSale),
        Vouchers::GlowUp => state.has_voucher(Vouchers::Hone),
        Vouchers::RerollGlut => state.has_voucher(Vouchers::RerollSurplus),
        Vouchers::OmenGlobe => state.has_voucher(Vouchers::CrystalBall),
        Vouchers::Observatory => state.has_voucher(Vouchers::Telescope),
        Vouchers::NachoTong => state.has_voucher(Vouchers::Grabber),
        Vouchers::Recyclomancy => state.has_voucher(Vouchers::Wasteful),
        Vouchers::TarotTycoon => state.has_voucher(Vouchers::TarotMerchant),
        Vouchers::PlanetTycoon => state.has_voucher(Vouchers::PlanetMerchant),
        Vouchers::MoneyTree => state.has_voucher(Vouchers::SeedMoney),
        Vouchers::Antimatter => state.has_voucher(Vouchers::Blank),
        Vouchers::Illusion => state.has_voucher(Vouchers::MagicTrick),
        Vouchers::Petroglpyh => state.has_voucher(Vouchers::Hieroglyph),
        Vouchers::Retcon => state.has_voucher(Vouchers::DirectorsCut),
        Vouchers::Palette => state.has_voucher(Vouchers::PaintBrush),
        _ => true,
    }
}

const SHOP_NEGATIVE_RATE: u32 = 30;
const SHOP_POLYCHROME_RATE: u32 = 30;
const SHOP_HOLOGRAPHIC_RATE: u32 = 140;
const SHOP_FOIL_RATE: u32 = 200;
const SHOP_EDITION_ROLL_MAX: u32 = 10_000;

pub(crate) fn random_shop_joker_edition(state: &mut GameState) -> JokerEdition {
    let multiplier = state.edition_rate_multiplier as u32;
    let roll = state.rng.random_range(0..SHOP_EDITION_ROLL_MAX);
    shop_edition_for_roll(roll, multiplier)
}

const STANDARD_PACK_POLYCHROME_RATE: u32 = 120;
const STANDARD_PACK_HOLOGRAPHIC_RATE: u32 = 280;
const STANDARD_PACK_FOIL_RATE: u32 = 400;

pub(crate) fn random_standard_pack_edition(state: &mut GameState) -> Edition {
    let multiplier = state.edition_rate_multiplier as u32;
    let roll = state.rng.random_range(0..SHOP_EDITION_ROLL_MAX);
    standard_pack_edition_for_roll(roll, multiplier)
}

pub(crate) fn standard_pack_edition_for_roll(roll: u32, multiplier: u32) -> Edition {
    let polychrome = STANDARD_PACK_POLYCHROME_RATE * multiplier;
    let holographic = STANDARD_PACK_HOLOGRAPHIC_RATE * multiplier;
    let foil = STANDARD_PACK_FOIL_RATE * multiplier;

    if roll < polychrome {
        Edition::Polychrome
    } else if roll < polychrome + holographic {
        Edition::Holographic
    } else if roll < polychrome + holographic + foil {
        Edition::Foil
    } else {
        Edition::None
    }
}

pub(crate) fn shop_edition_for_roll(roll: u32, multiplier: u32) -> JokerEdition {
    let polychrome = SHOP_POLYCHROME_RATE * multiplier;
    let holographic = SHOP_HOLOGRAPHIC_RATE * multiplier;
    let foil = SHOP_FOIL_RATE * multiplier;

    if roll < SHOP_NEGATIVE_RATE {
        JokerEdition::Negative
    } else if roll < SHOP_NEGATIVE_RATE + polychrome {
        JokerEdition::Polychrome
    } else if roll < SHOP_NEGATIVE_RATE + polychrome + holographic {
        JokerEdition::Holographic
    } else if roll < SHOP_NEGATIVE_RATE + polychrome + holographic + foil {
        JokerEdition::Foil
    } else {
        JokerEdition::None
    }
}

fn apply_effect(state: &mut GameState, voucher: Vouchers) {
    match voucher {
        Vouchers::ClearanceSale => {
            state.shop_discount_percent = 25;
            state.apply_shop_discount_to_owned_items();
        }
        Vouchers::Liquidation => {
            state.shop_discount_percent = 50;
            state.apply_shop_discount_to_owned_items();
        }
        Vouchers::Hone => state.edition_rate_multiplier = 2,
        Vouchers::GlowUp => state.edition_rate_multiplier = 4,
        Vouchers::RerollSurplus => state.shop.initial_reroll_cost = 3,
        Vouchers::RerollGlut => state.shop.initial_reroll_cost = 1,
        Vouchers::CrystalBall | Vouchers::OmenGlobe => {
            state.consumable_capacity = state.consumable_capacity.saturating_add(1);
        }
        Vouchers::Grabber | Vouchers::NachoTong => state.blind.adjust_hands_per_blind(1),
        Vouchers::Wasteful | Vouchers::Recyclomancy => state.blind.adjust_discards_per_blind(1),
        Vouchers::SeedMoney => state.interest_cap = state.interest_cap.max(10),
        Vouchers::MoneyTree => state.interest_cap = state.interest_cap.max(20),
        Vouchers::Antimatter => state.jokers.increase_capacity(1),
        Vouchers::Hieroglyph => {
            state.counters.ante = state.counters.ante.saturating_sub(1);
            state.blind.adjust_hands_per_blind(-1);
        }
        Vouchers::Petroglpyh => {
            state.counters.ante = state.counters.ante.saturating_sub(1);
            state.blind.adjust_discards_per_blind(-1);
        }
        Vouchers::DirectorsCut => state.boss_rerolls_per_ante = 1,
        Vouchers::Retcon => state.boss_reroll_unlimited = true,
        Vouchers::PaintBrush => state.deck.adjust_hand_size(1),
        Vouchers::Palette => state.deck.adjust_hand_size(1),
        Vouchers::Overstock | Vouchers::OverstockPlus => {
            state.shop.number_of_shop_items = state.shop.number_of_shop_items.saturating_add(1);
        }
        Vouchers::TarotMerchant => state.shop.offer_weights.multiply_tarot_weight(2),
        Vouchers::TarotTycoon => state.shop.offer_weights.multiply_tarot_weight(4),
        Vouchers::PlanetMerchant => state.shop.offer_weights.multiply_planet_weight(2),
        Vouchers::PlanetTycoon => state.shop.offer_weights.multiply_planet_weight(4),
        Vouchers::MagicTrick => state.shop.offer_weights.cards = 2,
        Vouchers::Telescope => state.shop.most_played_planet_in_pack = true,
        Vouchers::Blank | Vouchers::Observatory => {}
        Vouchers::Illusion => {
            state.shop.playing_card_edition_chance = 20;
            state.shop.playing_card_enhancement_chance = 40;
        }
    }
}
