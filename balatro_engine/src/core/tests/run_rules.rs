use super::*;
use crate::core::consumable::ConsumableTarget;
use crate::core::enums::{Rank, Suit};
use crate::core::joker_types::{JokerEdition, JokerKind};
use crate::core::shop::{ShopItem, ShopOfferKind, random_base_item};

const DECKS: [Decks; 15] = [
    Decks::Red,
    Decks::Blue,
    Decks::Yellow,
    Decks::Green,
    Decks::Black,
    Decks::Magic,
    Decks::Nebula,
    Decks::Ghost,
    Decks::Abandoned,
    Decks::Checkered,
    Decks::Zodiac,
    Decks::Painted,
    Decks::Anaglyph,
    Decks::Plasma,
    Decks::Erratic,
];
const STAKES: [Stakes; 8] = [
    Stakes::White,
    Stakes::Red,
    Stakes::Green,
    Stakes::Black,
    Stakes::Blue,
    Stakes::Purple,
    Stakes::Orange,
    Stakes::Gold,
];

fn winning_hand(game: &mut Game) {
    // Upgrade through the normal hand-level API; leave every blind's actual target intact.
    for hand in std::iter::once(PokerHand::HighCard).chain(PokerHand::DISCARD_HANDS) {
        game.state.upgrade_hand_score(hand, 60000, 100);
    }
    let indices = if game.state.blind.kind == BlindKind::Boss(BossBlinds::Psychich) {
        vec![0, 1, 2, 3, 4]
    } else {
        vec![game.state.blind.forced_card_index.unwrap_or(0) as u8]
    };
    game.apply_action(GameAction::PlayCards(indices)).unwrap();
    assert_eq!(game.decision_point(), DecisionPoint::Shop);
}

#[test]
fn complete_seeded_blind_cycles_advance_only_after_the_boss_shop() {
    let mut game = Game::new(701);
    let selector = game.state.blind_selector;
    let voucher = game.state.ante_voucher;
    for kind in [BlindKind::Small, BlindKind::Big, selector.boss().kind] {
        game.apply_action(GameAction::SelectBlind(kind)).unwrap();
        winning_hand(&mut game);
        assert_eq!(game.state.counters.ante, 1);
        assert_eq!(game.state.blind_selector, selector);
        assert_eq!(game.state.ante_voucher, voucher);
        game.apply_action(GameAction::LeaveShop).unwrap();
    }
    assert_eq!(game.state.counters.ante, 2);
    assert_eq!(game.state.counters.round, 4);
    assert_eq!(game.blind_stage, BlindStage::Small);
    assert_eq!(game.next_step(), DecisionPoint::BlindSelection);
    assert_eq!(
        game.apply_action(GameAction::LeaveShop),
        Err(ActionError::InvalidPhase)
    );
    game.apply_action(GameAction::SelectBlind(BlindKind::Small))
        .unwrap();
    assert_eq!(game.state.blind.score_requirement, 800);
    assert_eq!(
        game.hand.cards().len()
            + game.state.deck.cards().len()
            + game.state.deck.discarded_cards().len(),
        52
    );
}

#[test]
fn replayed_and_out_of_order_blinds_are_rejected_without_mutation() {
    let mut game = Game::new(702);
    let before = game.state.money;
    for action in [
        GameAction::SelectBlind(BlindKind::Big),
        GameAction::SkipBlind(BlindKind::Big),
    ] {
        assert_eq!(
            game.apply_action(action),
            Err(ActionError::Decision(DecisionError::InvalidBlindChoice))
        );
        assert_eq!(game.state.money, before);
        assert_eq!(game.state.counters.blinds_skipped, 0);
        assert_eq!(game.hand.hand_size(), 0);
    }
    game.apply_action(GameAction::SelectBlind(BlindKind::Small))
        .unwrap();
    winning_hand(&mut game);
    game.apply_action(GameAction::LeaveShop).unwrap();
    assert!(
        game.apply_action(GameAction::SelectBlind(BlindKind::Small))
            .is_err()
    );
}

#[test]
fn skipping_advances_blinds_without_round_income_or_ante_changes() {
    let mut game = Game::new(703);
    game.state
        .blind_selector
        .set_small_skip_tag(SkipTag::Double);
    game.apply_action(GameAction::SkipBlind(BlindKind::Small))
        .unwrap();
    assert_eq!(game.state.money, 4);
    assert_eq!(game.state.counters.round, 1);
    assert_eq!(game.state.counters.ante, 1);
    assert_eq!(game.blind_stage, BlindStage::Big);
    game.apply_action(GameAction::SelectBlind(BlindKind::Big))
        .unwrap();
    winning_hand(&mut game);
    game.apply_action(GameAction::LeaveShop).unwrap();
    assert_eq!(game.blind_stage, BlindStage::Boss);
    let boss = game.state.blind_selector.boss().kind;
    assert!(matches!(
        game.apply_action(GameAction::SkipBlind(boss)),
        Err(ActionError::BlindSkip(_))
    ));
}

#[test]
fn cycle_is_deterministic_for_identical_seed_and_actions() {
    let mut left = Game::new(704);
    let mut right = Game::new(704);
    for _ in 0..6 {
        for game in [&mut left, &mut right] {
            let blind = game.available_blind();
            game.apply_action(GameAction::SelectBlind(blind)).unwrap();
            winning_hand(game);
            game.apply_action(GameAction::LeaveShop).unwrap();
        }
        assert_eq!(left.state.blind_selector, right.state.blind_selector);
        assert_eq!(left.state.ante_voucher, right.state.ante_voucher);
        assert_eq!(left.state.money, right.state.money);
        assert_eq!(
            left.hand.cards().iter().map(Card::id).collect::<Vec<_>>(),
            right.hand.cards().iter().map(Card::id).collect::<Vec<_>>()
        );
    }
    assert_eq!(left.state.counters.ante, 3);
}

#[test]
fn empty_initial_draw_is_a_terminal_loss_and_cannot_pay_or_take_actions() {
    let mut game = Game::new(705);
    game.state.deck.remove_cards(52);
    assert_eq!(
        game.apply_action(GameAction::SelectBlind(BlindKind::Small)),
        Ok(DecisionPoint::RunComplete)
    );
    assert_eq!(game.state.counters.life, 0);
    assert!(!game.state.blind.finished);
    assert_eq!(game.state.money, 4);
    assert_eq!(game.next_step(), DecisionPoint::RunComplete);
    assert_eq!(
        game.apply_action(GameAction::PlayCards(vec![0])),
        Err(ActionError::InvalidPhase)
    );
}

#[test]
fn final_hand_loss_is_terminal_and_preserves_ante_and_unpaid_rewards() {
    let mut game = Game::new(706);
    game.apply_action(GameAction::SelectBlind(BlindKind::Small))
        .unwrap();
    game.state.blind.hands_remaining = 1;
    game.state.blind.score_requirement = u64::MAX;
    assert_eq!(
        game.apply_action(GameAction::PlayCards(vec![0])),
        Ok(DecisionPoint::RunComplete)
    );
    assert_eq!(game.state.counters.life, 0);
    assert_eq!(game.state.money, 4);
    assert_eq!(game.state.counters.ante, 1);
    assert!(!game.state.blind.finished);
}

#[test]
fn initial_empty_deck_does_not_consume_mr_bones_or_generate_rewards() {
    let mut game = Game::new(707);
    game.state
        .jokers
        .add(Joker::create_joker(JokerKind::MrBones));
    game.state.deck.remove_cards(52);
    game.apply_action(GameAction::SelectBlind(BlindKind::Small))
        .unwrap();
    assert_eq!(game.decision_point(), DecisionPoint::RunComplete);
    assert_eq!(game.state.jokers.as_slice().len(), 1);
    assert_eq!(game.state.money, 4);
}

#[test]
fn all_decks_have_their_starting_resources_and_inventory() {
    for deck in DECKS {
        let state = GameState::with_rules(708, deck, Stakes::White);
        assert_eq!(
            state.money,
            if deck == Decks::Yellow { 14 } else { 4 },
            "{deck:?}"
        );
        assert_eq!(
            state.hand_size(),
            if deck == Decks::Painted { 10 } else { 8 },
            "{deck:?}"
        );
        assert_eq!(
            state.hands_per_blind(),
            match deck {
                Decks::Blue => 5,
                Decks::Black => 3,
                _ => 4,
            }
        );
        assert_eq!(
            state.discards_per_blind(),
            if deck == Decks::Red { 4 } else { 3 }
        );
        assert_eq!(
            state.joker_capacity(),
            match deck {
                Decks::Black => 6,
                Decks::Painted => 4,
                _ => 5,
            }
        );
        assert_eq!(
            state.consumable_capacity(),
            match deck {
                Decks::Magic => 3,
                Decks::Nebula => 1,
                _ => 2,
            }
        );
        assert_eq!(
            state.deck.size(),
            if deck == Decks::Abandoned { 40 } else { 52 }
        );
        match deck {
            Decks::Magic => {
                assert!(state.has_voucher(Vouchers::CrystalBall));
                assert_eq!(state.consumables.len(), 2);
                assert!(
                    state
                        .consumables
                        .iter()
                        .all(|entry| entry.consumable == Consumable::Tarot(Tarot::Fool))
                );
            }
            Decks::Nebula => assert!(state.has_voucher(Vouchers::Telescope)),
            Decks::Ghost => {
                assert_eq!(
                    state.consumables[0].consumable,
                    Consumable::Spectral(Spectral::Hex)
                );
                assert_eq!(state.shop.offer_weights.spectrals, 2);
            }
            Decks::Zodiac => {
                for voucher in [
                    Vouchers::TarotMerchant,
                    Vouchers::PlanetMerchant,
                    Vouchers::Overstock,
                ] {
                    assert!(state.has_voucher(voucher));
                }
                assert_eq!(state.shop.number_of_shop_items, 3);
            }
            _ => assert!(state.consumables.is_empty()),
        }
    }
}

#[test]
fn checkered_and_abandoned_deck_composition_reaches_actual_draws() {
    for deck in [Decks::Checkered, Decks::Abandoned] {
        let mut game = Game::with_rules(709, deck, Stakes::White);
        game.apply_action(GameAction::SelectBlind(BlindKind::Small))
            .unwrap();
        for card in game.hand.cards().iter().chain(game.state.deck.cards()) {
            if deck == Decks::Checkered {
                assert!(matches!(card.suit(), Suit::Hearts | Suit::Spades));
            } else {
                assert!(!card.is_face_card());
            }
        }
    }
}

#[test]
fn erratic_deck_is_seeded_and_keeps_fifty_two_physical_cards() {
    let left = GameState::with_rules(710, Decks::Erratic, Stakes::White);
    let right = GameState::with_rules(710, Decks::Erratic, Stakes::White);
    assert_eq!(
        left.deck.cards().iter().map(Card::id).collect::<Vec<_>>(),
        right.deck.cards().iter().map(Card::id).collect::<Vec<_>>()
    );
    assert_eq!(left.deck.size(), 52);
    assert!(
        left.deck
            .cards()
            .iter()
            .map(Card::id)
            .collect::<std::collections::HashSet<_>>()
            .len()
            < 52
    );
}

#[test]
fn all_stakes_stack_discard_penalties_and_score_curves() {
    for stake in STAKES {
        let mut state = GameState::with_rules(711, Decks::Red, stake);
        state.counters.ante = 2;
        state.begin_blind_as(BlindKind::Small);
        assert_eq!(state.blind.stake, stake);
        assert_eq!(
            state.blind.discards_remaining,
            if stake as u8 >= Stakes::Blue as u8 {
                3
            } else {
                4
            }
        );
        assert_eq!(
            state.blind.score_requirement,
            match stake {
                Stakes::White | Stakes::Red => 800,
                Stakes::Green | Stakes::Black | Stakes::Blue => 900,
                _ => 1000,
            }
        );
        assert_eq!(
            state.blind.reward(),
            if stake == Stakes::White { 3 } else { 0 }
        );
        state.begin_blind_as(BlindKind::Big);
        assert_eq!(state.blind.reward(), 4);
    }
}

#[test]
fn deck_and_stake_discard_hand_modifiers_combine_with_vouchers() {
    let mut state = GameState::with_rules(712, Decks::Blue, Stakes::Gold);
    state.redeem_voucher(Vouchers::Grabber).unwrap();
    state.redeem_voucher(Vouchers::Wasteful).unwrap();
    state.begin_blind_as(BlindKind::Big);
    assert_eq!(
        (state.blind.hands_remaining, state.blind.discards_remaining),
        (6, 3)
    );
}

#[test]
fn normal_and_green_payouts_include_resources_and_green_ignores_interest_vouchers() {
    for deck in [Decks::Red, Decks::Green] {
        let mut state = GameState::with_rules(713, deck, Stakes::White);
        state.money = 50;
        state.redeem_voucher(Vouchers::SeedMoney).unwrap();
        state.begin_blind_as(BlindKind::Small);
        state.blind.hands_remaining = 2;
        state.blind.discards_remaining = 1;
        state.blind.chips_scored = state.blind.score_requirement;
        let payout = state.finish_blind(&Hand::new()).unwrap();
        assert_eq!(payout.interest, if deck == Decks::Green { 0 } else { 10 });
        assert_eq!(payout.hand_money, if deck == Decks::Green { 4 } else { 2 });
        assert_eq!(
            payout.discard_money,
            if deck == Decks::Green { 1 } else { 0 }
        );
        assert_eq!(payout.total, if deck == Decks::Green { 8 } else { 15 });
        assert!(state.finish_blind(&Hand::new()).is_none());
    }
}

#[test]
fn anaglyph_gains_one_double_tag_only_at_boss_completion() {
    let mut state = GameState::with_rules(714, Decks::Anaglyph, Stakes::White);
    state.begin_blind_as(BlindKind::Small);
    state.blind.chips_scored = state.blind.score_requirement;
    state.finish_blind(&Hand::new()).unwrap();
    assert!(state.tags.held().is_empty());
    state.begin_blind_as(BlindKind::Boss(BossBlinds::Hook));
    state.blind.chips_scored = state.blind.score_requirement;
    state.finish_blind(&Hand::new()).unwrap();
    assert_eq!(state.tags.held(), &[SkipTag::Double]);
    assert!(state.finish_blind(&Hand::new()).is_none());
    assert_eq!(state.tags.held().len(), 1);
}

#[test]
fn plasma_balances_final_score_and_keeps_double_targets_after_boss_disable() {
    let mut state = GameState::with_rules(715, Decks::Plasma, Stakes::White);
    state.begin_blind_as(BlindKind::Small);
    assert_eq!(state.blind.score_requirement, 600);
    let mut hand = Hand::new();
    hand.add_card(Card::new(
        Rank::King,
        Suit::Clubs,
        Enhancement::Mult,
        Edition::None,
        Seal::None,
    ));
    let score = state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!((score.chips, score.mult, score.score), (10, 10.0, 100.0));
    state.begin_blind_as(BlindKind::Boss(BossBlinds::Wall));
    assert_eq!(state.blind.score_requirement, 2400);
    state.blind.disable_boss_ability();
    assert_eq!(state.blind.score_requirement, 1200);
}

#[test]
fn shop_joker_stickers_are_seeded_and_respect_stake_thresholds() {
    for stake in STAKES {
        let mut a = GameState::with_rules(716, Decks::Red, stake);
        let mut b = GameState::with_rules(716, Decks::Red, stake);
        let mut eternal = false;
        let mut perishable = false;
        let mut rental = false;
        for _ in 0..150 {
            let ShopItem::Joker { joker: left, price } =
                random_base_item(ShopOfferKind::Joker, &mut a)
            else {
                panic!()
            };
            let ShopItem::Joker { joker: right, .. } =
                random_base_item(ShopOfferKind::Joker, &mut b)
            else {
                panic!()
            };
            assert_eq!(
                (
                    left.kind(),
                    left.is_eternal(),
                    left.perishable_rounds(),
                    left.is_rental()
                ),
                (
                    right.kind(),
                    right.is_eternal(),
                    right.perishable_rounds(),
                    right.is_rental()
                )
            );
            assert!(!(left.is_eternal() && left.perishable_rounds().is_some()));
            if left.is_rental() {
                assert_eq!(price, 1);
                assert_eq!(left.sell_value(), 1);
            }
            eternal |= left.is_eternal();
            perishable |= left.perishable_rounds().is_some();
            rental |= left.is_rental();
        }
        assert_eq!(eternal, stake as u8 >= Stakes::Black as u8);
        assert_eq!(perishable, stake as u8 >= Stakes::Orange as u8);
        assert_eq!(rental, stake == Stakes::Gold);
    }
}

#[test]
fn eternal_cannot_be_sold_or_eaten_by_dagger_and_madness_skips_it() {
    let mut state = GameState::new(717);
    state
        .jokers
        .add(Joker::create_joker(JokerKind::CeremonialDagger));
    let mut victim = Joker::create_joker(JokerKind::Joker);
    victim.set_eternal();
    state.jokers.add(victim);
    assert!(state.sell_joker(1).is_none());
    state.begin_blind_as(BlindKind::Small);
    assert_eq!(state.jokers.as_slice().len(), 2);
    state.jokers.add(Joker::create_joker(JokerKind::Madness));
    state.begin_blind_as(BlindKind::Big);
    assert!(state.jokers.as_slice().iter().any(Joker::is_eternal));
    assert!(
        state
            .jokers
            .as_slice()
            .iter()
            .any(|joker| joker.kind() == JokerKind::Madness)
    );
}

#[test]
fn sticker_compatibility_excludes_self_destructors_and_permanent_scalers() {
    for kind in [
        JokerKind::GrosMichel,
        JokerKind::IceCream,
        JokerKind::MrBones,
        JokerKind::Invisible,
    ] {
        let mut joker = Joker::create_joker(kind);
        joker.set_eternal();
        assert!(!joker.is_eternal());
    }
    for kind in [
        JokerKind::CeremonialDagger,
        JokerKind::Runner,
        JokerKind::LuckyCat,
        JokerKind::Glass,
    ] {
        let mut joker = Joker::create_joker(kind);
        joker.set_perishable();
        assert_eq!(joker.perishable_rounds(), None);
    }
}

#[test]
fn perishable_scores_five_rounds_then_stays_debuffed_across_blind_cleanup_and_copies() {
    let mut state = GameState::new(718);
    let mut joker = Joker::create_joker(JokerKind::Joker);
    joker.set_perishable();
    joker.set_edition(JokerEdition::Polychrome);
    state.jokers.add(Joker::create_joker(JokerKind::Blueprint));
    state.jokers.add(joker);
    for round in 0..6 {
        state.begin_blind_as(BlindKind::Small);
        state.blind.score_requirement = 1;
        let mut hand = Hand::new();
        hand.add_card(Card::new(
            Rank::Two,
            Suit::Clubs,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ));
        let score = state.play_hand(&mut hand, vec![0]).unwrap();
        assert_eq!(score.mult, if round < 5 { 13.5 } else { 1.0 });
        assert_eq!(state.jokers.get(1).unwrap().is_debuffed(), round >= 4);
    }
}

#[test]
fn rental_charges_while_debuffed_before_interest_and_pays_only_once() {
    let mut state = GameState::new(719);
    state.money = 10;
    let mut joker = Joker::create_joker(JokerKind::Joker);
    joker.set_rental();
    joker.set_debuffed(true);
    state.jokers.add(joker);
    state.begin_blind_as(BlindKind::Small);
    state.jokers.get_mut(0).unwrap().set_debuffed(true);
    state.blind.chips_scored = state.blind.score_requirement;
    let payout = state.finish_blind(&Hand::new()).unwrap();
    assert_eq!(payout.interest, 1);
    assert_eq!(state.money, 15);
    assert!(state.finish_blind(&Hand::new()).is_none());
    assert_eq!(state.money, 15);
}

#[test]
fn hex_and_ankh_preserve_eternal_cards_and_ankh_copies_the_selected_card() {
    for spectral in [Spectral::Hex, Spectral::Ankh] {
        let mut state = GameState::new(720);
        let mut eternal = Joker::create_joker(JokerKind::Joker);
        eternal.set_eternal();
        state.jokers.add(eternal);
        state.jokers.add(Joker::create_joker(JokerKind::Greedy));
        state
            .add_consumable(Consumable::Spectral(spectral))
            .unwrap();
        state
            .use_consumable(0, &mut Hand::new(), ConsumableTarget::default())
            .unwrap();
        assert!(state.jokers.as_slice().iter().any(Joker::is_eternal));
        if spectral == Spectral::Ankh {
            assert!(state.jokers.as_slice().len() >= 2);
        }
    }
}

#[test]
fn buffoon_packs_share_sticker_rules_and_keep_stickers_on_acquisition() {
    use crate::core::pack::{BoosterPackKind, BoosterPackSize, PackOption, ShopPack};
    let mut state = GameState::with_rules(721, Decks::Red, Stakes::Gold);
    let mut seen = [false; 3];
    for _ in 0..40 {
        let opening =
            ShopPack::new(BoosterPackKind::Buffoon, BoosterPackSize::Normal, 4).open(&mut state);
        for option in opening.options() {
            if let PackOption::Joker(joker) = option {
                seen[0] |= joker.is_eternal();
                seen[1] |= joker.perishable_rounds().is_some();
                seen[2] |= joker.is_rental();
                assert!(!(joker.is_eternal() && joker.perishable_rounds().is_some()));
            }
        }
    }
    assert_eq!(seen, [true; 3]);
    let mut state = GameState::with_rules(722, Decks::Red, Stakes::Gold);
    let mut opening =
        ShopPack::new(BoosterPackKind::Buffoon, BoosterPackSize::Normal, 4).open(&mut state);
    let PackOption::Joker(expected) = &opening.options()[0] else {
        panic!()
    };
    let expected = (
        expected.kind(),
        expected.is_eternal(),
        expected.perishable_rounds(),
        expected.is_rental(),
    );
    state.select_pack_options(&mut opening, &[0]).unwrap();
    let joker = state.jokers.get(0).unwrap();
    assert_eq!(
        (
            joker.kind(),
            joker.is_eternal(),
            joker.perishable_rounds(),
            joker.is_rental()
        ),
        expected
    );
}

#[test]
fn rental_shop_purchase_costs_one_and_selling_stops_future_charges() {
    let mut state = GameState::new(723);
    let mut joker = Joker::create_joker(JokerKind::Joker);
    joker.set_rental();
    state.shop.items.push(ShopItem::Joker {
        price: joker.price(),
        joker,
    });
    let mut shop = std::mem::replace(&mut state.shop, ShopState::new());
    shop.buy_item(0, &mut state).unwrap();
    state.shop = shop;
    assert_eq!(state.money, 3);
    assert_eq!(state.sell_joker(0), Some(1));
    assert_eq!(state.money, 4);
    state.begin_blind_as(BlindKind::Small);
    state.blind.chips_scored = state.blind.score_requirement;
    state.finish_blind(&Hand::new()).unwrap();
    assert_eq!(state.money, 11);
}

#[test]
fn expired_passive_and_generation_abilities_stay_inactive_after_cleanup() {
    let mut state = GameState::new(724);
    for kind in [JokerKind::OopsAll6s, JokerKind::Vagabond] {
        let mut joker = Joker::create_joker(kind);
        joker.set_perishable();
        state.jokers.add(joker);
    }
    for _ in 0..5 {
        state.begin_blind_as(BlindKind::Small);
        state.blind.chips_scored = state.blind.score_requirement;
        state.finish_blind(&Hand::new()).unwrap();
    }
    state.begin_blind_as(BlindKind::Small);
    state.money = 0;
    assert_eq!(state.jokers.modifiers().probability_multiplier, 1.0);
    let mut hand = Hand::new();
    hand.add_card(Card::new(
        Rank::Two,
        Suit::Clubs,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));
    state.play_hand(&mut hand, vec![0]).unwrap();
    assert!(state.consumables.is_empty());
}

#[test]
fn shop_skips_do_not_age_or_charge_stickers() {
    let mut game = Game::new(725);
    let mut joker = Joker::create_joker(JokerKind::Joker);
    joker.set_perishable();
    joker.set_rental();
    game.state.jokers.add(joker);
    game.state
        .blind_selector
        .set_small_skip_tag(SkipTag::Double);
    game.apply_action(GameAction::SkipBlind(BlindKind::Small))
        .unwrap();
    assert_eq!(game.state.money, 4);
    assert_eq!(
        game.state.jokers.get(0).unwrap().perishable_rounds(),
        Some(5)
    );
}

#[test]
fn round_started_generators_run_before_initial_empty_hand_loss_resolution() {
    let mut game = Game::new(726);
    game.state.deck.remove_cards(52);
    game.state
        .jokers
        .add(Joker::create_joker(JokerKind::Certificate));
    game.apply_action(GameAction::SelectBlind(BlindKind::Small))
        .unwrap();
    assert_eq!(game.decision_point(), DecisionPoint::PlayingHand);
    assert_eq!(game.hand.cards().len(), 1);
    assert_ne!(game.hand.cards()[0].seal(), Seal::None);
    assert_eq!(game.state.counters.life, 1);
}

#[test]
fn ankh_with_full_eternal_capacity_fails_without_destroying_or_consuming() {
    let mut state = GameState::new(727);
    for _ in 0..5 {
        let mut joker = Joker::create_joker(JokerKind::Joker);
        joker.set_eternal();
        state.jokers.add(joker);
    }
    state
        .add_consumable(Consumable::Spectral(Spectral::Ankh))
        .unwrap();
    assert_eq!(
        state.use_consumable(0, &mut Hand::new(), ConsumableTarget::default()),
        Err(crate::core::consumable::ConsumableUseError::NoCapacity)
    );
    assert_eq!(state.jokers.as_slice().len(), 5);
    assert_eq!(state.consumables.len(), 1);
}

#[test]
fn held_gold_and_blue_effects_follow_perishable_mime_expiry_at_round_end() {
    let mut state = GameState::new(728);
    let mut mime = Joker::create_joker(JokerKind::Mime);
    mime.set_perishable();
    state.jokers.add(mime);
    for _ in 0..4 {
        state.begin_blind_as(BlindKind::Small);
        state.blind.chips_scored = state.blind.score_requirement;
        state.finish_blind(&Hand::new()).unwrap();
    }
    state.money = 0;
    state.begin_blind_as(BlindKind::Small);
    state.blind.score_requirement = 1;
    let mut hand = Hand::new();
    hand.add_card(Card::new(
        Rank::Two,
        Suit::Clubs,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));
    hand.add_card(Card::new(
        Rank::King,
        Suit::Clubs,
        Enhancement::Gold,
        Edition::None,
        Seal::Blue,
    ));
    state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!(state.money, 9, "one Gold payout + reward + unused hands");
    assert_eq!(state.consumables.len(), 1);
}

#[test]
fn tarot_can_be_used_during_play_and_empty_hand_destruction_resolves_loss() {
    let mut game = Game::new(729);
    game.apply_action(GameAction::SelectBlind(BlindKind::Small))
        .unwrap();
    game.state.deck.remove_cards(52);
    let mut only_card = Hand::new();
    only_card.add_card(Card::new(
        Rank::Two,
        Suit::Clubs,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));
    game.hand = only_card;
    game.state
        .add_consumable(Consumable::Tarot(Tarot::HangedMan))
        .unwrap();
    assert_eq!(
        game.apply_action(GameAction::UseConsumable {
            index: 0,
            target: ConsumableTarget::hand(vec![0])
        }),
        Ok(DecisionPoint::RunComplete)
    );
    assert_eq!(game.state.counters.life, 0);
    assert!(!game.state.blind.finished);
}

#[test]
fn every_deck_and_stake_can_complete_a_seeded_small_big_boss_cycle() {
    for deck in DECKS {
        for stake in STAKES {
            let mut game = Game::with_rules(740, deck, stake);
            for _ in 0..3 {
                let kind = game.available_blind();
                game.apply_action(GameAction::SelectBlind(kind)).unwrap();
                winning_hand(&mut game);
                game.apply_action(GameAction::LeaveShop).unwrap();
            }
            assert_eq!(game.state.counters.ante, 2, "{deck:?}/{stake:?}");
            assert_eq!(game.state.counters.life, 1);
            assert_eq!(game.state.blind.stake, stake);
        }
    }
}

#[test]
fn reopening_an_unplayed_shop_cannot_reconsume_a_completed_blind() {
    let mut game = Game::new(741);
    game.apply_action(GameAction::SelectBlind(BlindKind::Small))
        .unwrap();
    winning_hand(&mut game);
    game.apply_action(GameAction::LeaveShop).unwrap();
    assert_eq!(game.blind_stage, BlindStage::Big);
    game.enter_shop().unwrap();
    game.apply_action(GameAction::LeaveShop).unwrap();
    assert_eq!(game.blind_stage, BlindStage::Big);
    assert_eq!(game.state.counters.ante, 1);
}

#[test]
fn ante_eight_finisher_shop_advances_to_endless_ante_nine() {
    let mut game = Game::new(742);
    game.state.counters.ante = 8;
    game.state.regenerate_blind_selector();
    game.blind_stage = BlindStage::Boss;
    let boss = game.available_blind();
    let BlindKind::Boss(kind) = boss else {
        panic!()
    };
    assert!(BossBlinds::SHOWDOWN.contains(&kind));
    game.apply_action(GameAction::SelectBlind(boss)).unwrap();
    // Finisher targets can exceed one low-level hand; use existing Planet level upgrades.
    for hand in std::iter::once(PokerHand::HighCard).chain(PokerHand::DISCARD_HANDS) {
        game.state.upgrade_hand_score(hand, 60000, 60000);
    }
    let forced = game.state.blind.forced_card_index.unwrap_or(0) as u8;
    game.apply_action(GameAction::PlayCards(vec![forced]))
        .unwrap();
    assert_eq!(game.decision_point(), DecisionPoint::Shop);
    assert_eq!(game.state.counters.ante, 8);
    game.apply_action(GameAction::LeaveShop).unwrap();
    assert_eq!(game.state.counters.ante, 9);
    game.apply_action(GameAction::SelectBlind(BlindKind::Small))
        .unwrap();
    assert!(game.state.blind.score_requirement > 50000);
}

#[test]
fn zero_hand_budget_at_blind_entry_is_a_terminal_loss_even_with_cards() {
    let mut game = Game::new(743);
    game.state.blind.hands_per_blind = 0;
    game.apply_action(GameAction::SelectBlind(BlindKind::Small))
        .unwrap();
    assert_eq!(game.hand.cards().len(), 8);
    assert_eq!(game.decision_point(), DecisionPoint::RunComplete);
    assert_eq!(game.state.counters.life, 0);
    assert_eq!(game.state.money, 4);
    assert!(!game.state.blind.finished);
}
