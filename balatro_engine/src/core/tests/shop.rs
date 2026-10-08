use super::*;

use crate::core::card::Card;
use crate::core::enums::{
    Consumable, Decks, Edition, Enhancement, Rank, Seal, Suit, Tarot, Vouchers,
};
use crate::core::joker::Joker;
use crate::core::joker_types::JokerKind;
use crate::core::pack::{BoosterPackKind, BoosterPackSize, ShopPack};
use crate::core::shop::{
    SPECTRAL_CONSUMABLE_PRICE, ShopError, ShopItem, ShopOfferKind, ShopOfferWeights, ShopState,
    ShopVoucher, random_base_item,
};

fn playing_card() -> Card {
    Card::new(
        Rank::Ace,
        Suit::Spades,
        Enhancement::None,
        Edition::None,
        Seal::None,
    )
}

#[test]
fn shop_purchases_move_offers_into_game_state_and_charge_money() {
    let mut state = GameState::new(1);
    state.money = 20;
    let starting_deck_size = state.deck_size();
    let mut shop = ShopState::new();
    shop.add_item(ShopItem::PlayingCard {
        card: playing_card(),
        price: 3,
    });
    shop.add_item(ShopItem::Joker {
        joker: Joker::create_joker(JokerKind::Joker),
        price: 5,
    });
    shop.add_item(ShopItem::Consumable {
        consumable: Consumable::Tarot(Tarot::Magician),
        price: 2,
    });

    shop.buy_item(0, &mut state).unwrap();
    shop.buy_item(0, &mut state).unwrap();
    shop.buy_item(0, &mut state).unwrap();

    assert_eq!(state.money, 10);
    assert_eq!(state.deck_size(), starting_deck_size + 1);
    assert_eq!(state.jokers.as_slice().len(), 1);
    assert_eq!(state.consumables.len(), 1);
    assert!(shop.items.is_empty());
}

#[test]
fn failed_shop_purchase_keeps_the_offer_and_game_state_unchanged() {
    let mut state = GameState::new(2);
    state.money = 2;
    let mut shop = ShopState::new();
    shop.add_item(ShopItem::PlayingCard {
        card: playing_card(),
        price: 3,
    });

    let result = shop.buy_item(0, &mut state);

    assert_eq!(result, Err(ShopError::InsufficientFunds));
    assert_eq!(state.money, 2);
    assert_eq!(state.deck_size(), 52);
    assert_eq!(shop.items.len(), 1);
}

#[test]
fn shop_checks_joker_and_consumable_capacity_before_removing_an_offer() {
    let mut state = GameState::new(3);
    state.money = 20;
    for _ in 0..5 {
        state.jokers.add(Joker::create_joker(JokerKind::Joker));
    }
    let mut shop = ShopState::new();
    shop.add_item(ShopItem::Joker {
        joker: Joker::create_joker(JokerKind::Greedy),
        price: 5,
    });

    assert_eq!(shop.buy_item(0, &mut state), Err(ShopError::InventoryFull));
    assert_eq!(shop.items.len(), 1);
    assert_eq!(state.money, 20);

    let mut full_consumables_state = GameState::new(4);
    full_consumables_state.money = 20;
    full_consumables_state
        .add_consumable(Consumable::Tarot(Tarot::Magician))
        .unwrap();
    full_consumables_state
        .add_consumable(Consumable::Tarot(Tarot::Hermit))
        .unwrap();
    shop.items.clear();
    shop.add_item(ShopItem::Consumable {
        consumable: Consumable::Tarot(Tarot::World),
        price: 1,
    });

    assert_eq!(
        shop.buy_item(0, &mut full_consumables_state),
        Err(ShopError::InventoryFull)
    );
    assert_eq!(shop.items.len(), 1);
}

#[test]
fn vouchers_and_packs_are_single_shop_offers_with_atomic_purchases() {
    let mut state = GameState::new(4);
    state.money = 20;
    state.shop.set_voucher(ShopVoucher {
        voucher: Vouchers::Overstock,
        price: 8,
    });
    let mut shop = ShopState::with_reroll_cost(4);
    shop.add_pack(ShopPack {
        kind: BoosterPackKind::Arcana,
        size: BoosterPackSize::Normal,
        price: 6,
    })
    .unwrap();
    shop.add_pack(ShopPack {
        kind: BoosterPackKind::Standard,
        size: BoosterPackSize::Normal,
        price: 4,
    })
    .unwrap();

    state.buy_shop_voucher().unwrap();
    let bought_pack = shop.buy_pack(0, &mut state).unwrap();

    assert_eq!(bought_pack.kind, BoosterPackKind::Arcana);
    assert_eq!(state.vouchers, vec![Vouchers::Overstock]);
    assert_eq!(shop.packs.len(), 1);
    assert_eq!(state.money, 6);

    state.shop.set_voucher(ShopVoucher {
        voucher: Vouchers::Overstock,
        price: 1,
    });
    assert_eq!(
        state.buy_shop_voucher(),
        Err(ShopError::VoucherAlreadyOwned)
    );
    assert_eq!(state.money, 6);
    assert!(state.shop.voucher.is_some());
}

#[test]
fn reroll_charges_the_current_cost_refreshes_offers_and_increases_the_next_cost() {
    let mut state = GameState::new(5);
    state.money = 10;
    let mut shop = ShopState::new();
    shop.add_item(ShopItem::PlayingCard {
        card: playing_card(),
        price: 1,
    });

    shop.reroll(&mut state).unwrap();

    assert_eq!(state.money, 5);
    assert_eq!(state.counters.shop_rerolls, 1);
    assert_eq!(shop.items.len(), 2);
    assert_eq!(shop.reroll_cost, 6);
}

#[test]
fn reroll_vouchers_update_the_first_shop_cost_without_using_game_counters() {
    let mut state = GameState::new(6);

    state.redeem_voucher(Vouchers::RerollSurplus).unwrap();
    assert_eq!(state.shop.initial_reroll_cost, 3);

    state.redeem_voucher(Vouchers::RerollGlut).unwrap();
    assert_eq!(state.shop.initial_reroll_cost, 1);
}

#[test]
fn shop_refresh_uses_two_three_or_four_item_slots_from_overstock_vouchers() {
    let mut state = GameState::new(9);

    assert_eq!(state.shop.number_of_shop_items, 2);
    state.redeem_voucher(Vouchers::Overstock).unwrap();
    assert_eq!(state.shop.number_of_shop_items, 3);
    state.redeem_voucher(Vouchers::OverstockPlus).unwrap();
    assert_eq!(state.shop.number_of_shop_items, 4);
}

#[test]
fn game_owns_the_active_shop_phase_and_rejects_nested_shops() {
    let mut game = Game::new(6);
    assert!(game.shop().is_none());
    game.enter_shop().unwrap();
    assert_eq!(game.shop().unwrap().items.len(), 2);
    assert_eq!(game.shop().unwrap().packs.len(), 2);
    assert!(game.shop().unwrap().voucher.is_some());
    assert_eq!(game.enter_shop(), Err(ShopError::ShopAlreadyActive));
    assert!(game.leave_shop().is_some());
    assert!(game.shop().is_none());
}

#[test]
fn entering_a_shop_applies_the_first_reroll_cost_from_owned_vouchers() {
    let mut game = Game::new(14);
    game.state.redeem_voucher(Vouchers::RerollSurplus).unwrap();

    game.enter_shop().unwrap();

    assert_eq!(game.shop().unwrap().initial_reroll_cost, 3);
}

#[test]
fn ante_voucher_is_reused_by_each_shop_until_the_ante_changes() {
    let mut game = Game::new(30);
    let ante_one_voucher = game.state.ante_voucher.expect("ante 1 voucher");

    game.enter_shop().unwrap();
    assert_eq!(
        game.shop().unwrap().voucher.as_ref().unwrap().voucher,
        ante_one_voucher
    );
    game.leave_shop();

    game.enter_shop().unwrap();
    assert_eq!(
        game.shop().unwrap().voucher.as_ref().unwrap().voucher,
        ante_one_voucher
    );
}

#[test]
fn advancing_to_the_next_ante_rolls_a_new_eligible_voucher() {
    let mut game = Game::new(31);
    game.state.money = 10;
    let ante_one_voucher = game.state.ante_voucher.expect("ante 1 voucher");

    game.enter_shop().unwrap();
    game.state.redeem_voucher(ante_one_voucher).unwrap();
    game.leave_shop().expect("active shop");

    game.state.begin_next_ante();
    assert_eq!(game.state.counters.ante, 2);
    let ante_two_voucher = game.state.ante_voucher.expect("ante 2 voucher");
    assert_ne!(ante_two_voucher, ante_one_voucher);
    assert!(game.state.can_redeem_voucher(ante_two_voucher));

    game.enter_shop().unwrap();
    assert_eq!(
        game.shop().unwrap().voucher.as_ref().unwrap().voucher,
        ante_two_voucher
    );
}

#[test]
fn game_state_can_start_with_a_non_red_deck_for_shop_card_purchases() {
    let mut state = GameState::with_deck(7, Decks::Black);
    state.money = 10;
    let mut shop = ShopState::new();
    shop.add_item(ShopItem::PlayingCard {
        card: playing_card(),
        price: 1,
    });

    shop.buy_item(0, &mut state).unwrap();

    assert_eq!(state.hand_size(), 7);
    assert_eq!(state.deck_size(), 53);
}

#[test]
fn base_shop_offer_weights_use_the_20_4_4_distribution() {
    let weights = ShopOfferWeights::BASE;

    assert_eq!(weights.total(), 28);
    assert_eq!(weights.kind_for_roll(0), ShopOfferKind::Joker);
    assert_eq!(weights.kind_for_roll(19), ShopOfferKind::Joker);
    assert_eq!(weights.kind_for_roll(20), ShopOfferKind::Tarot);
    assert_eq!(weights.kind_for_roll(23), ShopOfferKind::Tarot);
    assert_eq!(weights.kind_for_roll(24), ShopOfferKind::Planet);
    assert_eq!(weights.kind_for_roll(27), ShopOfferKind::Planet);
}

#[test]
fn base_shop_offer_roll_uses_the_game_rng() {
    let mut first_state = GameState::new(8);
    let mut second_state = GameState::new(8);
    let first_shop = ShopState::new();
    let second_shop = ShopState::new();
    let first = (0..20)
        .map(|_| first_shop.roll_base_offer_kind(&mut first_state))
        .collect::<Vec<_>>();
    let second = (0..20)
        .map(|_| second_shop.roll_base_offer_kind(&mut second_state))
        .collect::<Vec<_>>();

    assert_eq!(first, second);
}

#[test]
fn ghost_deck_and_magic_trick_add_their_shop_offer_categories() {
    let ghost_state = GameState::with_deck(10, Decks::Ghost);
    let mut ghost_shop = ShopState::new();
    ghost_shop.apply_deck_rules(ghost_state.deck.deck_type());
    assert!(ghost_shop.offer_weights.spectrals > 0);
    assert_eq!(ghost_shop.offer_weights.cards, 0);

    let mut magic_state = GameState::new(11);
    magic_state.redeem_voucher(Vouchers::MagicTrick).unwrap();
    assert_eq!(magic_state.shop.offer_weights.spectrals, 0);
    assert!(magic_state.shop.offer_weights.cards > 0);
}

#[test]
fn merchant_and_tycoon_vouchers_scale_their_shop_weights() {
    let mut state = GameState::new(13);
    state.redeem_voucher(Vouchers::TarotMerchant).unwrap();
    state.redeem_voucher(Vouchers::PlanetMerchant).unwrap();
    assert_eq!(state.shop.offer_weights.tarot, 8);
    assert_eq!(state.shop.offer_weights.planets, 8);

    state.redeem_voucher(Vouchers::TarotTycoon).unwrap();
    state.redeem_voucher(Vouchers::PlanetTycoon).unwrap();
    assert_eq!(state.shop.offer_weights.tarot, 32);
    assert_eq!(state.shop.offer_weights.planets, 32);
}

#[test]
fn buying_overstock_replenishes_the_new_shop_slot() {
    let mut state = GameState::new(12);
    state.money = 10;
    state.shop.add_item(ShopItem::PlayingCard {
        card: playing_card(),
        price: 1,
    });
    state.shop.add_item(ShopItem::PlayingCard {
        card: playing_card(),
        price: 1,
    });
    state.shop.set_voucher(ShopVoucher {
        voucher: Vouchers::Overstock,
        price: 5,
    });

    state.buy_shop_voucher().unwrap();

    assert_eq!(state.shop.number_of_shop_items, 3);
    assert_eq!(state.shop.items.len(), 3);
    assert!(state.vouchers.contains(&Vouchers::Overstock));
}

#[test]
fn shop_consumable_prices_match_their_card_types() {
    let mut state = GameState::new(14);

    let tarot = random_base_item(ShopOfferKind::Tarot, &mut state);
    let planet = random_base_item(ShopOfferKind::Planet, &mut state);
    let spectral = random_base_item(ShopOfferKind::Spectral, &mut state);

    assert_eq!(tarot.price(), 3);
    assert_eq!(planet.price(), 3);
    assert_eq!(spectral.price(), SPECTRAL_CONSUMABLE_PRICE);
}
