use super::*;
use crate::core::enums::{Rank, Suit};
use crate::core::joker_types::JokerKind;
use rand::RngExt;

fn card(enhancement: Enhancement, edition: Edition, seal: Seal) -> Card {
    Card::new(Rank::King, Suit::Clubs, enhancement, edition, seal)
}

fn hand_of(cards: impl IntoIterator<Item = Card>) -> Hand {
    let mut hand = Hand::new();
    for card in cards {
        hand.add_card(card);
    }
    hand
}

fn state_with(kinds: &[JokerKind]) -> GameState {
    let mut state = GameState::new(501);
    state.begin_blind_as(BlindKind::Small);
    state.blind.score_requirement = u64::MAX;
    state.money = 0;
    for &kind in kinds {
        state.jokers.add(Joker::create_joker(kind));
    }
    state
}

#[test]
fn played_enhancements_and_editions_apply_in_order() {
    for (enhancement, edition, chips, mult) in [
        (Enhancement::None, Edition::None, 15, 1.0),
        (Enhancement::Bonus, Edition::Foil, 95, 1.0),
        (Enhancement::Mult, Edition::Polychrome, 15, 7.5),
        (Enhancement::Glass, Edition::Holographic, 15, 12.0),
        (Enhancement::Stone, Edition::Foil, 105, 1.0),
        (Enhancement::Steel, Edition::None, 15, 1.0),
        (Enhancement::Gold, Edition::None, 15, 1.0),
    ] {
        let mut state = state_with(&[]);
        let mut hand = hand_of([card(enhancement, edition, Seal::None)]);
        let result = state.play_hand(&mut hand, vec![0]).unwrap();
        assert_eq!((result.chips, result.mult), (chips, mult));
        assert_eq!(state.money, 0);
    }
}

#[test]
fn red_seal_repeats_card_edition_and_played_jokers_additively() {
    let mut state = state_with(&[JokerKind::HangingChad, JokerKind::SmileyFace]);
    let mut hand = hand_of([card(Enhancement::Mult, Edition::Polychrome, Seal::Red)]);
    let result = state.play_hand(&mut hand, vec![0]).unwrap();
    // Four activations: original + Red + Chad's two. Each adds 4, multiplies 1.5, adds Smiley's 5.
    assert_eq!(result.chips, 45);
    assert_eq!(result.mult, 94.4375);
}

#[test]
fn non_scoring_cards_do_not_activate_seals_enhancements_or_editions() {
    let mut state = state_with(&[]);
    let mut hand = hand_of([
        Card::new(
            Rank::Two,
            Suit::Clubs,
            Enhancement::Glass,
            Edition::Foil,
            Seal::Gold,
        ),
        card(Enhancement::None, Edition::None, Seal::None),
    ]);
    let result = state.play_hand(&mut hand, vec![0, 1]).unwrap();
    assert_eq!((result.chips, result.mult), (15, 1.0));
    assert_eq!(state.money, 0);
    assert_eq!(state.deck.discarded_cards().len(), 2);
}

#[test]
fn gold_seal_pays_on_each_scoring_activation_before_bootstraps() {
    let mut state = state_with(&[JokerKind::HangingChad, JokerKind::Bootstraps]);
    let mut hand = hand_of([card(Enhancement::None, Edition::None, Seal::Gold)]);
    let result = state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!(state.money, 9);
    assert_eq!((result.chips, result.mult), (35, 3.0));
}

#[test]
fn held_seals_do_not_pay_gold_or_generate_purple_during_scoring() {
    let mut state = state_with(&[]);
    let mut hand = hand_of([
        card(Enhancement::None, Edition::None, Seal::None),
        card(Enhancement::None, Edition::None, Seal::Gold),
        card(Enhancement::None, Edition::None, Seal::Purple),
        card(Enhancement::None, Edition::None, Seal::Blue),
    ]);
    state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!(state.money, 0);
    assert!(state.consumables.is_empty());
}

#[test]
fn held_steel_and_baron_repeat_for_red_seal_and_mime_but_editions_do_not() {
    let mut state = state_with(&[JokerKind::Mime, JokerKind::Baron]);
    let mut hand = hand_of([
        card(Enhancement::None, Edition::None, Seal::None),
        card(Enhancement::Steel, Edition::Holographic, Seal::Red),
    ]);
    let result = state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!((result.chips, result.mult), (15, 1.5_f32.powi(6)));
}

#[test]
fn held_bonus_mult_glass_and_editions_never_score_as_played_cards() {
    for enhancement in [
        Enhancement::Bonus,
        Enhancement::Mult,
        Enhancement::Glass,
        Enhancement::Stone,
    ] {
        for edition in [Edition::Foil, Edition::Holographic, Edition::Polychrome] {
            let mut state = state_with(&[]);
            let mut hand = hand_of([
                card(Enhancement::None, Edition::None, Seal::None),
                card(enhancement, edition, Seal::Red),
            ]);
            let result = state.play_hand(&mut hand, vec![0]).unwrap();
            assert_eq!((result.chips, result.mult), (15, 1.0));
            assert_eq!(hand.cards()[0].enhancement(), enhancement);
        }
    }
}

#[test]
fn plant_suppresses_every_played_and_held_card_effect() {
    for enhancement in [
        Enhancement::Bonus,
        Enhancement::Mult,
        Enhancement::Glass,
        Enhancement::Lucky,
        Enhancement::Steel,
        Enhancement::Gold,
    ] {
        for seal in [Seal::Gold, Seal::Red, Seal::Blue, Seal::Purple] {
            let mut state =
                state_with(&[JokerKind::Mime, JokerKind::OopsAll6s, JokerKind::OopsAll6s]);
            state.begin_blind_as(BlindKind::Boss(BossBlinds::Plante));
            state.blind.score_requirement = 1;
            let mut hand = hand_of([
                card(enhancement, Edition::Polychrome, seal),
                card(enhancement, Edition::Foil, seal),
            ]);
            let result = state.play_hand(&mut hand, vec![0]).unwrap();
            assert_eq!((result.chips, result.mult), (5, 1.0));
            assert_eq!(state.money, 5, "only the boss reward pays");
            assert!(state.consumables.is_empty());
            assert_eq!(state.deck.discarded_cards().len(), 1);
        }
    }
}

#[test]
fn observatory_multiplies_after_jokers_and_counts_only_matching_planets() {
    let mut state = state_with(&[JokerKind::Joker]);
    state.redeem_voucher(Vouchers::Telescope).unwrap();
    state.redeem_voucher(Vouchers::Observatory).unwrap();
    state.consumable_capacity = 3;
    state
        .add_consumable(Consumable::Planet(Planet::Pluto))
        .unwrap();
    state
        .add_consumable(Consumable::Planet(Planet::Mercury))
        .unwrap();
    state
        .add_consumable(Consumable::Planet(Planet::Pluto))
        .unwrap();
    let mut hand = hand_of([card(Enhancement::None, Edition::None, Seal::None)]);
    let result = state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!(result.mult, 5.0 * 2.25);
    assert_eq!(state.consumables.len(), 3);
}

#[test]
fn planets_without_observatory_do_not_multiply_scores() {
    let mut state = state_with(&[]);
    state
        .add_consumable(Consumable::Planet(Planet::Pluto))
        .unwrap();
    let mut hand = hand_of([card(Enhancement::None, Edition::None, Seal::None)]);
    assert_eq!(state.play_hand(&mut hand, vec![0]).unwrap().mult, 1.0);
}

#[test]
fn gold_and_blue_round_effects_use_held_cards_before_refill_and_pay_once() {
    let mut state = state_with(&[]);
    state.money = 2;
    state.blind.score_requirement = 1;
    let mut hand = hand_of([
        card(Enhancement::None, Edition::None, Seal::None),
        card(Enhancement::Gold, Edition::None, Seal::Blue),
    ]);
    let deck_size = state.deck.size();
    state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!(hand.hand_size(), 1);
    assert_eq!(state.deck.size(), deck_size);
    assert_eq!(
        state.money, 9,
        "Gold pays before interest: 2 + 3 + 3 reward + 1 interest"
    );
    assert_eq!(
        state.consumables[0].consumable,
        Consumable::Planet(Planet::Pluto)
    );
    assert!(state.finish_blind(&hand).is_none());
    state.resolve_blind_end(&mut hand);
    assert_eq!(state.money, 9);
    assert_eq!(state.consumables.len(), 1);
}

#[test]
fn winning_hand_does_not_pay_newly_drawn_gold_cards_or_blue_seals() {
    let mut state = state_with(&[]);
    for index in 0..usize::from(state.deck.size()) {
        let card = state.deck.card_mut(index).unwrap();
        card.set_enhancement(Enhancement::Gold);
        card.set_seal(Seal::Blue);
    }
    state.blind.score_requirement = 1;
    let mut hand = hand_of([card(Enhancement::Gold, Edition::None, Seal::Blue)]);
    state.play_hand(&mut hand, vec![0]).unwrap();
    assert!(hand.cards().is_empty());
    assert_eq!(state.money, 3);
    assert!(state.consumables.is_empty());
}

#[test]
fn gold_red_round_effects_repeat_with_mime() {
    let mut state = state_with(&[JokerKind::Mime]);
    state.blind.score_requirement = 1;
    let mut hand = hand_of([
        card(Enhancement::None, Edition::None, Seal::None),
        card(Enhancement::Gold, Edition::Polychrome, Seal::Red),
    ]);
    state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!(state.money, 13, "three Gold payouts + reward + interest");
}

#[test]
fn blue_seal_uses_last_hand_not_most_played_and_mime_obeys_capacity() {
    let mut state = state_with(&[JokerKind::Mime, JokerKind::Mime]);
    state.counters.poker_hands_played[PokerHand::Flush as usize - 1] = 20;
    state.consumable_capacity = 2;
    state.blind.score_requirement = 1;
    let mut hand = hand_of([
        card(Enhancement::None, Edition::None, Seal::None),
        Card::new(
            Rank::King,
            Suit::Hearts,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ),
        card(Enhancement::None, Edition::None, Seal::Blue),
    ]);
    state.play_hand(&mut hand, vec![0, 1]).unwrap();
    assert_eq!(state.consumables.len(), 2);
    assert!(
        state
            .consumables
            .iter()
            .all(|owned| owned.consumable == Consumable::Planet(Planet::Mercury))
    );
}

#[test]
fn blue_seal_with_full_inventory_or_no_last_hand_generates_nothing() {
    for full in [false, true] {
        let mut state = state_with(&[]);
        if full {
            state.consumable_capacity = 0;
        }
        state.blind.chips_scored = state.blind.score_requirement;
        let hand = hand_of([card(Enhancement::Gold, Edition::None, Seal::Blue)]);
        if full {
            state.blind.current_hand = Some(PokerHand::Pair);
        }
        state.finish_blind(&hand).unwrap();
        assert!(state.consumables.is_empty());
    }
}

#[test]
fn losing_round_has_no_gold_or_blue_effects_but_mr_bones_save_does() {
    for saved in [false, true] {
        let mut state = state_with(if saved { &[JokerKind::MrBones] } else { &[] });
        state.blind.hands_remaining = 1;
        state.blind.score_requirement = 60;
        let mut hand = hand_of([
            card(Enhancement::None, Edition::None, Seal::None),
            card(Enhancement::Gold, Edition::None, Seal::Blue),
        ]);
        state.play_hand(&mut hand, vec![0]).unwrap();
        assert_eq!(state.blind.finished, saved);
        assert_eq!(state.consumables.len(), usize::from(saved));
        assert_eq!(state.money, if saved { 6 } else { 0 });
    }
}

#[test]
fn purple_seals_generate_once_per_discard_and_obey_capacity_and_duplicates() {
    let mut state = state_with(&[JokerKind::Mime]);
    state
        .add_consumable(Consumable::Tarot(Tarot::Fool))
        .unwrap();
    let mut hand = hand_of([
        card(Enhancement::None, Edition::None, Seal::Purple),
        card(Enhancement::None, Edition::None, Seal::Purple),
    ]);
    state.discard_selected_cards(&mut hand, vec![0, 1]).unwrap();
    assert_eq!(state.consumables.len(), 2);
    assert!(
        matches!(state.consumables[1].consumable, Consumable::Tarot(tarot) if tarot != Tarot::Fool)
    );
    assert_eq!(state.blind.discards_used, 1);
}

#[test]
fn purple_generation_is_seeded_and_invalid_discard_is_inert() {
    let mut states = [state_with(&[]), state_with(&[])];
    let mut hands = [
        hand_of([card(Enhancement::None, Edition::None, Seal::Purple)]),
        hand_of([card(Enhancement::None, Edition::None, Seal::Purple)]),
    ];
    assert!(
        states[0]
            .discard_selected_cards(&mut hands[0], vec![9])
            .is_none()
    );
    for index in 0..2 {
        states[index]
            .discard_selected_cards(&mut hands[index], vec![0])
            .unwrap();
    }
    assert_eq!(
        states[0].consumables[0].consumable,
        states[1].consumables[0].consumable
    );
    assert_eq!(states[0].rng.random::<u64>(), states[1].rng.random::<u64>());
}

#[test]
fn purple_is_suppressed_by_debuff_and_full_capacity_without_rng_rolls() {
    for debuffed in [false, true] {
        let mut state = state_with(&[]);
        let mut control = state_with(&[]);
        if !debuffed {
            state.consumable_capacity = 0;
            control.consumable_capacity = 0;
        }
        let mut purple = card(Enhancement::None, Edition::None, Seal::Purple);
        purple.set_debuffed(debuffed);
        let mut hand = hand_of([purple]);
        let mut control_hand = hand_of([card(Enhancement::None, Edition::None, Seal::None)]);
        state.discard_selected_cards(&mut hand, vec![0]).unwrap();
        control
            .discard_selected_cards(&mut control_hand, vec![0])
            .unwrap();
        assert!(state.consumables.is_empty());
        assert_eq!(state.rng.random::<u64>(), control.rng.random::<u64>());
    }
}

#[test]
fn hook_discards_trigger_purple_without_using_a_discard() {
    let mut state = state_with(&[]);
    state.begin_blind_as(BlindKind::Boss(BossBlinds::Hook));
    let mut hand = hand_of([
        card(Enhancement::None, Edition::None, Seal::Purple),
        card(Enhancement::None, Edition::None, Seal::Purple),
    ]);
    assert_eq!(state.apply_hook_discard(&mut hand).len(), 2);
    assert_eq!(state.consumables.len(), 2);
    assert_eq!(state.blind.discards_used, 0);
}

#[test]
fn played_purple_and_blue_seals_do_not_generate_consumables() {
    for seal in [Seal::Purple, Seal::Blue] {
        let mut state = state_with(&[]);
        state.blind.score_requirement = 1;
        let mut hand = hand_of([card(Enhancement::None, Edition::None, seal)]);
        state.play_hand(&mut hand, vec![0]).unwrap();
        assert!(state.consumables.is_empty());
    }
}

#[test]
fn lucky_rolls_are_independent_seeded_and_repeat_per_red_activation() {
    let mut saw = [false; 4];
    for seed in 0..200 {
        let mut state = state_with(&[JokerKind::LuckyCat]);
        state.rng = StdRng::seed_from_u64(seed);
        state.blind.hands_remaining = 1;
        let mut expected_rng = StdRng::seed_from_u64(seed);
        let mut mult = 1.0;
        let mut money = 0;
        let mut cat = 1.0;
        for _ in 0..2 {
            let mult_success = expected_rng.random_bool(1.0 / 5.0);
            let money_success = expected_rng.random_bool(1.0 / 15.0);
            saw[usize::from(mult_success) + 2 * usize::from(money_success)] = true;
            if mult_success {
                mult += 20.0;
            }
            if money_success {
                money += 20;
            }
            if mult_success || money_success {
                cat += 0.25;
            }
        }
        let mut hand = hand_of([card(Enhancement::Lucky, Edition::None, Seal::Red)]);
        let result = state.play_hand(&mut hand, vec![0]).unwrap();
        assert_eq!(
            (result.chips, result.mult, state.money),
            (25, mult * cat, money)
        );
        assert_eq!(state.rng.random::<u64>(), expected_rng.random::<u64>());
    }
    assert_eq!(saw, [true; 4]);
}

#[test]
fn oops_doubles_lucky_odds_and_copied_cat_grows_only_once() {
    let mut state = state_with(&[JokerKind::Blueprint, JokerKind::LuckyCat]);
    for _ in 0..4 {
        state.jokers.add(Joker::create_joker(JokerKind::OopsAll6s));
    }
    state.blind.hands_remaining = 1;
    let mut hand = hand_of([card(Enhancement::Lucky, Edition::None, Seal::Red)]);
    let result = state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!(state.money, 40);
    assert_eq!(result.mult, 41.0 * 1.5 * 1.5);
}

#[test]
fn glass_destruction_is_once_per_card_after_score_and_updates_scalers() {
    let mut state = state_with(&[
        JokerKind::Glass,
        JokerKind::Canio,
        JokerKind::HangingChad,
        JokerKind::OopsAll6s,
        JokerKind::OopsAll6s,
    ]);
    state.blind.hands_remaining = 2;
    let mut hand = hand_of([
        card(Enhancement::Glass, Edition::None, Seal::Red),
        card(Enhancement::None, Edition::None, Seal::None),
    ]);
    let result = state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!(
        result.mult, 16.0,
        "four Glass activations, scaler growth follows scoring"
    );
    assert!(state.deck.discarded_cards().is_empty());
    let result = state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!(result.mult, 1.75 * 2.0);
    assert_eq!(state.deck.discarded_cards().len(), 1);
}

#[test]
fn glass_roll_is_seeded_and_oops_doubles_probability() {
    for doubled in [false, true] {
        let mut broke = false;
        let mut survived = false;
        for seed in 0..30 {
            let mut state = state_with(if doubled {
                &[JokerKind::OopsAll6s]
            } else {
                &[]
            });
            state.blind.hands_remaining = 1;
            state.rng = StdRng::seed_from_u64(seed);
            let mut expected_rng = StdRng::seed_from_u64(seed);
            let expected_break = expected_rng.random_bool(if doubled { 0.5 } else { 0.25 });
            let mut hand = hand_of([card(Enhancement::Glass, Edition::None, Seal::Red)]);
            state.play_hand(&mut hand, vec![0]).unwrap();
            assert_eq!(state.deck.discarded_cards().is_empty(), expected_break);
            assert_eq!(state.rng.random::<u64>(), expected_rng.random::<u64>());
            broke |= expected_break;
            survived |= !expected_break;
        }
        assert!(broke && survived);
    }
}

#[test]
fn glass_non_scoring_and_whole_hand_debuff_do_not_roll_destruction() {
    let mut state = state_with(&[JokerKind::OopsAll6s, JokerKind::OopsAll6s]);
    state.begin_blind_as(BlindKind::Boss(BossBlinds::Psychich));
    state.blind.hands_remaining = 1;
    let mut hand = hand_of([card(Enhancement::Glass, Edition::None, Seal::Red)]);
    let seed = 503;
    state.rng = StdRng::seed_from_u64(seed);
    let mut expected_rng = StdRng::seed_from_u64(seed);
    assert_eq!(state.play_hand(&mut hand, vec![0]).unwrap().score, 0.0);
    assert_eq!(state.deck.discarded_cards().len(), 1);
    assert_eq!(state.rng.random::<u64>(), expected_rng.random::<u64>());
}

#[test]
fn stone_scores_without_rank_suit_or_low_rank_retrigger() {
    let mut state = state_with(&[JokerKind::Hack, JokerKind::Scholar, JokerKind::Greedy]);
    let mut hand = hand_of([
        Card::new(
            Rank::Two,
            Suit::Diamonds,
            Enhancement::Stone,
            Edition::None,
            Seal::None,
        ),
        Card::new(
            Rank::Ace,
            Suit::Diamonds,
            Enhancement::Stone,
            Edition::None,
            Seal::None,
        ),
        card(Enhancement::None, Edition::None, Seal::None),
    ]);
    let result = state.play_hand(&mut hand, vec![0, 1, 2]).unwrap();
    assert_eq!(result.hand, PokerHand::HighCard);
    assert_eq!((result.chips, result.mult), (115, 1.0));
    assert_eq!(result.scoring_data.scoring_played_cards.len(), 3);
}

#[test]
fn all_stone_hand_is_high_card_and_stones_never_create_a_pair_or_flush() {
    let mut state = state_with(&[]);
    let mut hand = hand_of((0..5).map(|_| card(Enhancement::Stone, Edition::None, Seal::Red)));
    let result = state.play_hand(&mut hand, vec![0, 1, 2, 3, 4]).unwrap();
    assert_eq!(result.hand, PokerHand::HighCard);
    assert_eq!(result.chips, 505);
}

#[test]
fn wild_activates_all_suit_scoring_and_economy_jokers() {
    let mut state = state_with(&[
        JokerKind::Greedy,
        JokerKind::Lusty,
        JokerKind::Wrathful,
        JokerKind::Gluttonous,
        JokerKind::RoughGem,
    ]);
    let mut hand = hand_of([card(Enhancement::Wild, Edition::None, Seal::Red)]);
    let result = state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!(result.mult, 25.0);
    assert_eq!(state.money, 2);
}

#[test]
fn wild_bloodstone_arrowhead_and_onyx_apply_their_suit_effects() {
    let mut state = state_with(&[
        JokerKind::OopsAll6s,
        JokerKind::Bloodstone,
        JokerKind::Arrowhead,
        JokerKind::OnyxAgate,
    ]);
    let mut hand = hand_of([card(Enhancement::Wild, Edition::None, Seal::None)]);
    let result = state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!((result.chips, result.mult), (65, 8.5));
}

#[test]
fn flower_pot_uses_each_wild_once_and_requires_four_scoring_cards() {
    for count in [3, 4] {
        let mut state = state_with(&[JokerKind::FlowerPot, JokerKind::Splash]);
        let mut hand =
            hand_of((0..count).map(|_| card(Enhancement::Wild, Edition::None, Seal::None)));
        let result = state.play_hand(&mut hand, (0..count).collect()).unwrap();
        assert_eq!(result.mult, if count == 4 { 21.0 } else { 3.0 });
    }
}

#[test]
fn seeing_double_needs_two_cards_even_if_the_first_is_wild() {
    for count in [1, 2] {
        let mut state = state_with(&[JokerKind::SeeingDouble]);
        let mut hand =
            hand_of((0..count).map(|_| card(Enhancement::Wild, Edition::None, Seal::None)));
        let result = state.play_hand(&mut hand, (0..count).collect()).unwrap();
        assert_eq!(result.mult, if count == 2 { 4.0 } else { 1.0 });
    }
}

#[test]
fn blackboard_accepts_held_wild_but_not_stone() {
    for enhancement in [Enhancement::Wild, Enhancement::Stone] {
        let mut state = state_with(&[JokerKind::Blackboard]);
        let mut hand = hand_of([
            card(Enhancement::None, Edition::None, Seal::None),
            Card::new(
                Rank::Two,
                Suit::Hearts,
                enhancement,
                Edition::None,
                Seal::None,
            ),
        ]);
        let result = state.play_hand(&mut hand, vec![0]).unwrap();
        assert_eq!(
            result.mult,
            if enhancement == Enhancement::Wild {
                3.0
            } else {
                1.0
            }
        );
    }
}

#[test]
fn debuffed_wild_keeps_printed_suit_for_detection_but_not_wild_suit_effects() {
    let mut state = state_with(&[JokerKind::Greedy, JokerKind::RoughGem]);
    state.begin_blind_as(BlindKind::Boss(BossBlinds::Plante));
    let mut hand = hand_of([
        Card::new(
            Rank::Two,
            Suit::Diamonds,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ),
        Card::new(
            Rank::Four,
            Suit::Diamonds,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ),
        Card::new(
            Rank::Six,
            Suit::Diamonds,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ),
        Card::new(
            Rank::Eight,
            Suit::Diamonds,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ),
        card(Enhancement::Wild, Edition::None, Seal::None),
    ]);
    let result = state.play_hand(&mut hand, vec![0, 1, 2, 3, 4]).unwrap();
    assert_eq!(result.hand, PokerHand::HighCard);
    assert_eq!(result.mult, 1.0);
    assert_eq!(state.money, 0);
}

#[test]
fn card_ids_preserve_distinct_seal_enhancement_rank_and_suit_properties() {
    let mut ids = std::collections::HashSet::new();
    for rank in ALL_RANKS {
        for suit in ALL_SUITS {
            for enhancement in [
                Enhancement::None,
                Enhancement::Bonus,
                Enhancement::Mult,
                Enhancement::Wild,
                Enhancement::Glass,
                Enhancement::Steel,
                Enhancement::Stone,
                Enhancement::Gold,
                Enhancement::Lucky,
            ] {
                for seal in [Seal::None, Seal::Gold, Seal::Red, Seal::Blue, Seal::Purple] {
                    for edition in [
                        Edition::None,
                        Edition::Foil,
                        Edition::Holographic,
                        Edition::Polychrome,
                    ] {
                        let card = Card::new(rank, suit, enhancement, edition, seal);
                        assert!(ids.insert(card.id()));
                        assert_eq!(Rank::from_card_id(card.id()), rank);
                    }
                }
            }
        }
    }
}

#[test]
fn lucky_base_and_doubled_odds_consume_two_independent_rolls() {
    for doubled in [false, true] {
        for seed in 0..40 {
            let mut state = state_with(if doubled {
                &[JokerKind::OopsAll6s]
            } else {
                &[]
            });
            state.blind.hands_remaining = 1;
            state.rng = StdRng::seed_from_u64(seed);
            let mut oracle = StdRng::seed_from_u64(seed);
            let factor = if doubled { 2.0 } else { 1.0 };
            let mult_success = oracle.random_bool(factor / 5.0);
            let money_success = oracle.random_bool(factor / 15.0);
            let mut hand = hand_of([card(Enhancement::Lucky, Edition::Polychrome, Seal::Gold)]);
            let score = state.play_hand(&mut hand, vec![0]).unwrap();
            assert_eq!(score.mult, (if mult_success { 21.0 } else { 1.0 }) * 1.5);
            assert_eq!(state.money, 3 + if money_success { 20 } else { 0 });
            assert_eq!(state.rng.random::<u64>(), oracle.random::<u64>());
        }
    }
}

#[test]
fn gold_round_card_effects_follow_round_joker_payouts_and_precede_interest() {
    let mut state = state_with(&[JokerKind::Golden]);
    state.blind.score_requirement = 1;
    let mut hand = hand_of([
        card(Enhancement::None, Edition::None, Seal::None),
        card(Enhancement::Gold, Edition::None, Seal::None),
    ]);
    state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!(
        state.money, 11,
        "Golden 4 + held Gold 3 + interest 1 + reward 3"
    );
}

#[test]
fn ordinary_discard_never_pays_gold_or_blue_round_effects() {
    let mut state = state_with(&[]);
    let mut hand = hand_of([card(Enhancement::Gold, Edition::Foil, Seal::Blue)]);
    state.discard_selected_cards(&mut hand, vec![0]).unwrap();
    assert_eq!(state.money, 0);
    assert!(state.consumables.is_empty());
}

#[test]
fn blue_seal_generation_after_scoring_does_not_retroactively_activate_observatory() {
    let mut state = state_with(&[]);
    state.redeem_voucher(Vouchers::Telescope).unwrap();
    state.redeem_voucher(Vouchers::Observatory).unwrap();
    state.blind.score_requirement = 1;
    let mut hand = hand_of([
        card(Enhancement::None, Edition::None, Seal::None),
        card(Enhancement::None, Edition::None, Seal::Blue),
    ]);
    assert_eq!(state.play_hand(&mut hand, vec![0]).unwrap().mult, 1.0);
    assert_eq!(
        state.consumables[0].consumable,
        Consumable::Planet(Planet::Pluto)
    );
    state.begin_blind_as(BlindKind::Big);
    state.blind.score_requirement = u64::MAX;
    hand.add_card(card(Enhancement::None, Edition::None, Seal::None));
    assert_eq!(state.play_hand(&mut hand, vec![1]).unwrap().mult, 1.5);
}

#[test]
fn purple_uses_showman_eligibility_and_exhausted_tarot_pool_fallback() {
    for showman in [false, true] {
        let mut observed_duplicate = false;
        for seed in 0..30 {
            let mut state = state_with(if showman { &[JokerKind::Showman] } else { &[] });
            state.consumable_capacity = Tarot::ALL.len() + 1;
            for tarot in Tarot::ALL {
                if tarot != Tarot::Sun {
                    state.add_consumable(Consumable::Tarot(tarot)).unwrap();
                }
            }
            state.rng = StdRng::seed_from_u64(seed);
            let mut hand = hand_of([card(Enhancement::None, Edition::None, Seal::Purple)]);
            state.discard_selected_cards(&mut hand, vec![0]).unwrap();
            let generated = state.consumables.last().unwrap().consumable;
            if showman {
                observed_duplicate |= generated != Consumable::Tarot(Tarot::Sun);
            } else {
                assert_eq!(generated, Consumable::Tarot(Tarot::Sun));
            }
        }
        assert_eq!(observed_duplicate, showman);
    }
    let mut state = state_with(&[]);
    state.consumable_capacity = Tarot::ALL.len() + 1;
    for tarot in Tarot::ALL {
        state.add_consumable(Consumable::Tarot(tarot)).unwrap();
    }
    let mut hand = hand_of([card(Enhancement::None, Edition::None, Seal::Purple)]);
    state.discard_selected_cards(&mut hand, vec![0]).unwrap();
    assert_eq!(
        state.consumables.last().unwrap().consumable,
        Consumable::Tarot(Tarot::Fool)
    );
}

#[test]
fn glass_destruction_removes_the_physical_card_from_the_run() {
    let mut state = state_with(&[JokerKind::OopsAll6s, JokerKind::OopsAll6s]);
    let mut hand = Hand::new();
    state.deal_cards(&mut hand);
    hand.cards_mut()[0].set_enhancement(Enhancement::Glass);
    hand.cards_mut()[0].set_seal(Seal::Red);
    state.blind.score_requirement = 1;
    assert_eq!(usize::from(state.deck.size()) + hand.cards().len(), 52);
    state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!(
        usize::from(state.deck.size()) + hand.cards().len() + state.deck.discarded_cards().len(),
        51
    );
    state.deck.reset_deck();
    assert_eq!(usize::from(state.deck.size()) + hand.cards().len(), 51);
}

#[test]
fn dna_copy_retains_blue_gold_and_scores_held_steel_red_during_same_hand() {
    let mut state = state_with(&[JokerKind::DNA]);
    state.blind.score_requirement = 1;
    let mut hand = hand_of([card(Enhancement::Gold, Edition::None, Seal::Blue)]);
    state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!(hand.cards().len(), 1);
    assert_eq!(state.money, 6);
    assert_eq!(state.consumables.len(), 1);

    let mut state = state_with(&[JokerKind::DNA]);
    let mut hand = hand_of([card(Enhancement::Steel, Edition::None, Seal::Red)]);
    let score = state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!((score.chips, score.mult), (25, 2.25));
}

#[test]
fn wild_and_smeared_conditions_use_distinct_cards_for_flower_pot() {
    let mut state = state_with(&[JokerKind::FlowerPot, JokerKind::Splash, JokerKind::Smeared]);
    let mut hand = hand_of([
        Card::new(
            Rank::Two,
            Suit::Hearts,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ),
        Card::new(
            Rank::Four,
            Suit::Hearts,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ),
        Card::new(
            Rank::Six,
            Suit::Clubs,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ),
        Card::new(
            Rank::Eight,
            Suit::Hearts,
            Enhancement::Wild,
            Edition::None,
            Seal::None,
        ),
    ]);
    assert_eq!(
        state.play_hand(&mut hand, vec![0, 1, 2, 3]).unwrap().mult,
        3.0
    );
}

#[test]
fn flower_pot_counts_debuffed_printed_suits_but_not_debuffed_wilds() {
    for wild in [false, true] {
        let mut state = state_with(&[JokerKind::FlowerPot, JokerKind::Splash]);
        state.begin_blind_as(BlindKind::Boss(BossBlinds::Plante));
        state.blind.score_requirement = u64::MAX;
        let mut hand = hand_of([
            Card::new(
                Rank::King,
                Suit::Hearts,
                if wild {
                    Enhancement::Wild
                } else {
                    Enhancement::None
                },
                Edition::None,
                Seal::None,
            ),
            Card::new(
                Rank::Two,
                Suit::Diamonds,
                Enhancement::None,
                Edition::None,
                Seal::None,
            ),
            Card::new(
                Rank::Four,
                Suit::Clubs,
                Enhancement::None,
                Edition::None,
                Seal::None,
            ),
            Card::new(
                Rank::Six,
                Suit::Spades,
                Enhancement::None,
                Edition::None,
                Seal::None,
            ),
        ]);
        let score = state.play_hand(&mut hand, vec![0, 1, 2, 3]).unwrap();
        assert_eq!(score.mult, if wild { 1.0 } else { 3.0 });
    }
}
