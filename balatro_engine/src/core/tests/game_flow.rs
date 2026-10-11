use super::*;

use crate::core::blind::{BlindKind, BlindSkipError};
use crate::core::card::Card;
use crate::core::consumable::ConsumableTarget;
use crate::core::enums::{
    Consumable, Edition, Enhancement, Rank, Seal, SkipTag, Suit, Tarot, Vouchers,
};
use crate::core::enums::{Decks, Stakes};
use crate::core::game::{ActionError, DecisionError, DecisionPoint, Game, GameAction};
use crate::core::joker::Joker;
use crate::core::joker_types::JokerKind;
use crate::core::pack::{BoosterPackKind, BoosterPackSize, ShopPack};
use crate::core::shop::{ShopItem, ShopVoucher};
use std::collections::HashSet;

#[test]
fn new_red_white_game_starts_with_a_complete_deck_and_blind_selector() {
    let state = GameState::new(12345);

    assert_eq!(state.deck.deck_type(), Decks::Red);
    assert_eq!(state.deck.size(), 52);
    assert_eq!(state.deck.starting_size(), 52);
    assert_eq!(state.deck.discarded_cards().len(), 0);

    let card_ids = state
        .deck
        .cards()
        .iter()
        .map(crate::core::card::Card::id)
        .collect::<HashSet<_>>();
    assert_eq!(card_ids.len(), 52);

    assert_eq!(state.blind.stake, Stakes::White);
    assert_eq!(state.blind.hands_per_blind, 4);
    assert_eq!(state.blind.discards_per_blind, 4);
    assert_eq!(state.blind.hands_remaining, 4);
    assert_eq!(state.blind.discards_remaining, 4);
    assert_eq!(state.counters.ante, 1);

    let selector = state.blind_selector;
    assert_eq!(selector.small().kind, BlindKind::Small);
    assert_eq!(selector.big().kind, BlindKind::Big);
    assert!(matches!(selector.boss().kind, BlindKind::Boss(_)));
    assert!(selector.small().skip_tag.is_some());
    assert!(selector.big().skip_tag.is_some());
    assert_eq!(selector.boss().skip_tag, None);
}

#[test]
fn new_game_waits_at_blind_selection_before_dealing_cards() {
    let mut game = Game::new(12345);

    assert_eq!(game.next_step(), DecisionPoint::BlindSelection);
    assert_eq!(game.decision_point(), DecisionPoint::BlindSelection);
    assert_eq!(game.hand().hand_size(), 0);
    assert_eq!(game.state().deck.size(), 52);
    assert_eq!(game.state().blind_selector.small().kind, BlindKind::Small);
    assert_eq!(game.state().blind_selector.big().kind, BlindKind::Big);
    assert!(matches!(
        game.state().blind_selector.boss().kind,
        BlindKind::Boss(_)
    ));
}

#[test]
fn decision_points_use_stable_integer_codes() {
    assert_eq!(DecisionPoint::BlindSelection.code(), 0);
    assert_eq!(DecisionPoint::PlayingHand.code(), 1);
    assert_eq!(DecisionPoint::Shop.code(), 2);
    assert_eq!(DecisionPoint::PackSelection.code(), 3);
    assert_eq!(DecisionPoint::ConsumableSelection.code(), 4);
    assert_eq!(DecisionPoint::RunComplete.code(), 5);
}

#[test]
fn selecting_small_blind_starts_playing_with_a_dealt_hand() {
    let mut game = Game::new(12345);

    game.select_blind(BlindKind::Small).unwrap();

    assert_eq!(game.next_step(), DecisionPoint::PlayingHand);
    assert_eq!(game.decision_point(), DecisionPoint::PlayingHand);
    assert_eq!(game.state().blind.kind, BlindKind::Small);
    assert_eq!(game.state().blind.hands_remaining, 4);
    assert_eq!(game.state().blind.discards_remaining, 4);
    assert_eq!(game.hand().hand_size(), 8);
    assert_eq!(game.state().deck.size(), 44);
}

#[test]
fn each_generated_blind_can_be_selected_and_deals_a_hand() {
    let mut small = Game::new(31);
    assert_eq!(
        small.apply_action(GameAction::SelectBlind(BlindKind::Small)),
        Ok(DecisionPoint::PlayingHand)
    );

    let mut big = Game::new(32);
    assert_eq!(
        big.apply_action(GameAction::SelectBlind(BlindKind::Big)),
        Ok(DecisionPoint::PlayingHand)
    );

    let mut boss = Game::new(33);
    let boss_kind = boss.state().blind_selector.boss().kind;
    assert!(boss_kind.is_boss());
    assert_eq!(
        boss.apply_action(GameAction::SelectBlind(boss_kind)),
        Ok(DecisionPoint::PlayingHand)
    );
    assert_eq!(boss.hand().hand_size(), 8);
}

#[test]
fn a_blind_cannot_be_selected_after_playing_has_started() {
    let mut game = Game::new(12345);
    game.select_blind(BlindKind::Small).unwrap();

    assert_eq!(
        game.select_blind(BlindKind::Big),
        Err(DecisionError::InvalidPhase)
    );
}

#[test]
fn action_dispatch_covers_skip_play_and_discard_decisions() {
    let mut skipped = Game::new(21);
    let small = skipped.state().blind_selector.small().kind;
    assert_eq!(
        skipped.apply_action(GameAction::SkipBlind(small)),
        Ok(DecisionPoint::BlindSelection)
    );

    let mut discarded = Game::new(22);
    discarded
        .apply_action(GameAction::SelectBlind(BlindKind::Small))
        .unwrap();
    assert_eq!(
        discarded.apply_action(GameAction::DiscardCards(vec![0])),
        Ok(DecisionPoint::PlayingHand)
    );
    assert_eq!(discarded.state().blind.discards_remaining, 3);

    let mut played = Game::new(23);
    played
        .apply_action(GameAction::SelectBlind(BlindKind::Small))
        .unwrap();
    assert_eq!(
        played.apply_action(GameAction::PlayCards(vec![0])),
        Ok(DecisionPoint::PlayingHand)
    );
    assert_eq!(played.state().blind.hands_remaining, 3);
}

#[test]
fn playing_and_discarding_are_limited_to_five_cards() {
    let mut game = Game::new(26);
    game.apply_action(GameAction::SelectBlind(BlindKind::Small))
        .unwrap();

    assert_eq!(
        game.apply_action(GameAction::PlayCards(vec![0, 1, 2, 3, 4, 5])),
        Err(ActionError::InvalidAction)
    );
    assert_eq!(game.state().blind.hands_remaining, 4);

    assert_eq!(
        game.apply_action(GameAction::DiscardCards(vec![0, 1, 2, 3, 4, 5])),
        Err(ActionError::InvalidAction)
    );
    assert_eq!(game.state().blind.discards_remaining, 4);
}

#[test]
fn winning_a_blind_transitions_to_the_shop() {
    let mut game = Game::new(34);
    game.apply_action(GameAction::SelectBlind(BlindKind::Small))
        .unwrap();
    game.state.blind.score_requirement = 0;

    assert_eq!(
        game.apply_action(GameAction::PlayCards(vec![0])),
        Ok(DecisionPoint::Shop)
    );
    assert!(game.shop().is_some());
    assert!(game.state.blind.finished);
}

#[test]
fn running_out_of_hands_enters_the_terminal_decision() {
    let mut game = Game::new(35);
    game.apply_action(GameAction::SelectBlind(BlindKind::Small))
        .unwrap();
    game.state.blind.hands_remaining = 1;
    game.state.blind.score_requirement = u64::MAX;

    assert_eq!(
        game.apply_action(GameAction::PlayCards(vec![0])),
        Ok(DecisionPoint::RunComplete)
    );
    assert_eq!(game.next_step(), DecisionPoint::RunComplete);
}

#[test]
fn boss_blinds_cannot_be_skipped() {
    let mut game = Game::new(36);
    let boss = game.state().blind_selector.boss().kind;

    assert_eq!(
        game.apply_action(GameAction::SkipBlind(boss)),
        Err(ActionError::BlindSkip(
            BlindSkipError::BossBlindCannotBeSkipped
        ))
    );
    assert_eq!(game.decision_point(), DecisionPoint::BlindSelection);
}

#[test]
fn skipped_tag_packs_use_the_same_selection_with_a_blind_return_point() {
    let mut game = Game::new(37);
    game.state
        .blind_selector
        .set_small_skip_tag(SkipTag::Ethereal);

    assert_eq!(
        game.apply_action(GameAction::SkipBlind(BlindKind::Small)),
        Ok(DecisionPoint::PackSelection)
    );
    assert_eq!(
        game.pending_pack.as_ref().unwrap().previous_state,
        DecisionPoint::BlindSelection
    );
    assert_eq!(game.next_step(), DecisionPoint::PackSelection);

    assert_eq!(
        game.apply_action(GameAction::SelectPackOptions(vec![0])),
        Ok(DecisionPoint::BlindSelection)
    );
    assert!(game.state.tag_pack_openings.is_empty());
}

#[test]
fn shop_packs_store_shop_as_the_previous_state() {
    let mut game = Game::new(38);
    game.state.money = 100;
    game.enter_shop().unwrap();
    game.shop_mut().unwrap().packs.clear();
    game.shop_mut()
        .unwrap()
        .add_pack(ShopPack::new(
            BoosterPackKind::Standard,
            BoosterPackSize::Normal,
            1,
        ))
        .unwrap();

    assert_eq!(
        game.apply_action(GameAction::BuyPack(0)),
        Ok(DecisionPoint::PackSelection)
    );
    assert_eq!(
        game.pending_pack.as_ref().unwrap().previous_state,
        DecisionPoint::Shop
    );

    assert_eq!(
        game.apply_action(GameAction::SelectPackOptions(vec![0])),
        Ok(DecisionPoint::Shop)
    );
}

#[test]
fn action_dispatch_covers_shop_and_pack_decisions() {
    let mut game = Game::new(24);
    game.state.money = 100;
    game.enter_shop().unwrap();
    game.shop_mut().unwrap().items.clear();
    game.shop_mut().unwrap().add_item(ShopItem::PlayingCard {
        card: Card::new(
            Rank::Ace,
            Suit::Spades,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ),
        price: 1,
    });
    assert_eq!(
        game.apply_action(GameAction::BuyShopItem(0)),
        Ok(DecisionPoint::Shop)
    );

    game.shop_mut().unwrap().set_voucher(ShopVoucher {
        voucher: Vouchers::ClearanceSale,
        price: 1,
    });
    assert_eq!(
        game.apply_action(GameAction::BuyVoucher(0)),
        Ok(DecisionPoint::Shop)
    );
    game.shop_mut().unwrap().extra_vouchers.push(ShopVoucher {
        voucher: Vouchers::Blank,
        price: 1,
    });
    assert_eq!(
        game.apply_action(GameAction::BuyVoucher(1)),
        Ok(DecisionPoint::Shop)
    );
    assert_eq!(
        game.apply_action(GameAction::RerollShop),
        Ok(DecisionPoint::Shop)
    );

    game.state
        .add_consumable(Consumable::Tarot(Tarot::Hermit))
        .unwrap();
    game.state.jokers.add(Joker::create_joker(JokerKind::Joker));
    assert_eq!(
        game.apply_action(GameAction::SellJoker(0)),
        Ok(DecisionPoint::Shop)
    );
    assert_eq!(
        game.apply_action(GameAction::SellConsumable(0)),
        Ok(DecisionPoint::Shop)
    );
    game.state
        .add_consumable(Consumable::Tarot(Tarot::Hermit))
        .unwrap();
    assert_eq!(
        game.apply_action(GameAction::UseConsumable {
            index: 0,
            target: ConsumableTarget::default(),
        }),
        Ok(DecisionPoint::Shop)
    );

    game.shop_mut().unwrap().packs.clear();
    game.shop_mut()
        .unwrap()
        .add_pack(ShopPack::new(
            BoosterPackKind::Standard,
            BoosterPackSize::Normal,
            1,
        ))
        .unwrap();
    assert_eq!(
        game.apply_action(GameAction::BuyPack(0)),
        Ok(DecisionPoint::PackSelection)
    );
    assert_eq!(
        game.apply_action(GameAction::SelectPackOptions(vec![0])),
        Ok(DecisionPoint::Shop)
    );
    assert_eq!(
        game.apply_action(GameAction::LeaveShop),
        Ok(DecisionPoint::BlindSelection)
    );
}

#[test]
fn leaving_the_shop_materializes_perkeos_negative_consumable_copy() {
    let mut game = Game::new(27);
    game.state
        .add_consumable(Consumable::Tarot(Tarot::Hermit))
        .unwrap();
    game.state
        .jokers
        .add(Joker::create_joker(JokerKind::Perkeo));
    game.enter_shop().unwrap();

    assert_eq!(
        game.apply_action(GameAction::LeaveShop),
        Ok(DecisionPoint::BlindSelection)
    );
    assert_eq!(game.state.consumables.len(), 2);
    assert!(game.state.consumables.iter().any(|consumable| {
        consumable.negative && consumable.consumable == Consumable::Tarot(Tarot::Hermit)
    }));
}

#[test]
fn action_dispatch_rejects_actions_from_the_wrong_decision_point() {
    let mut game = Game::new(25);

    assert_eq!(
        game.apply_action(GameAction::DiscardCards(vec![0])),
        Err(ActionError::InvalidPhase)
    );
    assert_eq!(
        game.apply_action(GameAction::LeaveShop),
        Err(ActionError::InvalidPhase)
    );
}
