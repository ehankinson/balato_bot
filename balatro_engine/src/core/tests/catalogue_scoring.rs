use super::*;
use crate::core::consumable::ConsumableTarget;
use crate::core::enums::{Rank, Suit};
use crate::core::joker_types::{JokerEdition, JokerKind};

fn card(rank: Rank, suit: Suit) -> Card {
    Card::new(rank, suit, Enhancement::None, Edition::None, Seal::None)
}
fn hand(cards: impl IntoIterator<Item = Card>) -> Hand {
    let mut hand = Hand::new();
    for card in cards {
        hand.add_card(card);
    }
    hand
}
fn state(kinds: &[JokerKind]) -> GameState {
    let mut state = GameState::new(801);
    state.begin_blind_as(BlindKind::Small);
    state.blind.score_requirement = u64::MAX;
    state.money = 0;
    for &kind in kinds {
        state.jokers.add(Joker::create_joker(kind));
    }
    state
}
fn near(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.0001,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn tr05_photograph_chad_and_red_seal_repeat_additively() {
    for red in [false, true] {
        let mut s = state(&[JokerKind::Photograph, JokerKind::HangingChad]);
        let mut king = card(Rank::King, Suit::Clubs);
        if red {
            king.set_seal(Seal::Red);
        }
        let score = s.play_hand(&mut hand([king]), vec![0]).unwrap();
        assert_eq!(
            (score.chips, score.mult, score.score),
            if red {
                (45, 16.0, 720.0)
            } else {
                (35, 8.0, 280.0)
            }
        );
    }
}

#[test]
fn tr07_hiker_grows_only_the_scored_physical_card_and_survives_redraw() {
    let mut s = state(&[JokerKind::HangingChad, JokerKind::Hiker]);
    let mut h = hand([card(Rank::Two, Suit::Clubs), card(Rank::Two, Suit::Clubs)]);
    s.play_hand(&mut h, vec![0]).unwrap();
    assert_eq!(
        s.deck.discarded_cards()[0].scoring_values().chips,
        17,
        "rank 2 plus permanent 15"
    );
    assert_eq!(
        h.cards()[0].scoring_values().chips,
        2,
        "identical unplayed card has no bonus"
    );
    s.deck
        .discard_cards(h.select_cards((0..h.hand_size()).collect()));
    s.deck.reset_deck();
    s.begin_blind_as(BlindKind::Big);
    s.blind.score_requirement = u64::MAX;
    let original = s
        .deck
        .cards()
        .iter()
        .position(|c| c.was_played_in_ante(1))
        .unwrap();
    let mut h = hand([s.deck.remove_card_at(original).unwrap()]);
    assert_eq!(s.play_hand(&mut h, vec![0]).unwrap().chips, 71);
}

#[test]
fn tr08_wee_grows_for_each_scoring_two_but_not_other_ranks_or_kickers() {
    for condition in 0..3 {
        let mut s = state(&[JokerKind::HangingChad, JokerKind::Wee]);
        let mut cards = vec![card(
            if condition == 1 {
                Rank::Three
            } else {
                Rank::Two
            },
            Suit::Clubs,
        )];
        if condition == 2 {
            cards.push(card(Rank::King, Suit::Hearts));
        }
        let mut h = hand(cards);
        let positions = (0..h.hand_size()).collect();
        let score = s.play_hand(&mut h, positions).unwrap();
        assert_eq!(
            score.chips,
            match condition {
                0 => 35,
                1 => 14,
                _ => 35,
            }
        );
    }
}

#[test]
fn tr09_hack_chad_red_and_final_hand_dusk_add_their_repetitions() {
    for final_hand in [false, true] {
        let mut s = state(&[JokerKind::Hack, JokerKind::HangingChad, JokerKind::Dusk]);
        if final_hand {
            s.blind.hands_remaining = 1;
        }
        let mut two = card(Rank::Two, Suit::Clubs);
        two.set_seal(Seal::Red);
        let score = s.play_hand(&mut hand([two]), vec![0]).unwrap();
        assert_eq!(
            (score.chips, score.mult),
            (if final_hand { 17 } else { 15 }, 1.0)
        );
    }
}

#[test]
fn sc02_smiley_photograph_order_is_repeated_per_chad_activation() {
    for reversed in [false, true] {
        let mut s = state(if reversed {
            &[
                JokerKind::Photograph,
                JokerKind::SmileyFace,
                JokerKind::HangingChad,
            ]
        } else {
            &[
                JokerKind::SmileyFace,
                JokerKind::Photograph,
                JokerKind::HangingChad,
            ]
        });
        let score = s
            .play_hand(&mut hand([card(Rank::King, Suit::Clubs)]), vec![0])
            .unwrap();
        assert_eq!(
            (score.chips, score.mult, score.score),
            if reversed {
                (35, 43.0, 1505.0)
            } else {
                (35, 78.0, 2730.0)
            }
        );
    }
}

#[test]
fn sc03_chad_repeats_ticket_payout_without_repeating_midas_transformation() {
    for reversed in [false, true] {
        let mut s = state(if reversed {
            &[
                JokerKind::GoldenTicket,
                JokerKind::MidasMask,
                JokerKind::HangingChad,
            ]
        } else {
            &[
                JokerKind::MidasMask,
                JokerKind::GoldenTicket,
                JokerKind::HangingChad,
            ]
        });
        s.play_hand(&mut hand([card(Rank::King, Suit::Clubs)]), vec![0])
            .unwrap();
        assert_eq!(s.money, 12);
        assert_eq!(s.deck.discarded_cards()[0].enhancement(), Enhancement::Gold);
    }
}

#[test]
fn sc05_midas_vampire_order_preserves_seals_and_editions() {
    for reversed in [false, true] {
        let mut s = state(if reversed {
            &[JokerKind::Vampire, JokerKind::MidasMask]
        } else {
            &[JokerKind::MidasMask, JokerKind::Vampire]
        });
        let mut king = card(Rank::King, Suit::Clubs);
        king.set_seal(Seal::Red);
        king.set_edition(Edition::Foil);
        let score = s.play_hand(&mut hand([king]), vec![0]).unwrap();
        near(score.mult, if reversed { 1.0 } else { 1.1 });
        let result = &s.deck.discarded_cards()[0];
        assert_eq!(
            result.enhancement(),
            if reversed {
                Enhancement::Gold
            } else {
                Enhancement::None
            }
        );
        assert_eq!(
            (result.seal(), result.edition()),
            (Seal::Red, Edition::Foil)
        );
    }
}

#[test]
fn sc06_dna_copies_the_enhancement_visible_at_its_position_before_vampire() {
    for reversed in [false, true] {
        let mut s = state(if reversed {
            &[JokerKind::Vampire, JokerKind::DNA, JokerKind::HangingChad]
        } else {
            &[JokerKind::DNA, JokerKind::Vampire, JokerKind::HangingChad]
        });
        let mut two = card(Rank::Two, Suit::Clubs);
        two.set_enhancement(Enhancement::Bonus);
        let mut h = hand([two]);
        let score = s.play_hand(&mut h, vec![0]).unwrap();
        assert_eq!(s.deck.discarded_cards()[0].enhancement(), Enhancement::None);
        assert_eq!(
            h.cards()[0].enhancement(),
            if reversed {
                Enhancement::None
            } else {
                Enhancement::Bonus
            }
        );
        near(score.mult, 1.1);
    }
}

#[test]
fn sc08_space_and_its_copy_upgrade_before_reading_base_hand_values() {
    for copied in [false, true] {
        let mut kinds = vec![JokerKind::Space, JokerKind::OopsAll6s, JokerKind::OopsAll6s];
        if copied {
            kinds.insert(0, JokerKind::Blueprint);
        }
        let mut s = state(&kinds);
        let score = s
            .play_hand(&mut hand([card(Rank::Two, Suit::Clubs)]), vec![0])
            .unwrap();
        assert_eq!(
            (score.chips, score.mult, score.score),
            if copied {
                (27, 3.0, 81.0)
            } else {
                (17, 2.0, 34.0)
            }
        );
    }
}

#[test]
fn he02_copying_mime_and_baron_changes_different_parts_of_held_repetition() {
    for copy_baron in [false, true] {
        let mut s = state(if copy_baron {
            &[JokerKind::Mime, JokerKind::Blueprint, JokerKind::Baron]
        } else {
            &[JokerKind::Baron, JokerKind::Blueprint, JokerKind::Mime]
        });
        let mut king = card(Rank::King, Suit::Clubs);
        king.set_enhancement(Enhancement::Steel);
        king.set_seal(Seal::Red);
        let score = s
            .play_hand(&mut hand([card(Rank::Two, Suit::Hearts), king]), vec![0])
            .unwrap();
        near(score.mult, if copy_baron { 38.44336 } else { 25.628906 });
    }
}

#[test]
fn he03_held_queen_king_card_order_is_preserved_in_both_joker_orders() {
    for reverse_jokers in [false, true] {
        for reverse_cards in [false, true] {
            let mut s = state(if reverse_jokers {
                &[JokerKind::Baron, JokerKind::ShootTheMoon]
            } else {
                &[JokerKind::ShootTheMoon, JokerKind::Baron]
            });
            let mut held = vec![
                card(Rank::Queen, Suit::Clubs),
                card(Rank::King, Suit::Clubs),
            ];
            if reverse_cards {
                held.reverse();
            }
            let mut cards = vec![card(Rank::Two, Suit::Hearts)];
            cards.extend(held);
            let score = s.play_hand(&mut hand(cards), vec![0]).unwrap();
            near(score.mult, if reverse_cards { 14.5 } else { 21.0 });
        }
    }
}

#[test]
fn he05_blackboard_is_independent_of_mime_repetitions_and_accepts_empty_hand() {
    for condition in 0..3 {
        let mut s = state(&[JokerKind::Mime, JokerKind::Blackboard]);
        let mut cards = vec![card(Rank::Two, Suit::Hearts)];
        if condition < 2 {
            cards.push(card(
                Rank::Seven,
                if condition == 1 {
                    Suit::Hearts
                } else {
                    Suit::Clubs
                },
            ));
        }
        let score = s.play_hand(&mut hand(cards), vec![0]).unwrap();
        near(score.mult, if condition == 1 { 1.0 } else { 3.0 });
    }
}

#[test]
fn co04_copied_stuntman_scores_twice_but_its_hand_size_penalty_is_single() {
    let mut s = state(&[JokerKind::Blueprint, JokerKind::Stuntman]);
    assert_eq!(s.hand_size(), 6);
    let score = s
        .play_hand(&mut hand([card(Rank::Two, Suit::Clubs)]), vec![0])
        .unwrap();
    assert_eq!(score.chips, 507);
    s.sell_joker(0).unwrap();
    assert_eq!(s.hand_size(), 6);
    s.sell_joker(0).unwrap();
    assert_eq!(s.hand_size(), 8);
}

#[test]
fn co05_burnt_copy_only_levels_the_first_discard_and_resets_next_round() {
    let mut s = state(&[JokerKind::Blueprint, JokerKind::BurntJoker]);
    for round in 0..2 {
        if round > 0 {
            s.begin_blind_as(BlindKind::Big);
        }
        for action in 0..2 {
            let mut h = hand([
                card(Rank::Seven, Suit::Clubs),
                card(Rank::Seven, Suit::Hearts),
            ]);
            s.discard_selected_cards(&mut h, vec![0, 1]).unwrap();
            assert_eq!(
                s.hand_score(PokerHand::Pair),
                HandScore {
                    chips: 40 + 30 * round,
                    mult: 4 + 2 * round
                },
                "action {action}, round {round}"
            );
        }
    }
}

#[test]
fn su02_pareidolia_canio_counts_destroyed_numbered_cards_not_joker_sales() {
    for pareidolia in [false, true] {
        let mut s = state(if pareidolia {
            &[JokerKind::Pareidolia, JokerKind::Canio, JokerKind::Joker]
        } else {
            &[JokerKind::Canio, JokerKind::Joker]
        });
        let mut h = hand([
            card(Rank::Two, Suit::Clubs),
            card(Rank::Three, Suit::Hearts),
            card(Rank::Seven, Suit::Spades),
        ]);
        s.add_consumable(Consumable::Tarot(Tarot::HangedMan))
            .unwrap();
        s.use_consumable(0, &mut h, ConsumableTarget::hand(vec![0, 1]))
            .unwrap();
        s.sell_joker(if pareidolia { 2 } else { 1 }).unwrap();
        let score = s.play_hand(&mut h, vec![0]).unwrap();
        near(score.mult, if pareidolia { 3.0 } else { 1.0 });
    }
}

#[test]
fn su05_four_fingers_shortcut_accept_separate_straight_and_flush_subsets() {
    for condition in 0..3 {
        let mut s = state(match condition {
            0 => &[JokerKind::FourFingers, JokerKind::Shortcut],
            1 => &[JokerKind::FourFingers],
            _ => &[JokerKind::Shortcut],
        });
        let mut h = hand([
            card(Rank::Queen, Suit::Spades),
            card(Rank::Jack, Suit::Spades),
            card(Rank::Nine, Suit::Hearts),
            card(Rank::Seven, Suit::Spades),
            card(Rank::Three, Suit::Spades),
        ]);
        assert_eq!(
            s.play_hand(&mut h, vec![0, 1, 2, 3, 4]).unwrap().hand,
            match condition {
                0 => PokerHand::StraightFlush,
                1 => PokerHand::Flush,
                _ => PokerHand::HighCard,
            }
        );
    }
}

#[test]
fn su07_three_of_a_kind_contains_pair_but_four_of_a_kind_is_not_two_pair() {
    let mut s = state(&[JokerKind::Jolly, JokerKind::Sly]);
    let mut h = hand([
        card(Rank::Seven, Suit::Clubs),
        card(Rank::Seven, Suit::Hearts),
        card(Rank::Seven, Suit::Spades),
    ]);
    let score = s.play_hand(&mut h, vec![0, 1, 2]).unwrap();
    assert_eq!((score.chips, score.mult), (101, 11.0));
    let mut s = state(&[JokerKind::Mad]);
    let mut h = hand([
        card(Rank::Seven, Suit::Clubs),
        card(Rank::Seven, Suit::Hearts),
        card(Rank::Seven, Suit::Spades),
        card(Rank::Seven, Suit::Diamonds),
    ]);
    assert_eq!(s.play_hand(&mut h, vec![0, 1, 2, 3]).unwrap().mult, 7.0);
}

#[test]
fn ge06_three_guaranteed_dual_lucky_successes_grow_cat_once_per_activation() {
    let mut s = state(&[JokerKind::LuckyCat, JokerKind::HangingChad]);
    s.redeem_voucher(Vouchers::Blank).unwrap();
    s.redeem_voucher(Vouchers::Antimatter).unwrap();
    for _ in 0..4 {
        s.jokers.add(Joker::create_joker(JokerKind::OopsAll6s));
    }
    let mut king = card(Rank::King, Suit::Clubs);
    king.set_enhancement(Enhancement::Lucky);
    let score = s.play_hand(&mut hand([king]), vec![0]).unwrap();
    near(score.mult, 106.75);
    assert_eq!(s.money, 60);
    assert_eq!(score.chips, 35);
}

#[test]
fn tr03_copied_dna_with_chad_grows_hologram_for_two_new_cards_only() {
    let mut s = state(&[
        JokerKind::Blueprint,
        JokerKind::DNA,
        JokerKind::Hologram,
        JokerKind::HangingChad,
    ]);
    let mut h = hand([card(Rank::Two, Suit::Clubs)]);
    let score = s.play_hand(&mut h, vec![0]).unwrap();
    assert_eq!((score.chips, score.mult, score.score), (11, 1.5, 16.0));
    assert_eq!(
        s.deck.cards().len() + s.deck.discarded_cards().len() + h.cards().len(),
        55,
        "52 owned + synthetic original + two copies"
    );
}

#[test]
fn tr10_splash_keeps_chads_first_card_distinct_from_photographs_first_face() {
    for chad in [false, true] {
        let mut kinds = vec![JokerKind::Splash, JokerKind::Photograph];
        if chad {
            kinds.push(JokerKind::HangingChad);
        }
        let mut s = state(&kinds);
        let score = s
            .play_hand(
                &mut hand([card(Rank::Two, Suit::Clubs), card(Rank::King, Suit::Hearts)]),
                vec![0, 1],
            )
            .unwrap();
        assert_eq!(
            (score.chips, score.mult, score.score),
            if chad {
                (21, 2.0, 42.0)
            } else {
                (17, 2.0, 34.0)
            }
        );
    }
}

#[test]
fn he01_red_seal_without_mime_repeats_held_steel_and_baron_twice() {
    let mut s = state(&[JokerKind::Baron]);
    let mut king = card(Rank::King, Suit::Clubs);
    king.set_enhancement(Enhancement::Steel);
    king.set_seal(Seal::Red);
    near(
        s.play_hand(&mut hand([card(Rank::Two, Suit::Hearts), king]), vec![0])
            .unwrap()
            .mult,
        5.0625,
    );
}

#[test]
fn he04_mime_repeats_held_gold_blue_rewards_for_pair_and_respects_one_free_slot() {
    for condition in 0..3 {
        let mut s = state(&[JokerKind::Mime]);
        if condition == 1 {
            s.add_consumable(Consumable::Planet(Planet::Pluto)).unwrap();
        }
        if condition < 2 {
            s.blind.score_requirement = 1;
        }
        let mut held = card(Rank::King, Suit::Clubs);
        held.set_enhancement(Enhancement::Gold);
        held.set_seal(Seal::Blue);
        let score = s
            .play_hand(
                &mut hand([
                    card(Rank::Seven, Suit::Clubs),
                    card(Rank::Seven, Suit::Hearts),
                    held,
                ]),
                vec![0, 1],
            )
            .unwrap();
        assert_eq!(
            (score.hand, score.chips, score.mult),
            (PokerHand::Pair, 24, 2.0)
        );
        assert_eq!(
            s.consumables
                .iter()
                .filter(|c| c.consumable == Consumable::Planet(Planet::Mercury))
                .count(),
            match condition {
                0 => 2,
                1 => 1,
                _ => 0,
            }
        );
        assert_eq!(s.money, if condition < 2 { 13 } else { 0 });
    }
}

#[test]
fn co02_dangling_blueprint_and_blueprint_brainstorm_cycle_do_not_activate() {
    for kinds in [
        &[JokerKind::Blueprint][..],
        &[JokerKind::Blueprint, JokerKind::Brainstorm][..],
    ] {
        let mut s = state(kinds);
        let score = s
            .play_hand(&mut hand([card(Rank::King, Suit::Clubs)]), vec![0])
            .unwrap();
        assert_eq!((score.chips, score.mult, score.score), (15, 1.0, 15.0));
        assert_eq!(s.jokers.as_slice().len(), kinds.len());
    }
}

#[test]
fn co07_photograph_copy_and_target_debuffs_suppress_only_owned_abilities_and_editions() {
    for condition in 0..3 {
        let mut s = state(&[JokerKind::Blueprint, JokerKind::Photograph]);
        if condition > 0 {
            let index = if condition == 1 { 1 } else { 0 };
            s.jokers
                .get_mut(index)
                .unwrap()
                .set_edition(JokerEdition::Polychrome);
            s.jokers.get_mut(index).unwrap().set_debuffed(true);
        }
        let score = s
            .play_hand(&mut hand([card(Rank::King, Suit::Clubs)]), vec![0])
            .unwrap();
        assert_eq!(
            score.mult,
            match condition {
                0 => 4.0,
                1 => 1.0,
                _ => 2.0,
            }
        );
    }
}

#[test]
fn tr06_chad_and_photograph_use_scoring_king_selected_after_an_excluded_two() {
    let mut s = state(&[JokerKind::Photograph, JokerKind::HangingChad]);
    let score = s
        .play_hand(
            &mut hand([card(Rank::Two, Suit::Clubs), card(Rank::King, Suit::Hearts)]),
            vec![0, 1],
        )
        .unwrap();
    assert_eq!((score.chips, score.mult, score.score), (35, 8.0, 280.0));
}

#[test]
fn su01_pareidolia_sock_and_smiley_repeat_face_mult_without_rewriting_rank() {
    for pareidolia in [false, true] {
        let mut kinds = vec![JokerKind::SockAndBuskin, JokerKind::SmileyFace];
        if pareidolia {
            kinds.push(JokerKind::Pareidolia);
        }
        let mut s = state(&kinds);
        let score = s
            .play_hand(&mut hand([card(Rank::Two, Suit::Clubs)]), vec![0])
            .unwrap();
        assert_eq!(
            (score.chips, score.mult),
            if pareidolia { (9, 11.0) } else { (7, 1.0) }
        );
        assert_eq!(s.deck.discarded_cards()[0].rank(), Rank::Two);
    }
}

#[test]
fn su03_smeared_lets_lusty_and_greedy_both_score_one_diamond() {
    for smeared in [false, true] {
        let mut kinds = vec![JokerKind::Lusty, JokerKind::Greedy];
        if smeared {
            kinds.push(JokerKind::Smeared);
        }
        let mut s = state(&kinds);
        let score = s
            .play_hand(&mut hand([card(Rank::Two, Suit::Diamonds)]), vec![0])
            .unwrap();
        assert_eq!(score.mult, if smeared { 7.0 } else { 4.0 });
        assert_eq!(s.deck.discarded_cards()[0].suit(), Suit::Diamonds);
    }
}

#[test]
fn su04_smeared_bloodstone_oops_and_hearts_boss_treat_diamonds_consistently() {
    for condition in 0..3 {
        let mut kinds = vec![JokerKind::Bloodstone, JokerKind::OopsAll6s];
        if condition != 1 {
            kinds.push(JokerKind::Smeared);
        }
        let mut s = state(&kinds);
        if condition == 2 {
            s.begin_blind_as(BlindKind::Boss(BossBlinds::Head));
            s.blind.score_requirement = u64::MAX;
        }
        let score = s
            .play_hand(&mut hand([card(Rank::Two, Suit::Diamonds)]), vec![0])
            .unwrap();
        assert_eq!(score.mult, if condition == 0 { 1.5 } else { 1.0 });
        assert_eq!(score.chips, if condition == 2 { 5 } else { 7 });
    }
}

#[test]
fn su06_four_printed_suits_require_splash_when_only_high_card_scores() {
    for splash in [false, true] {
        let mut kinds = vec![JokerKind::FlowerPot];
        if splash {
            kinds.push(JokerKind::Splash);
        }
        let mut s = state(&kinds);
        let score = s
            .play_hand(
                &mut hand([
                    card(Rank::Two, Suit::Clubs),
                    card(Rank::Four, Suit::Diamonds),
                    card(Rank::Seven, Suit::Hearts),
                    card(Rank::King, Suit::Spades),
                ]),
                vec![0, 1, 2, 3],
            )
            .unwrap();
        assert_eq!(
            (score.chips, score.mult, score.score),
            if splash {
                (28, 3.0, 84.0)
            } else {
                (15, 1.0, 15.0)
            }
        );
    }
}

#[test]
fn copied_wee_reads_growth_without_growing_the_target_twice() {
    let mut s = state(&[JokerKind::Blueprint, JokerKind::Wee, JokerKind::HangingChad]);
    let first = s
        .play_hand(&mut hand([card(Rank::Two, Suit::Clubs)]), vec![0])
        .unwrap();
    assert_eq!(
        first.chips, 59,
        "three Twos grow Wee by 24; two scoring abilities read 24 each"
    );
    let later = s
        .play_hand(&mut hand([card(Rank::Three, Suit::Clubs)]), vec![0])
        .unwrap();
    assert_eq!(later.chips, 62, "growth persists without another Two");
}

#[test]
fn copied_vampire_does_not_strip_midas_gold_at_the_copy_position() {
    let mut s = state(&[
        JokerKind::Vampire,
        JokerKind::MidasMask,
        JokerKind::Brainstorm,
    ]);
    let score = s
        .play_hand(&mut hand([card(Rank::King, Suit::Clubs)]), vec![0])
        .unwrap();
    near(score.mult, 1.0);
    assert_eq!(s.deck.discarded_cards()[0].enhancement(), Enhancement::Gold);
}

#[test]
fn hiker_grows_after_the_card_scores_and_copy_adds_its_own_bonus() {
    for copied in [false, true] {
        let mut s = state(if copied {
            &[JokerKind::Blueprint, JokerKind::Hiker]
        } else {
            &[JokerKind::Hiker]
        });
        let score = s
            .play_hand(&mut hand([card(Rank::Two, Suit::Clubs)]), vec![0])
            .unwrap();
        assert_eq!(
            score.chips, 7,
            "new permanent bonuses apply to later activations"
        );
        assert_eq!(
            s.deck.discarded_cards()[0].scoring_values().chips,
            if copied { 12 } else { 7 }
        );
    }
}

#[test]
fn debuffed_scoring_card_does_not_grow_hiker_or_wee() {
    let mut s = state(&[JokerKind::Hiker, JokerKind::Wee, JokerKind::HangingChad]);
    s.begin_blind_as(BlindKind::Boss(BossBlinds::Club));
    s.blind.score_requirement = u64::MAX;
    assert_eq!(
        s.play_hand(&mut hand([card(Rank::Two, Suit::Clubs)]), vec![0])
            .unwrap()
            .chips,
        5
    );
    let mut original = s.deck.discarded_cards()[0].clone();
    original.set_debuffed(false);
    assert_eq!(original.scoring_values().chips, 2);
    s.begin_blind_as(BlindKind::Big);
    s.blind.score_requirement = u64::MAX;
    assert_eq!(
        s.play_hand(&mut hand([card(Rank::Three, Suit::Hearts)]), vec![0])
            .unwrap()
            .chips,
        29
    );
}

#[test]
fn debuffed_space_and_its_copy_cannot_level_the_hand() {
    let mut s = state(&[
        JokerKind::Blueprint,
        JokerKind::Space,
        JokerKind::OopsAll6s,
        JokerKind::OopsAll6s,
    ]);
    s.jokers.get_mut(1).unwrap().set_debuffed(true);
    let score = s
        .play_hand(&mut hand([card(Rank::Two, Suit::Clubs)]), vec![0])
        .unwrap();
    assert_eq!((score.chips, score.mult), (7, 1.0));
    assert_eq!(
        s.hand_score(PokerHand::HighCard),
        HandScore { chips: 5, mult: 1 }
    );
}

#[test]
fn four_fingers_keeps_five_card_straights_and_matching_duplicate_ranks_scoring() {
    for ranks in [
        [Rank::Two, Rank::Three, Rank::Four, Rank::Five, Rank::Six],
        [Rank::Two, Rank::Three, Rank::Four, Rank::Five, Rank::Five],
    ] {
        let mut s = state(&[JokerKind::FourFingers]);
        let mut h = hand(ranks.into_iter().enumerate().map(|(index, rank)| {
            card(
                rank,
                if index % 2 == 0 {
                    Suit::Clubs
                } else {
                    Suit::Hearts
                },
            )
        }));
        let score = s.play_hand(&mut h, vec![0, 1, 2, 3, 4]).unwrap();
        assert_eq!(score.hand, PokerHand::Straight);
        assert_eq!(score.scoring_data.scoring_played_cards.len(), 5);
        assert_eq!(score.chips, if ranks[4] == Rank::Six { 50 } else { 49 });
    }
}

#[test]
fn four_fingers_does_not_demote_four_of_a_kind_to_a_flush() {
    let mut s = state(&[JokerKind::FourFingers]);
    let mut h = hand([
        card(Rank::Seven, Suit::Spades),
        card(Rank::Seven, Suit::Spades),
        card(Rank::Seven, Suit::Spades),
        card(Rank::Seven, Suit::Spades),
        card(Rank::King, Suit::Hearts),
    ]);
    let score = s.play_hand(&mut h, vec![0, 1, 2, 3, 4]).unwrap();
    assert_eq!((score.hand, score.chips), (PokerHand::FourOfAKind, 88));
    assert_eq!(score.scoring_data.non_scoring_played_cards[0].index, 4);
}

#[test]
fn straight_flush_contains_both_straight_and_flush_for_scoring_jokers() {
    let mut s = state(&[
        JokerKind::Crazy,
        JokerKind::Droll,
        JokerKind::Devious,
        JokerKind::Crafty,
    ]);
    let mut h = hand(
        [Rank::Two, Rank::Three, Rank::Four, Rank::Five, Rank::Six]
            .into_iter()
            .map(|rank| card(rank, Suit::Spades)),
    );
    let score = s.play_hand(&mut h, vec![0, 1, 2, 3, 4]).unwrap();
    assert_eq!(
        (score.hand, score.chips, score.mult),
        (PokerHand::StraightFlush, 300, 30.0)
    );
}
