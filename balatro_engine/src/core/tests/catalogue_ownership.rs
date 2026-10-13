use super::*;
use crate::core::consumable::ConsumableTarget;
use crate::core::enums::{Rank, Suit};
use crate::core::joker_types::JokerKind;

fn state(kinds: &[JokerKind]) -> GameState {
    let mut s = GameState::new(802);
    s.begin_blind_as(BlindKind::Small);
    s.money = 0;
    s.blind.score_requirement = u64::MAX;
    for &kind in kinds {
        s.jokers.add(Joker::create_joker(kind));
    }
    s
}
fn deal(s: &mut GameState) -> Hand {
    let mut h = Hand::new();
    s.deal_cards(&mut h);
    h.cards_mut()[0].set_rank(Rank::Two);
    h
}
fn owned(s: &GameState, h: &Hand) -> usize {
    s.deck.cards().len() + s.deck.discarded_cards().len() + h.cards().len()
}

#[test]
fn de01_steel_joker_counts_owned_cards_in_all_zones_and_destruction() {
    for destroy in [false, true] {
        let mut s = state(&[JokerKind::Steel]);
        let mut h = deal(&mut s);
        s.deck
            .card_mut(0)
            .unwrap()
            .set_enhancement(Enhancement::Steel);
        h.cards_mut()[7].set_enhancement(Enhancement::Steel);
        let mut discarded = s.deck.remove_card_at(1).unwrap();
        discarded.set_enhancement(Enhancement::Steel);
        s.deck.discard_card(discarded);
        if destroy {
            s.add_consumable(Consumable::Tarot(Tarot::HangedMan))
                .unwrap();
            s.use_consumable(0, &mut h, ConsumableTarget::hand(vec![7]))
                .unwrap();
        }
        assert_eq!(owned(&s, &h), if destroy { 51 } else { 52 });
        let score = s.play_hand(&mut h, vec![0]).unwrap();
        let expected = if destroy { 1.4 } else { 2.4 };
        assert!(
            (score.mult - expected).abs() < 0.0001,
            "expected {expected}, got {}",
            score.mult
        );
    }
}

#[test]
fn de02_marble_stone_card_keeps_its_stone_joker_contribution_after_moving_zones() {
    let mut s = state(&[JokerKind::Stone, JokerKind::Marble]);
    s.begin_blind_as(BlindKind::Small);
    s.blind.score_requirement = u64::MAX;
    let index = s
        .deck
        .cards()
        .iter()
        .position(|c| c.enhancement() == Enhancement::Stone)
        .unwrap();
    let stone = s.deck.remove_card_at(index).unwrap();
    let mut h = Hand::new();
    h.add_card(stone);
    let mut plain = s.deck.remove_card_at(0).unwrap();
    plain.set_rank(Rank::Two);
    h.add_card(plain);
    s.discard_selected_cards(&mut h, vec![0]).unwrap();
    assert_eq!(owned(&s, &h), 53);
    assert_eq!(s.play_hand(&mut h, vec![0]).unwrap().chips, 32);
}

#[test]
fn de03_blue_uses_remaining_draw_count_and_erosion_uses_owned_count() {
    for destroy in [false, true] {
        let mut s = state(&[JokerKind::Blue, JokerKind::Erosion]);
        let mut h = deal(&mut s);
        if destroy {
            s.add_consumable(Consumable::Tarot(Tarot::HangedMan))
                .unwrap();
            s.use_consumable(0, &mut h, ConsumableTarget::hand(vec![7]))
                .unwrap();
        }
        assert_eq!(s.deck.size(), 44);
        assert_eq!(owned(&s, &h), if destroy { 51 } else { 52 });
        let score = s.play_hand(&mut h, vec![0]).unwrap();
        assert_eq!(
            (score.chips, score.mult),
            (95, if destroy { 5.0 } else { 1.0 })
        );
    }
}

#[test]
fn de04_cloud_nine_settles_all_owned_zones_and_excludes_destroyed_nines() {
    for destroy in [false, true] {
        let mut s = state(&[JokerKind::Cloud9]);
        let mut h = deal(&mut s);
        for card in s.deck.cards().iter().chain(h.cards()) {
            assert_eq!(card.enhancement(), Enhancement::None);
        }
        for i in 0..usize::from(s.deck.size()) {
            if s.deck.card(i).unwrap().rank() == Rank::Nine {
                s.deck.card_mut(i).unwrap().set_rank(Rank::Three);
            }
        }
        for card in h.cards_mut() {
            if card.rank() == Rank::Nine {
                card.set_rank(Rank::Three);
            }
        }
        s.deck.card_mut(0).unwrap().set_rank(Rank::Nine);
        h.cards_mut()[7].set_rank(Rank::Nine);
        let mut discarded = s.deck.remove_card_at(1).unwrap();
        discarded.set_rank(Rank::Nine);
        s.deck.discard_card(discarded);
        if destroy {
            s.add_consumable(Consumable::Tarot(Tarot::HangedMan))
                .unwrap();
            s.use_consumable(0, &mut h, ConsumableTarget::hand(vec![7]))
                .unwrap();
        }
        s.play_hand(&mut h, vec![0]).unwrap();
        assert_eq!(s.money, 0, "no per-hand payout");
        s.blind.chips_scored = s.blind.score_requirement;
        s.finish_blind(&h).unwrap();
        assert_eq!(
            s.money,
            if destroy { 8 } else { 9 },
            "Cloud 9 + $3 reward + $3 unused hands"
        );
    }
}

#[test]
fn de05_drivers_license_counts_enhancements_in_all_owned_zones_only() {
    for stripped in [false, true] {
        let mut s = state(&[JokerKind::DriversLicense]);
        let mut h = deal(&mut s);
        for i in 0..8 {
            s.deck
                .card_mut(i)
                .unwrap()
                .set_enhancement(Enhancement::Bonus);
        }
        for i in 1..8 {
            h.cards_mut()[i].set_enhancement(Enhancement::Bonus);
        }
        let mut discarded = s.deck.remove_card_at(8).unwrap();
        discarded.set_enhancement(Enhancement::Bonus);
        s.deck.discard_card(discarded);
        if stripped {
            h.cards_mut()[1].set_enhancement(Enhancement::None);
            h.cards_mut()[1].set_edition(Edition::Foil);
            h.cards_mut()[1].set_seal(Seal::Gold);
        }
        assert_eq!(owned(&s, &h), 52);
        assert_eq!(
            s.play_hand(&mut h, vec![0]).unwrap().mult,
            if stripped { 1.0 } else { 3.0 }
        );
    }
}

#[test]
fn li01_card_sharp_remembers_nonconsecutive_hands_and_resets_next_blind() {
    let mut s = state(&[JokerKind::CardSharp]);
    for (pair, expected) in [(true, 2.0), (false, 1.0), (true, 6.0)] {
        let mut h = Hand::new();
        h.add_card(Card::new(
            Rank::Two,
            Suit::Clubs,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ));
        if pair {
            h.add_card(Card::new(
                Rank::Two,
                Suit::Hearts,
                Enhancement::None,
                Edition::None,
                Seal::None,
            ));
        }
        let positions = (0..h.hand_size()).collect();
        assert_eq!(s.play_hand(&mut h, positions).unwrap().mult, expected);
    }
    s.begin_blind_as(BlindKind::Big);
    s.blind.score_requirement = u64::MAX;
    let mut h = Hand::new();
    for suit in [Suit::Clubs, Suit::Hearts] {
        h.add_card(Card::new(
            Rank::Two,
            suit,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ));
    }
    assert_eq!(s.play_hand(&mut h, vec![0, 1]).unwrap().mult, 2.0);
}

#[test]
fn li05_madness_grows_without_a_victim_and_only_on_non_boss_selection() {
    let mut s = state(&[JokerKind::Madness]);
    for (kind, expected) in [
        (BlindKind::Small, 1.5),
        (BlindKind::Big, 2.0),
        (BlindKind::Boss(BossBlinds::Hook), 2.0),
    ] {
        s.begin_blind_as(kind);
        s.blind.score_requirement = u64::MAX;
        let mut h = deal(&mut s);
        assert_eq!(s.play_hand(&mut h, vec![0]).unwrap().mult, expected);
        assert_eq!(s.jokers.as_slice().len(), 1);
    }
}

#[test]
fn li06_burglar_copies_remove_all_discards_even_when_allowance_exceeds_four() {
    for copied in [false, true] {
        let mut s = state(if copied {
            &[
                JokerKind::Blueprint,
                JokerKind::Burglar,
                JokerKind::MerryAndy,
            ]
        } else {
            &[JokerKind::Burglar, JokerKind::MerryAndy]
        });
        // Explicitly configure Andy's raised allowance, avoiding an accidental pass at <=4.
        s.blind.discards_per_blind = 6;
        s.begin_blind_as(BlindKind::Small);
        assert_eq!(s.blind.hands_remaining, if copied { 10 } else { 7 });
        assert_eq!(s.blind.discards_remaining, 0);
    }
}

#[test]
fn ge04_trading_card_removes_first_single_discard_permanently() {
    let mut s = state(&[JokerKind::TradingCard, JokerKind::Erosion]);
    let mut h = deal(&mut s);
    s.discard_selected_cards(&mut h, vec![7]).unwrap();
    assert_eq!(s.money, 3);
    assert_eq!(owned(&s, &h), 51);
    assert_eq!(s.play_hand(&mut h, vec![0]).unwrap().mult, 5.0);
}

#[test]
fn ge04_trading_card_does_not_pay_for_multiple_or_later_discards() {
    let mut s = state(&[JokerKind::TradingCard]);
    let mut h = deal(&mut s);
    s.discard_selected_cards(&mut h, vec![6, 7]).unwrap();
    assert_eq!(s.money, 0);
    assert_eq!(owned(&s, &h), 52);
    s.discard_selected_cards(&mut h, vec![7]).unwrap();
    assert_eq!(s.money, 0);
    assert_eq!(owned(&s, &h), 52);
}

#[test]
fn ge05_planet_usage_grows_constellation_per_use_and_satellite_per_unique_planet() {
    let mut s = state(&[JokerKind::Constellation, JokerKind::Satellite]);
    let mut h = Hand::new();
    for planet in [Planet::Pluto, Planet::Pluto, Planet::Mercury] {
        s.add_consumable(Consumable::Planet(planet)).unwrap();
        s.use_consumable(0, &mut h, ConsumableTarget::default())
            .unwrap();
    }
    h.add_card(Card::new(
        Rank::Two,
        Suit::Clubs,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));
    let score = s.play_hand(&mut h, vec![0]).unwrap();
    assert!(
        (score.mult - 3.9).abs() < 0.0001,
        "High Card upgraded twice; Constellation x1.3"
    );
    assert_eq!(s.money, 0);
    s.blind.chips_scored = s.blind.score_requirement;
    s.finish_blind(&h).unwrap();
    assert_eq!(s.money, 8, "Satellite $2 + reward $3 + three unused hands");
}

#[test]
fn ge05_black_hole_generation_purchase_and_sale_do_not_count_as_planet_use() {
    use crate::core::shop::ShopItem;
    let mut s = state(&[JokerKind::Constellation, JokerKind::Satellite]);
    s.money = 20;
    let mut shop = ShopState::new();
    shop.items.push(ShopItem::Consumable {
        consumable: Consumable::Planet(Planet::Pluto),
        price: 3,
    });
    shop.buy_item(0, &mut s).unwrap();
    s.sell_consumable(0).unwrap();
    s.materialize_generated_effects(
        &[JokerOperation::GenerateConsumable {
            kind: GeneratedConsumable::Planet,
            count: 1,
        }],
        &mut [],
        None,
    );
    s.add_consumable(Consumable::Spectral(Spectral::BlackHole))
        .unwrap();
    let mut h = Hand::new();
    s.use_consumable(1, &mut h, ConsumableTarget::default())
        .unwrap();
    assert_eq!(s.counters.planet_cards_used, 0);
    assert_eq!(s.counters.used_planets, 0);
    h.add_card(Card::new(
        Rank::Two,
        Suit::Clubs,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));
    assert_eq!(
        s.play_hand(&mut h, vec![0]).unwrap().mult,
        2.0,
        "only Black Hole's hand-level upgrade"
    );
}

#[test]
fn ge03_riff_raff_rechecks_one_free_slot_including_copied_activation() {
    for condition in 0..3 {
        let mut s = state(if condition == 2 {
            &[
                JokerKind::Blueprint,
                JokerKind::RiffRaff,
                JokerKind::Joker,
                JokerKind::Joker,
            ]
        } else if condition == 1 {
            &[
                JokerKind::RiffRaff,
                JokerKind::Joker,
                JokerKind::Joker,
                JokerKind::Joker,
                JokerKind::Joker,
            ]
        } else {
            &[
                JokerKind::RiffRaff,
                JokerKind::Joker,
                JokerKind::Joker,
                JokerKind::Joker,
            ]
        });
        s.begin_blind_as(BlindKind::Small);
        assert_eq!(s.jokers.as_slice().len(), 5);
        if condition != 1 {
            let generated = s.jokers.get(4).unwrap();
            assert_eq!(
                generated.rarity(),
                crate::core::joker_types::JokerRarity::Common
            );
            assert_eq!(
                generated.edition(),
                crate::core::joker_types::JokerEdition::None
            );
        }
    }
}

#[test]
fn li07_burglar_leaves_no_delayed_gratification_payout() {
    let mut s = state(&[JokerKind::Burglar, JokerKind::DelayedGratification]);
    s.begin_blind_as(BlindKind::Small);
    assert_eq!(s.blind.discards_remaining, 0);
    s.blind.chips_scored = s.blind.score_requirement;
    let payout = s.finish_blind(&Hand::new()).unwrap();
    assert_eq!(
        s.money,
        i16::from(payout.blind_reward) + payout.hand_money + payout.interest
    );
}

#[test]
fn steel_joker_counts_played_steel_without_applying_the_held_card_multiplier() {
    let mut s = state(&[JokerKind::Steel]);
    let mut h = deal(&mut s);
    h.cards_mut()[0].set_enhancement(Enhancement::Steel);
    let score = s.play_hand(&mut h, vec![0]).unwrap();
    assert!((score.mult - 1.2).abs() < 0.0001);
    assert_eq!(owned(&s, &h), 52);
}

#[test]
fn debuffed_trading_card_does_not_pay_or_destroy_the_single_discard() {
    let mut s = state(&[JokerKind::TradingCard]);
    s.jokers.get_mut(0).unwrap().set_debuffed(true);
    let mut h = deal(&mut s);
    s.discard_selected_cards(&mut h, vec![7]).unwrap();
    assert_eq!((s.money, owned(&s, &h)), (0, 52));
}

#[test]
fn hook_does_not_use_the_first_discard_for_burnt_or_trading_card() {
    let mut s = state(&[JokerKind::BurntJoker, JokerKind::TradingCard]);
    s.begin_blind_as(BlindKind::Boss(BossBlinds::Hook));
    let mut h = Hand::new();
    h.add_card(s.deck.remove_card_at(0).unwrap());
    s.apply_hook_discard(&mut h);
    assert_eq!(s.money, 0);
    assert_eq!(owned(&s, &h), 52);
    assert_eq!(s.blind.discards_used, 0);
    assert_eq!(
        s.hand_score(PokerHand::HighCard),
        HandScore { chips: 5, mult: 1 }
    );
    s.deal_cards(&mut h);
    h.cards_mut()[0].set_rank(Rank::Two);
    s.discard_selected_cards(&mut h, vec![0]).unwrap();
    assert_eq!((s.money, owned(&s, &h)), (3, 51));
    assert_eq!(
        s.hand_score(PokerHand::HighCard),
        HandScore { chips: 15, mult: 2 }
    );
}
