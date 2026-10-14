use super::*;
use crate::core::enums::{Rank, Suit};
use crate::core::joker_types::JokerKind;
use crate::core::pack::{BoosterPackKind, BoosterPackSize, ShopPack};

fn shop_game(kinds: &[JokerKind]) -> Game {
    let mut g = Game::new(803);
    g.state.money = 100;
    for &kind in kinds {
        g.state.jokers.add(Joker::create_joker(kind));
    }
    g.enter_shop().unwrap();
    g
}
fn pack(g: &mut Game, kind: BoosterPackKind, size: BoosterPackSize) {
    g.state.shop.packs.clear();
    g.state.shop.packs.push(ShopPack::new(kind, size, 0));
    g.apply_action(GameAction::BuyPack(0)).unwrap();
}
fn score_mult(g: &mut Game) -> f32 {
    g.state.begin_blind_as(BlindKind::Small);
    g.state.blind.score_requirement = u64::MAX;
    let mut h = Hand::new();
    h.add_card(Card::new(
        Rank::Two,
        Suit::Clubs,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));
    g.state.play_hand(&mut h, vec![0]).unwrap().mult
}

#[test]
fn pk03_bean_remaining_bonus_after_one_round_is_four() {
    let mut g = shop_game(&[JokerKind::TurtleBean]);
    g.state.begin_blind_as(BlindKind::Small);
    g.state.blind.chips_scored = g.state.blind.score_requirement;
    g.state.finish_blind(&Hand::new()).unwrap();
    assert_eq!(g.state.hand_size(), 12);
}

#[test]
fn pk04_bean_expiry_removes_it_once_and_does_not_keep_shrinking_hand_size() {
    let mut g = shop_game(&[JokerKind::TurtleBean]);
    for round in 0..7 {
        g.state.begin_blind_as(BlindKind::Small);
        g.state.blind.chips_scored = g.state.blind.score_requirement;
        g.state.finish_blind(&Hand::new()).unwrap();
        assert_eq!(g.state.hand_size(), 8 + (4i8 - round).max(0));
    }
    assert!(g.state.jokers.as_slice().is_empty());
}

#[test]
fn pk05_bean_juggler_stuntman_modifiers_compose_and_selling_restores_size() {
    let mut g = shop_game(&[
        JokerKind::TurtleBean,
        JokerKind::Juggler,
        JokerKind::Stuntman,
    ]);
    assert_eq!(g.state.hand_size(), 12);
    g.apply_action(GameAction::SellJoker(2)).unwrap();
    assert_eq!(g.state.hand_size(), 14);
}

#[test]
fn pk06_blueprint_does_not_duplicate_turtle_bean_hand_size_bonus() {
    let g = shop_game(&[JokerKind::Blueprint, JokerKind::TurtleBean]);
    assert_eq!(g.state.hand_size(), 13);
}

#[test]
fn pk08_full_inventory_celestial_option_is_used_and_grows_constellation_once() {
    let mut g = shop_game(&[JokerKind::Constellation]);
    g.state
        .add_consumable(Consumable::Tarot(Tarot::Fool))
        .unwrap();
    g.state
        .add_consumable(Consumable::Tarot(Tarot::Hermit))
        .unwrap();
    g.state.redeem_voucher(Vouchers::Telescope).unwrap();
    g.state.counters.poker_hands_played[0] = 1;
    pack(&mut g, BoosterPackKind::Celestial, BoosterPackSize::Normal);
    assert_eq!(g.state.counters.planet_cards_used, 0);
    assert_eq!(
        g.apply_action(GameAction::UsePackConsumable {
            index: 0,
            target: ConsumableTarget::default()
        }),
        Ok(DecisionPoint::Shop)
    );
    assert_eq!(g.state.consumables.len(), 2);
    assert_eq!(g.state.counters.planet_cards_used, 1);
    assert_eq!(
        g.state.hand_score(PokerHand::HighCard),
        HandScore { chips: 15, mult: 2 }
    );
    assert!((score_mult(&mut g) - 2.2).abs() < 0.0001);
}

#[test]
fn pk09_hallucination_opens_once_with_normal_or_tag_pack_and_respects_capacity() {
    for tag in [false, true] {
        for full in [false, true] {
            let mut g = Game::new(804);
            g.state.money = 100;
            g.state
                .jokers
                .add(Joker::create_joker(JokerKind::Hallucination));
            g.state
                .jokers
                .add(Joker::create_joker(JokerKind::OopsAll6s));
            g.state
                .add_consumable(Consumable::Planet(Planet::Pluto))
                .unwrap();
            if full {
                g.state
                    .add_consumable(Consumable::Planet(Planet::Mercury))
                    .unwrap();
            }
            if tag {
                g.state.blind_selector.set_small_skip_tag(SkipTag::Standard);
                g.apply_action(GameAction::SkipBlind(BlindKind::Small))
                    .unwrap();
            } else {
                g.enter_shop().unwrap();
                pack(&mut g, BoosterPackKind::Standard, BoosterPackSize::Normal);
            }
            let tarots = g
                .state
                .consumables
                .iter()
                .filter(|c| matches!(c.consumable, Consumable::Tarot(_)))
                .count();
            assert_eq!(tarots, usize::from(!full));
            g.apply_action(GameAction::SelectPackOption(0)).unwrap();
            assert_eq!(
                g.state
                    .consumables
                    .iter()
                    .filter(|c| matches!(c.consumable, Consumable::Tarot(_)))
                    .count(),
                tarots
            );
        }
    }
}

#[test]
fn pk10_mega_can_take_one_option_then_skip_remaining_choice_once() {
    let mut g = shop_game(&[JokerKind::RedCard]);
    pack(&mut g, BoosterPackKind::Standard, BoosterPackSize::Mega);
    assert_eq!(
        g.apply_action(GameAction::SelectPackOption(0)),
        Ok(DecisionPoint::PackSelection)
    );
    assert_eq!(
        g.apply_action(GameAction::SkipPack),
        Ok(DecisionPoint::Shop)
    );
    assert_eq!(score_mult(&mut g), 4.0);
}

#[test]
fn pk10_whole_pack_skip_grows_red_card_once_but_both_choices_and_unopened_do_not() {
    for condition in 0..3 {
        let mut g = shop_game(&[JokerKind::RedCard]);
        if condition < 2 {
            pack(&mut g, BoosterPackKind::Standard, BoosterPackSize::Mega);
            if condition == 0 {
                g.apply_action(GameAction::SkipPack).unwrap();
            } else {
                g.apply_action(GameAction::SelectPackOption(0)).unwrap();
                g.apply_action(GameAction::SelectPackOption(0)).unwrap();
            }
        }
        assert_eq!(score_mult(&mut g), if condition == 0 { 4.0 } else { 1.0 });
    }
}

#[test]
fn ge01_perkeo_full_inventory_generates_on_shop_exit_not_pack_close() {
    let mut g = shop_game(&[JokerKind::Perkeo]);
    g.state
        .add_consumable(Consumable::Planet(Planet::Pluto))
        .unwrap();
    g.state
        .add_consumable(Consumable::Planet(Planet::Mercury))
        .unwrap();
    pack(&mut g, BoosterPackKind::Standard, BoosterPackSize::Normal);
    g.apply_action(GameAction::SelectPackOption(0)).unwrap();
    assert_eq!(g.state.consumables.len(), 2);
    g.apply_action(GameAction::LeaveShop).unwrap();
    assert_eq!(
        g.state.consumables.iter().filter(|c| !c.negative).count(),
        2
    );
    assert_eq!(g.state.consumables.iter().filter(|c| c.negative).count(), 1);
    let mut empty = shop_game(&[JokerKind::Perkeo]);
    empty.apply_action(GameAction::LeaveShop).unwrap();
    assert!(empty.state.consumables.is_empty());
}

#[test]
fn ge02_copied_perkeo_adds_two_plutos_without_usage_growth_then_observatory_scores() {
    let mut g = shop_game(&[
        JokerKind::Blueprint,
        JokerKind::Perkeo,
        JokerKind::Constellation,
    ]);
    g.state
        .add_consumable(Consumable::Planet(Planet::Pluto))
        .unwrap();
    g.state.redeem_voucher(Vouchers::Telescope).unwrap();
    g.state.redeem_voucher(Vouchers::Observatory).unwrap();
    g.apply_action(GameAction::LeaveShop).unwrap();
    assert_eq!(g.state.consumables.len(), 3);
    assert_eq!(g.state.counters.planet_cards_used, 0);
    assert_eq!(
        g.state.hand_score(PokerHand::HighCard),
        HandScore { chips: 5, mult: 1 }
    );
    assert!((score_mult(&mut g) - 3.375).abs() < 0.0001);
}

#[test]
fn pk04_selling_bean_after_one_completed_round_restores_exact_base_size() {
    let mut g = shop_game(&[JokerKind::TurtleBean]);
    g.state.begin_blind_as(BlindKind::Small);
    g.state.blind.chips_scored = g.state.blind.score_requirement;
    g.state.finish_blind(&Hand::new()).unwrap();
    g.apply_action(GameAction::SellJoker(0)).unwrap();
    assert_eq!(g.state.hand_size(), 8);
}

#[test]
fn invalid_pack_selections_preserve_the_open_pack_and_its_remaining_choices() {
    let mut g = shop_game(&[JokerKind::RedCard]);
    pack(&mut g, BoosterPackKind::Standard, BoosterPackSize::Mega);
    let owned = g.state.deck.size();
    for index in [99, usize::MAX] {
        assert_eq!(
            g.apply_action(GameAction::SelectPackOption(index)),
            Err(ActionError::Pack(PackError::InvalidSelection))
        );
        assert_eq!(g.decision_point, DecisionPoint::PackSelection);
        assert_eq!(g.pending_pack.as_ref().unwrap().opening.choices(), 2);
        assert_eq!(g.state.deck.size(), owned);
    }
    g.apply_action(GameAction::SelectPackOption(0)).unwrap();
    assert_eq!(g.pending_pack.as_ref().unwrap().opening.choices(), 1);
    assert_eq!(g.pending_pack.as_ref().unwrap().opening.options().len(), 4);
    assert_eq!(g.state.deck.size(), owned + 1);
    g.apply_action(GameAction::SelectPackOption(0)).unwrap();
    assert_eq!(
        g.state.deck.size(),
        owned + 2,
        "index zero now selects a different remaining card"
    );
    assert_eq!(
        g.apply_action(GameAction::SkipPack),
        Err(ActionError::InvalidPhase)
    );
    assert_eq!(
        score_mult(&mut g),
        1.0,
        "both choices taken; Red Card does not grow"
    );
}

#[test]
fn negative_consumables_keep_normal_inventory_full_after_copy_and_sale() {
    let mut g = shop_game(&[JokerKind::Perkeo]);
    g.state
        .add_consumable(Consumable::Planet(Planet::Pluto))
        .unwrap();
    g.state
        .add_consumable(Consumable::Planet(Planet::Mercury))
        .unwrap();
    g.apply_action(GameAction::LeaveShop).unwrap();
    assert_eq!(g.state.consumable_capacity(), 3);
    assert_eq!(
        g.state.add_consumable(Consumable::Tarot(Tarot::Fool)),
        Err(ConsumableUseError::NoCapacity)
    );
    g.enter_shop().unwrap();
    g.apply_action(GameAction::SellConsumable(2)).unwrap();
    assert_eq!(g.state.consumable_capacity(), 2);
    assert_eq!(
        g.state.add_consumable(Consumable::Tarot(Tarot::Fool)),
        Err(ConsumableUseError::NoCapacity)
    );
    g.apply_action(GameAction::SellConsumable(0)).unwrap();
    g.state
        .add_consumable(Consumable::Tarot(Tarot::Fool))
        .unwrap();
}

#[test]
fn full_inventory_mega_celestial_pack_uses_both_planets_one_choice_at_a_time() {
    let mut g = shop_game(&[JokerKind::Constellation]);
    g.state
        .add_consumable(Consumable::Tarot(Tarot::Fool))
        .unwrap();
    g.state
        .add_consumable(Consumable::Tarot(Tarot::Hermit))
        .unwrap();
    pack(&mut g, BoosterPackKind::Celestial, BoosterPackSize::Mega);
    assert_eq!(
        g.apply_action(GameAction::UsePackConsumable {
            index: 0,
            target: ConsumableTarget::default()
        }),
        Ok(DecisionPoint::PackSelection)
    );
    assert_eq!(g.state.counters.planet_cards_used, 1);
    assert_eq!(
        g.apply_action(GameAction::UsePackConsumable {
            index: 0,
            target: ConsumableTarget::default()
        }),
        Ok(DecisionPoint::Shop)
    );
    assert_eq!(g.state.counters.planet_cards_used, 2);
    assert_eq!(g.state.consumables.len(), 2);
}

#[test]
fn failed_buffoon_capacity_check_retains_pack_until_a_slot_is_freed() {
    let mut g = shop_game(&[
        JokerKind::Joker,
        JokerKind::Joker,
        JokerKind::Joker,
        JokerKind::Joker,
        JokerKind::Joker,
    ]);
    pack(&mut g, BoosterPackKind::Buffoon, BoosterPackSize::Normal);
    assert_eq!(
        g.apply_action(GameAction::SelectPackOption(0)),
        Err(ActionError::Pack(PackError::InventoryFull))
    );
    assert_eq!(g.pending_pack.as_ref().unwrap().opening.options().len(), 2);
    // Free a slot through the owned collection's sale entry point.
    g.state.sell_joker(0).unwrap();
    g.apply_action(GameAction::SelectPackOption(0)).unwrap();
    assert_eq!(g.state.jokers.as_slice().len(), 5);
}
