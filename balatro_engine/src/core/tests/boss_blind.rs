use super::*;
use crate::core::enums::{Rank, Suit};
use crate::core::joker_types::{JokerEdition, JokerKind};

fn card(rank: Rank, suit: Suit) -> Card {
    Card::new(rank, suit, Enhancement::None, Edition::None, Seal::None)
}

fn boss_state(boss: BossBlinds) -> GameState {
    let mut state = GameState::new(401);
    if BossBlinds::SHOWDOWN.contains(&boss) {
        state.counters.ante = 8;
    }
    state.jokers.add(Joker::create_joker(JokerKind::Matador));
    state.begin_blind_as(BlindKind::Boss(boss));
    state
}

fn hand_of(cards: impl IntoIterator<Item = Card>) -> Hand {
    let mut hand = Hand::new();
    for card in cards {
        hand.add_card(card);
    }
    hand
}

#[test]
fn every_boss_has_normal_gameplay_behavior() {
    for boss in BossBlinds::ALL {
        let mut state = boss_state(boss);
        state.money = 20;
        let before = state.money;
        match boss {
            BossBlinds::Hook => {
                let mut hand = Hand::new();
                state.deal_cards(&mut hand);
                state.play_hand(&mut hand, vec![0]).unwrap();
                assert_eq!(state.deck.discarded_cards().len(), 3);
                assert_eq!(state.blind.discards_used, 0);
                assert_eq!(hand.hand_size(), 8);
                assert_eq!(state.money, before);
            }
            BossBlinds::Ox => {
                let mut hand = hand_of([card(Rank::Two, Suit::Clubs)]);
                state.play_hand(&mut hand, vec![0]).unwrap();
                assert_eq!(state.money, 8);
            }
            BossBlinds::House => {
                let mut hand = Hand::new();
                state.deal_cards(&mut hand);
                assert!(hand.cards().iter().all(Card::is_face_down));
                state.discard_selected_cards(&mut hand, vec![0]).unwrap();
                assert_eq!(
                    hand.cards()
                        .iter()
                        .filter(|card| card.is_face_down())
                        .count(),
                    7
                );
                assert_eq!(state.money, before);
            }
            BossBlinds::Wall | BossBlinds::VioletVessel => {
                let normal = BlindKind::Boss(BossBlinds::Hook).score_requirement(
                    state.blind.stake,
                    state.counters.ante,
                    false,
                );
                assert_eq!(
                    state.blind.score_requirement,
                    normal * if boss == BossBlinds::Wall { 2 } else { 3 }
                );
                let mut hand = hand_of([card(Rank::Two, Suit::Clubs)]);
                assert_eq!(state.play_hand(&mut hand, vec![0]).unwrap().score, 7.0);
                assert_eq!(state.money, before);
            }
            BossBlinds::Wheel => {
                for _ in 0..3 {
                    state.jokers.add(Joker::create_joker(JokerKind::OopsAll6s));
                }
                let mut hand = Hand::new();
                state.deal_cards(&mut hand);
                assert!(hand.cards().iter().all(Card::is_face_down));
                assert_eq!(state.money, before);
            }
            BossBlinds::Arm => {
                state.upgrade_hand_score(PokerHand::HighCard, 10, 1);
                let mut hand = hand_of([card(Rank::Two, Suit::Clubs)]);
                let result = state.play_hand(&mut hand, vec![0]).unwrap();
                assert_eq!(result.score, 7.0);
                assert_eq!(state.hand_score(PokerHand::HighCard), BASE_HAND_SCORES[0]);
                assert_eq!(state.money, before + 8);
            }
            BossBlinds::Club | BossBlinds::Goad | BossBlinds::Window | BossBlinds::Head => {
                let suit = match boss {
                    BossBlinds::Club => Suit::Clubs,
                    BossBlinds::Goad => Suit::Spades,
                    BossBlinds::Window => Suit::Diamonds,
                    _ => Suit::Hearts,
                };
                let mut hand = hand_of([card(Rank::Two, suit)]);
                assert_eq!(state.play_hand(&mut hand, vec![0]).unwrap().chips, 5);
                assert_eq!(state.money, before + 8);
            }
            BossBlinds::Fish => {
                let mut hand = Hand::new();
                state.deal_cards(&mut hand);
                assert!(hand.cards().iter().all(|card| !card.is_face_down()));
                state.play_hand(&mut hand, vec![0]).unwrap();
                assert_eq!(
                    hand.cards()
                        .iter()
                        .filter(|card| card.is_face_down())
                        .count(),
                    1
                );
                assert_eq!(state.money, before);
            }
            BossBlinds::Psychich => {
                let mut hand = hand_of([card(Rank::Two, Suit::Clubs)]);
                assert_eq!(state.play_hand(&mut hand, vec![0]).unwrap().score, 0.0);
                assert_eq!(state.blind.hands_remaining, 3);
                assert_eq!(state.money, before + 8);
            }
            BossBlinds::Water => {
                assert_eq!(state.blind.discards_remaining, 0);
                let mut hand = hand_of([card(Rank::Two, Suit::Clubs)]);
                assert!(state.discard_selected_cards(&mut hand, vec![0]).is_none());
                assert_eq!(hand.hand_size(), 1);
                assert_eq!(state.money, before);
            }
            BossBlinds::Manacle => {
                let mut hand = Hand::new();
                state.deal_cards(&mut hand);
                assert_eq!(hand.hand_size(), 7);
                assert_eq!(state.money, before);
            }
            BossBlinds::Eye => {
                let mut hand = hand_of([card(Rank::Two, Suit::Clubs)]);
                assert!(state.play_hand(&mut hand, vec![0]).unwrap().score > 0.0);
                assert_eq!(state.play_hand(&mut hand, vec![0]).unwrap().score, 0.0);
                assert_eq!(state.blind.hands_remaining, 2);
                assert_eq!(state.money, before + 8);
            }
            BossBlinds::Mouth => {
                let mut hand =
                    hand_of([card(Rank::Two, Suit::Clubs), card(Rank::Two, Suit::Hearts)]);
                assert!(state.play_hand(&mut hand, vec![0, 1]).unwrap().score > 0.0);
                assert_eq!(state.play_hand(&mut hand, vec![0]).unwrap().score, 0.0);
                assert_eq!(state.money, before + 8);
            }
            BossBlinds::Plante => {
                let mut hand = hand_of([card(Rank::King, Suit::Clubs)]);
                assert_eq!(state.play_hand(&mut hand, vec![0]).unwrap().chips, 5);
                assert_eq!(state.money, before + 8);
            }
            BossBlinds::Serpent => {
                let mut hand = Hand::new();
                state.deal_cards(&mut hand);
                assert_eq!(hand.hand_size(), 8);
                state.play_hand(&mut hand, vec![0]).unwrap();
                assert_eq!(hand.hand_size(), 10);
                state.discard_selected_cards(&mut hand, vec![0]).unwrap();
                assert_eq!(hand.hand_size(), 12);
                assert!(hand.cards().iter().all(|card| !card.is_face_down()));
                assert_eq!(state.money, before);
            }
            BossBlinds::Pillar => {
                let mut seen = card(Rank::Two, Suit::Clubs);
                seen.mark_played(state.counters.ante);
                let mut hand = hand_of([seen]);
                assert_eq!(state.play_hand(&mut hand, vec![0]).unwrap().chips, 5);
                assert_eq!(state.money, before + 8);
            }
            BossBlinds::Needle => {
                assert_eq!(state.blind.hands_remaining, 1);
                let mut hand = hand_of([card(Rank::Two, Suit::Clubs)]);
                state.play_hand(&mut hand, vec![0]).unwrap();
                assert_eq!(state.blind.hands_remaining, 0);
                assert!(state.play_hand(&mut hand, vec![0]).is_none());
                assert_eq!(state.money, before);
            }
            BossBlinds::Tooth => {
                let mut hand = hand_of([
                    card(Rank::Two, Suit::Clubs),
                    card(Rank::Three, Suit::Hearts),
                ]);
                state.play_hand(&mut hand, vec![0, 1]).unwrap();
                assert_eq!(state.money, before - 2);
            }
            BossBlinds::Flint => {
                let mut hand = hand_of([card(Rank::Two, Suit::Clubs)]);
                assert_eq!(state.play_hand(&mut hand, vec![0]).unwrap().score, 5.0);
                assert_eq!(state.money, before + 8);
            }
            BossBlinds::Mark => {
                let mut hand = hand_of([card(Rank::Two, Suit::Clubs)]);
                state.deal_cards(&mut hand);
                assert!(!hand.cards()[0].is_face_down());
                for card in &hand.cards()[1..] {
                    assert_eq!(card.is_face_down(), card.is_face_card());
                }
                assert_eq!(state.money, before);
            }
            BossBlinds::AmberAcorn => {
                assert!(
                    state
                        .jokers
                        .as_slice()
                        .iter()
                        .all(|joker| joker.is_face_down())
                );
                let mut hand = hand_of([card(Rank::Two, Suit::Clubs)]);
                state.play_hand(&mut hand, vec![0]).unwrap();
                assert_eq!(state.money, before);
            }
            BossBlinds::VerdantLeaf => {
                let mut hand = hand_of([card(Rank::Two, Suit::Clubs)]);
                assert_eq!(state.play_hand(&mut hand, vec![0]).unwrap().chips, 5);
                assert_eq!(state.money, before + 8);
            }
            BossBlinds::CrimsonHeart => {
                assert_eq!(
                    state
                        .jokers
                        .as_slice()
                        .iter()
                        .filter(|joker| joker.is_debuffed())
                        .count(),
                    1
                );
                let mut hand = hand_of([card(Rank::Two, Suit::Clubs)]);
                state.play_hand(&mut hand, vec![0]).unwrap();
                assert_eq!(
                    state
                        .jokers
                        .as_slice()
                        .iter()
                        .filter(|joker| joker.is_debuffed())
                        .count(),
                    1
                );
                assert_eq!(state.money, before);
            }
            BossBlinds::CeruleanBell => {
                let mut hand = Hand::new();
                state.deal_cards(&mut hand);
                let forced = state.blind.forced_card_index.unwrap();
                let other = (forced + 1) % hand.cards().len();
                assert!(state.play_hand(&mut hand, vec![other as u8]).is_none());
                assert_eq!(state.blind.hands_remaining, 4);
                assert!(state.play_hand(&mut hand, vec![forced as u8]).is_some());
                assert_eq!(state.money, before);
            }
        }
    }
}

#[test]
fn chicot_disables_every_boss_and_restores_resources_before_gameplay() {
    for boss in BossBlinds::ALL {
        let mut state = GameState::new(402);
        state.jokers.add(Joker::create_joker(JokerKind::Chicot));
        state.jokers.add(Joker::create_joker(JokerKind::Matador));
        state.begin_blind_as(BlindKind::Boss(boss));
        assert!(state.blind.boss_ability_disabled, "{boss:?}");
        assert_eq!(state.hand_size(), 8, "{boss:?}");
        assert_eq!(state.blind.hands_remaining, 4);
        assert_eq!(state.blind.discards_remaining, 4);
        assert!(
            state
                .jokers
                .as_slice()
                .iter()
                .all(|joker| !joker.is_face_down() && !joker.is_debuffed())
        );
        let before = state.money;
        let mut hand = hand_of([card(Rank::Two, Suit::Clubs)]);
        assert_eq!(
            state.play_hand(&mut hand, vec![0]).unwrap().score,
            7.0,
            "{boss:?}"
        );
        assert_eq!(state.money, before, "{boss:?}");
        assert!(
            hand.cards()
                .iter()
                .all(|card| !card.is_face_down() && !card.is_debuffed())
        );
    }
}

#[test]
fn matador_pays_once_per_hand_in_joker_order_and_copies_can_pay() {
    for (kinds, expected_mult, payout) in [
        (vec![JokerKind::Matador, JokerKind::Bootstraps], 6.0, 8),
        (vec![JokerKind::Bootstraps, JokerKind::Matador], 2.0, 8),
        (vec![JokerKind::Blueprint, JokerKind::Matador], 2.0, 16),
    ] {
        let mut state = GameState::new(403);
        for kind in kinds {
            state.jokers.add(Joker::create_joker(kind));
        }
        state.begin_blind_as(BlindKind::Boss(BossBlinds::Club));
        state.money = 4;
        let mut hand = hand_of([card(Rank::Two, Suit::Clubs), card(Rank::Two, Suit::Clubs)]);
        let score = state.play_hand(&mut hand, vec![0, 1]).unwrap();
        assert_eq!(score.mult, expected_mult);
        assert_eq!(score.chips, 10);
        assert_eq!(state.money, 4 + payout);
    }
}

#[test]
fn matador_ignores_debuffed_held_cards_and_excluded_kickers_unless_splash_scores_them() {
    let mut state = boss_state(BossBlinds::Plante);
    let mut hand = hand_of([card(Rank::Two, Suit::Clubs), card(Rank::King, Suit::Hearts)]);
    let before = state.money;
    state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!(state.money, before);
    for splash in [false, true] {
        let mut state = boss_state(BossBlinds::Club);
        if splash {
            state.jokers.add(Joker::create_joker(JokerKind::Splash));
        }
        let mut hand = hand_of([card(Rank::Two, Suit::Clubs), card(Rank::King, Suit::Hearts)]);
        let before = state.money;
        state.play_hand(&mut hand, vec![0, 1]).unwrap();
        assert_eq!(state.money - before, if splash { 8 } else { 0 });
    }
}

#[test]
fn arm_has_a_level_one_floor_and_only_pays_matador_when_it_lowers_a_level() {
    let mut state = boss_state(BossBlinds::Arm);
    for (upgrades, payout, remaining_extra_levels) in [(0, 0, 0), (2, 8, 1), (0, 8, 0), (0, 0, 0)] {
        for _ in 0..upgrades {
            state.upgrade_hand_score(PokerHand::HighCard, 10, 1);
        }
        let before = state.money;
        let mut hand = hand_of([card(Rank::Two, Suit::Hearts)]);
        state.play_hand(&mut hand, vec![0]).unwrap();
        assert_eq!(state.money - before, payout);
        assert_eq!(
            state.hand_score(PokerHand::HighCard).chips,
            5 + 10 * remaining_extra_levels
        );
    }
    assert_eq!(state.hand_score(PokerHand::HighCard), BASE_HAND_SCORES[0]);
}

#[test]
fn flint_rounds_both_base_values_up_and_leaves_card_and_joker_bonuses_unhalved() {
    let mut state = boss_state(BossBlinds::Flint);
    state.jokers.add(Joker::create_joker(JokerKind::Joker));
    let mut hand = hand_of([card(Rank::Two, Suit::Clubs)]);
    let score = state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!((score.chips, score.mult, score.score), (5, 5.0, 25.0));
    // Four of a Kind starts with odd Mult 7, halved to 4.
    let mut hand = hand_of((0..4).map(|_| card(Rank::Two, Suit::Clubs)));
    let score = state.play_hand(&mut hand, vec![0, 1, 2, 3]).unwrap();
    assert_eq!((score.chips, score.mult), (38, 8.0));
}

#[test]
fn flint_halving_does_not_overflow_at_the_hand_score_limit() {
    let mut state = boss_state(BossBlinds::Flint);
    state.hand_scores[0] = HandScore {
        chips: u16::MAX,
        mult: u16::MAX,
    };
    state.blind.score_requirement = u64::MAX;
    let mut hand = hand_of([card(Rank::Two, Suit::Clubs)]);
    let score = state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!((score.chips, score.mult), (32_770, 32_768.0));
}

#[test]
fn whole_hand_debuffs_consume_cards_and_do_not_activate_card_jokers_or_editions() {
    let mut state = boss_state(BossBlinds::Psychich);
    state.jokers.add(Joker::create_joker(JokerKind::DNA));
    let mut joker = Joker::create_joker(JokerKind::Joker);
    joker.set_edition(JokerEdition::Polychrome);
    state.jokers.add(joker);
    let mut steel = card(Rank::King, Suit::Hearts);
    steel.set_enhancement(Enhancement::Steel);
    let mut hand = hand_of([card(Rank::Two, Suit::Clubs), steel]);
    let total = state.deck.cards().len() + state.deck.discarded_cards().len() + hand.cards().len();
    let result = state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!((result.chips, result.mult, result.score), (0, 0.0, 0.0));
    assert!(result.scoring_data.scoring_played_cards.is_empty());
    assert_eq!(result.scoring_data.non_scoring_played_cards.len(), 1);
    assert_eq!(state.blind.hands_remaining, 3);
    assert_eq!(state.deck.discarded_cards().len(), 1);
    assert_eq!(
        state.deck.cards().len() + state.deck.discarded_cards().len() + hand.cards().len(),
        total
    );
    assert_eq!(state.money, 8);
}

#[test]
fn psychic_scores_five_cards_eye_accepts_new_types_and_mouth_accepts_its_initial_type() {
    let mut state = boss_state(BossBlinds::Psychich);
    let mut hand = hand_of(
        [Rank::Two, Rank::Three, Rank::Four, Rank::Five, Rank::Six]
            .into_iter()
            .enumerate()
            .map(|(index, rank)| {
                card(
                    rank,
                    [
                        Suit::Clubs,
                        Suit::Spades,
                        Suit::Hearts,
                        Suit::Diamonds,
                        Suit::Clubs,
                    ][index],
                )
            }),
    );
    let before = state.money;
    assert!(
        state
            .play_hand(&mut hand, vec![0, 1, 2, 3, 4])
            .unwrap()
            .score
            > 0.0
    );
    assert_eq!(state.money, before);
    for boss in [BossBlinds::Eye, BossBlinds::Mouth] {
        let mut state = boss_state(boss);
        let mut first = hand_of([card(Rank::Two, Suit::Hearts), card(Rank::Two, Suit::Clubs)]);
        state.play_hand(&mut first, vec![0, 1]).unwrap();
        let mut second = hand_of([
            card(Rank::Three, Suit::Hearts),
            card(Rank::Three, Suit::Clubs),
        ]);
        let score = state.play_hand(&mut second, vec![0, 1]).unwrap();
        assert_eq!(score.score == 0.0, boss == BossBlinds::Eye);
    }
}

#[test]
fn invalid_selections_do_not_consume_hands_or_trigger_boss_economy() {
    for boss in [
        BossBlinds::Psychich,
        BossBlinds::Eye,
        BossBlinds::Mouth,
        BossBlinds::Ox,
        BossBlinds::Tooth,
    ] {
        let mut state = boss_state(boss);
        let mut hand = hand_of([card(Rank::Two, Suit::Clubs)]);
        let money = state.money;
        for positions in [vec![], vec![0, 0], vec![1], vec![0, 1, 2, 3, 4, 5]] {
            assert!(state.play_hand(&mut hand, positions).is_none());
            assert_eq!(state.money, money);
            assert_eq!(state.blind.hands_remaining, 4);
            assert_eq!(hand.hand_size(), 1);
            assert!(state.deck.discarded_cards().is_empty());
            assert_eq!(state.counters.poker_hands_played, [0; 12]);
        }
    }
}

#[test]
fn fish_only_flips_new_play_refills_and_discard_refills_preserve_retained_cards() {
    let mut state = boss_state(BossBlinds::Fish);
    let mut hand = Hand::new();
    state.deal_cards(&mut hand);
    state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!(
        hand.cards()
            .iter()
            .filter(|card| card.is_face_down())
            .count(),
        1
    );
    state.discard_selected_cards(&mut hand, vec![0]).unwrap();
    assert_eq!(
        hand.cards()
            .iter()
            .filter(|card| card.is_face_down())
            .count(),
        1
    );
    assert!(!hand.cards().last().unwrap().is_face_down());
    assert_eq!(state.blind.discards_used, 1);
    assert_eq!(hand.hand_size(), 8);
}

#[test]
fn serpent_draws_exactly_three_after_actions_and_stops_at_an_empty_draw_pile() {
    let mut state = boss_state(BossBlinds::Serpent);
    let mut hand = Hand::new();
    state.deal_cards(&mut hand);
    state.deck.remove_cards(42); // Two cards remain after the opening eight.
    state.discard_selected_cards(&mut hand, vec![0]).unwrap();
    assert_eq!(hand.hand_size(), 9);
    assert_eq!(state.deck.cards().len(), 0);
    state.discard_selected_cards(&mut hand, vec![0]).unwrap();
    assert_eq!(hand.hand_size(), 8);
    assert_eq!(state.blind.discards_used, 2);
    assert!(hand.cards().iter().all(|card| !card.is_face_down()));
}

#[test]
fn failed_discard_is_atomic_and_does_not_refill_or_change_boss_draw_flags() {
    for boss in [BossBlinds::Fish, BossBlinds::Serpent, BossBlinds::House] {
        let mut state = boss_state(boss);
        let mut hand = Hand::new();
        state.deal_cards(&mut hand);
        let ids = hand.cards().iter().map(Card::id).collect::<Vec<_>>();
        let remaining = state.deck.cards().len();
        for positions in [vec![], vec![0, 0], vec![99]] {
            assert!(state.discard_selected_cards(&mut hand, positions).is_none());
            assert_eq!(hand.cards().iter().map(Card::id).collect::<Vec<_>>(), ids);
            assert_eq!(state.deck.cards().len(), remaining);
            assert_eq!(state.blind.discards_used, 0);
            assert_eq!(state.blind.discards_remaining, 4);
        }
    }
}

#[test]
fn burglar_can_add_hands_to_needle() {
    let mut state = GameState::new(404);
    state.jokers.add(Joker::create_joker(JokerKind::Burglar));
    state.begin_blind_as(BlindKind::Boss(BossBlinds::Needle));
    assert_eq!(state.blind.hands_remaining, 4);
    let mut hand = hand_of([card(Rank::Two, Suit::Hearts)]);
    assert!(state.play_hand(&mut hand, vec![0]).is_some());
    assert!(state.play_hand(&mut hand, vec![0]).is_some());
    assert_eq!(state.blind.hands_remaining, 2);
}

#[test]
fn pillar_tracks_occurrences_through_property_changes_and_keeps_small_blind_history() {
    let mut state = GameState::new(405);
    state.begin_blind_as(BlindKind::Small);
    state.blind.score_requirement = 1;
    let mut opening = hand_of([card(Rank::Two, Suit::Clubs)]);
    state.play_hand(&mut opening, vec![0]).unwrap();
    assert!(state.deck.discarded_cards()[0].was_played_in_ante(1));
    state.deck.reset_deck();
    let index = state
        .deck
        .cards()
        .iter()
        .position(|card| card.was_played_in_ante(1))
        .unwrap();
    let mut original = state.deck.remove_card_at(index).unwrap();
    original.set_rank(Rank::Five);
    original.set_suit(Suit::Hearts);
    original.set_enhancement(Enhancement::Bonus);
    original.set_edition(Edition::Foil);
    original.set_seal(Seal::Blue);
    let duplicate = Card::new(
        Rank::Five,
        Suit::Hearts,
        Enhancement::Bonus,
        Edition::Foil,
        Seal::Blue,
    );
    assert_eq!(original.id(), duplicate.id());
    let mut hand = hand_of([original, duplicate]);
    state.jokers.add(Joker::create_joker(JokerKind::Matador));
    state.begin_blind_as(BlindKind::Boss(BossBlinds::Pillar));
    let before = state.money;
    let score = state.play_hand(&mut hand, vec![0, 1]).unwrap();
    assert_eq!(
        (score.chips, score.mult),
        (95, 2.0),
        "only the previously played occurrence is debuffed"
    );
    assert_eq!(state.money, before + 8);
    assert!(state.deck.discarded_cards()[0].is_debuffed());
    assert!(!state.deck.discarded_cards()[1].is_debuffed());
}

#[test]
fn pillar_debuffs_previously_played_held_cards_and_does_not_mark_ordinary_discards() {
    let mut state = GameState::new(406);
    state.begin_blind_as(BlindKind::Small);
    let mut hand = hand_of([card(Rank::Two, Suit::Clubs)]);
    state.discard_selected_cards(&mut hand, vec![0]).unwrap();
    assert!(!state.deck.discarded_cards()[0].was_played_in_ante(1));
    let mut steel = card(Rank::King, Suit::Hearts);
    steel.set_enhancement(Enhancement::Steel);
    steel.mark_played(1);
    let mut hand = hand_of([card(Rank::Two, Suit::Hearts), steel]);
    state.jokers.add(Joker::create_joker(JokerKind::Baron));
    state.begin_blind_as(BlindKind::Boss(BossBlinds::Pillar));
    assert_eq!(state.play_hand(&mut hand, vec![0]).unwrap().mult, 1.0);
    assert!(hand.cards()[0].is_debuffed());
}

#[test]
fn pillar_history_survives_big_blind_and_clears_at_boss_completion_or_new_ante() {
    let mut state = GameState::new(407);
    state.begin_blind_as(BlindKind::Small);
    state.blind.score_requirement = 1;
    let mut hand = hand_of([card(Rank::Two, Suit::Clubs), card(Rank::Two, Suit::Clubs)]);
    state.play_hand(&mut hand, vec![0]).unwrap();
    state.begin_blind_as(BlindKind::Big);
    assert!(state.deck.discarded_cards()[0].was_played_in_ante(1));
    state.blind.score_requirement = 1;
    state.play_hand(&mut hand, vec![0]).unwrap();
    assert!(
        state
            .deck
            .discarded_cards()
            .iter()
            .all(|card| card.was_played_in_ante(1))
    );
    state.begin_blind_as(BlindKind::Boss(BossBlinds::Flint));
    state.blind.score_requirement = 1;
    state.play_hand(&mut hand, vec![0]).unwrap();
    assert!(
        state
            .deck
            .cards()
            .iter()
            .chain(state.deck.discarded_cards())
            .all(|card| !card.was_played_in_ante(1))
    );
    let mut retained = card(Rank::Two, Suit::Hearts);
    retained.mark_played(1);
    let mut hand = hand_of([retained]);
    state.begin_next_ante();
    state.begin_blind_as(BlindKind::Boss(BossBlinds::Pillar));
    assert_eq!(state.play_hand(&mut hand, vec![0]).unwrap().chips, 7);
}

#[test]
fn wild_cards_are_debuffed_by_each_suit_boss_but_stone_cards_have_no_suit() {
    for boss in [
        BossBlinds::Club,
        BossBlinds::Goad,
        BossBlinds::Window,
        BossBlinds::Head,
    ] {
        let mut state = boss_state(boss);
        let mut wild = card(Rank::Two, Suit::Hearts);
        wild.set_enhancement(Enhancement::Wild);
        let mut hand = hand_of([wild]);
        let before = state.money;
        assert_eq!(
            state.play_hand(&mut hand, vec![0]).unwrap().chips,
            5,
            "{boss:?}"
        );
        assert_eq!(state.money, before + 8);
        let suit = match boss {
            BossBlinds::Club => Suit::Clubs,
            BossBlinds::Goad => Suit::Spades,
            BossBlinds::Window => Suit::Diamonds,
            _ => Suit::Hearts,
        };
        let mut stone = card(Rank::King, suit);
        stone.set_enhancement(Enhancement::Stone);
        let mut hand = hand_of([stone]);
        let before = state.money;
        state.play_hand(&mut hand, vec![0]).unwrap();
        assert!(!state.deck.discarded_cards().last().unwrap().is_debuffed());
        assert_eq!(state.money, before);
    }
    let mut state = boss_state(BossBlinds::Plante);
    let mut stone = card(Rank::King, Suit::Hearts);
    stone.set_enhancement(Enhancement::Stone);
    let mut hand = hand_of([stone]);
    state.play_hand(&mut hand, vec![0]).unwrap();
    assert!(!state.deck.discarded_cards().last().unwrap().is_debuffed());
}

#[test]
fn crimson_heart_changes_its_victim_after_play_but_not_after_discard() {
    let mut state = GameState::new(408);
    for kind in [JokerKind::Joker, JokerKind::SmileyFace] {
        state.jokers.add(Joker::create_joker(kind));
    }
    state.begin_blind_as(BlindKind::Boss(BossBlinds::CrimsonHeart));
    let mut hand = Hand::new();
    state.deal_cards(&mut hand);
    let first = state
        .jokers
        .as_slice()
        .iter()
        .position(|joker| joker.is_debuffed())
        .unwrap();
    state.play_hand(&mut hand, vec![0]).unwrap();
    let second = state
        .jokers
        .as_slice()
        .iter()
        .position(|joker| joker.is_debuffed())
        .unwrap();
    assert_ne!(first, second);
    state.discard_selected_cards(&mut hand, vec![0]).unwrap();
    assert_eq!(
        state
            .jokers
            .as_slice()
            .iter()
            .position(|joker| joker.is_debuffed()),
        Some(second)
    );
    state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!(
        state
            .jokers
            .as_slice()
            .iter()
            .position(|joker| joker.is_debuffed()),
        Some(first)
    );
}

#[test]
fn cerulean_bell_preserves_a_retained_forced_card_across_index_shifts() {
    let mut state = boss_state(BossBlinds::CeruleanBell);
    let mut hand = Hand::new();
    state.deal_cards(&mut hand);
    let forced = state.blind.forced_card_index.unwrap();
    let forced_id = hand.cards()[forced].id();
    let discard = if forced > 0 { forced - 1 } else { forced + 1 };
    state
        .discard_selected_cards(&mut hand, vec![discard as u8])
        .unwrap();
    let shifted = state.blind.forced_card_index.unwrap();
    assert_eq!(shifted, forced - usize::from(discard < forced));
    assert_eq!(hand.cards()[shifted].id(), forced_id);
    assert!(
        state
            .play_hand(&mut hand, vec![((shifted + 1) % 8) as u8])
            .is_none()
    );
    state.play_hand(&mut hand, vec![shifted as u8]).unwrap();
    assert!(state.blind.forced_card_index.is_some());
}

#[test]
fn luchador_does_not_disable_boss_until_sold_and_selling_during_play_clears_verdant_leaf() {
    let mut game = Game::new(409);
    game.state
        .jokers
        .add(Joker::create_joker(JokerKind::Luchador));
    game.apply_action(GameAction::SelectBlind(BlindKind::Small))
        .unwrap();
    game.state
        .begin_blind_as(BlindKind::Boss(BossBlinds::VerdantLeaf));
    game.state.deal_cards(&mut game.hand);
    assert!(!game.state.blind.boss_ability_disabled);
    assert!(game.hand.cards().iter().all(|card| card.is_debuffed()));
    assert_eq!(
        game.apply_action(GameAction::SellJoker(0)),
        Ok(DecisionPoint::PlayingHand)
    );
    assert!(game.state.blind.boss_ability_disabled);
    assert!(game.hand.cards().iter().all(|card| !card.is_debuffed()));

    let mut game = Game::new(410);
    game.state.jokers.add(Joker::create_joker(JokerKind::Joker));
    game.apply_action(GameAction::SelectBlind(BlindKind::Small))
        .unwrap();
    game.state
        .begin_blind_as(BlindKind::Boss(BossBlinds::VerdantLeaf));
    game.apply_action(GameAction::SellJoker(0)).unwrap();
    assert!(!game.state.blind.verdant_leaf_active);
    assert!(!game.state.blind.boss_ability_disabled);
}

#[test]
fn boss_disable_restores_resources_once_and_reveals_amber_jokers() {
    for boss in [
        BossBlinds::Water,
        BossBlinds::Needle,
        BossBlinds::Manacle,
        BossBlinds::AmberAcorn,
    ] {
        let mut state = GameState::new(411);
        state.jokers.add(Joker::create_joker(JokerKind::Luchador));
        state.jokers.add(Joker::create_joker(JokerKind::Joker));
        state.begin_blind_as(BlindKind::Boss(boss));
        let luchador = state
            .jokers
            .as_slice()
            .iter()
            .position(|joker| joker.kind() == JokerKind::Luchador)
            .unwrap();
        state.sell_joker(luchador).unwrap();
        assert_eq!(state.blind.hands_remaining, 4);
        assert_eq!(state.blind.discards_remaining, 4);
        assert_eq!(state.hand_size(), 8);
        assert!(
            state
                .jokers
                .as_slice()
                .iter()
                .all(|joker| !joker.is_face_down())
        );
        state.blind.disable_boss_ability();
        assert_eq!(state.blind.hands_remaining, 4);
        assert_eq!(state.blind.discards_remaining, 4);
        assert_eq!(state.hand_size(), 8);
    }
}

fn final_hand_game(seed: u64, before_score: u64, bones: usize, debuffed: bool) -> Game {
    let mut game = Game::new(seed);
    game.apply_action(GameAction::SelectBlind(BlindKind::Small))
        .unwrap();
    for _ in 0..bones {
        let mut bones = Joker::create_joker(JokerKind::MrBones);
        bones.set_debuffed(debuffed);
        game.state.jokers.add(bones);
    }
    game.hand.card_mut(0).unwrap().set_rank(Rank::Two);
    game.hand.card_mut(0).unwrap().set_suit(Suit::Clubs);
    game.state.blind.hands_remaining = 1;
    game.state.blind.chips_scored = before_score;
    game.state.blind.score_requirement = 100;
    game.state.money = 4;
    game
}

#[test]
fn mr_bones_saves_at_exactly_a_quarter_and_does_not_inflate_the_scored_chips() {
    for (before_score, debuffed, expected) in [
        (17, false, DecisionPoint::RunComplete),
        (18, true, DecisionPoint::RunComplete),
        (18, false, DecisionPoint::Shop),
    ] {
        let mut game = final_hand_game(412, before_score, 1, debuffed);
        assert_eq!(
            game.apply_action(GameAction::PlayCards(vec![0])),
            Ok(expected)
        );
        assert_eq!(game.state.blind.chips_scored, before_score + 7);
        assert_eq!(game.state.blind.finished, expected == DecisionPoint::Shop);
        assert_eq!(
            game.state.jokers.as_slice().len(),
            usize::from(expected != DecisionPoint::Shop)
        );
        assert_eq!(
            game.state.money,
            if expected == DecisionPoint::Shop {
                7
            } else {
                4
            }
        );
        assert_eq!(game.next_step(), expected);
    }
}

#[test]
fn death_prevention_consumes_only_one_bones_and_dispatches_completion_once() {
    let mut game = final_hand_game(413, 18, 2, false);
    game.state
        .jokers
        .add(Joker::create_joker(JokerKind::Golden));
    assert_eq!(
        game.apply_action(GameAction::PlayCards(vec![0])),
        Ok(DecisionPoint::Shop)
    );
    assert_eq!(
        game.state
            .jokers
            .as_slice()
            .iter()
            .filter(|joker| joker.kind() == JokerKind::MrBones)
            .count(),
        1
    );
    assert_eq!(game.state.money, 12); // $4 start + $4 Golden + $3 reward + $1 interest.
    assert!(game.state.finish_blind().is_none());
    assert_eq!(game.state.money, 12);
}

#[test]
fn winning_normally_does_not_consume_mr_bones() {
    let mut game = final_hand_game(414, 93, 1, false);
    assert_eq!(
        game.apply_action(GameAction::PlayCards(vec![0])),
        Ok(DecisionPoint::Shop)
    );
    assert_eq!(game.state.jokers.as_slice().len(), 1);
    assert!(!game.state.blind.prevent_death);
    assert_eq!(game.state.blind.chips_scored, 100);
}

#[test]
fn an_empty_hand_checks_death_prevention_even_with_hands_remaining() {
    for bones in [0, 1] {
        let mut game = final_hand_game(415, 18, bones, false);
        game.state.blind.hands_remaining = 4;
        game.state.deck.remove_cards(52);
        let removed = game.hand.select_cards((1..game.hand.hand_size()).collect());
        game.state.deck.discard_cards(removed);
        let expected = if bones == 1 {
            DecisionPoint::Shop
        } else {
            DecisionPoint::RunComplete
        };
        assert_eq!(
            game.apply_action(GameAction::PlayCards(vec![0])),
            Ok(expected)
        );
        assert_eq!(game.state.blind.hands_remaining, 3);
        assert_eq!(game.state.blind.chips_scored, 25);
        assert!(game.hand.cards().is_empty());
    }
}

#[test]
fn emptying_the_hand_by_discard_also_checks_death_prevention() {
    for bones in [0, 1] {
        let mut game = final_hand_game(416, 25, bones, false);
        game.state.blind.hands_remaining = 4;
        game.state.deck.remove_cards(52);
        let removed = game.hand.select_cards((1..game.hand.hand_size()).collect());
        game.state.deck.discard_cards(removed);
        let expected = if bones == 1 {
            DecisionPoint::Shop
        } else {
            DecisionPoint::RunComplete
        };
        assert_eq!(
            game.apply_action(GameAction::DiscardCards(vec![0])),
            Ok(expected)
        );
        assert_eq!(game.state.blind.chips_scored, 25);
        assert_eq!(game.state.blind.discards_used, 1);
    }
}

#[test]
fn runtime_keeps_the_same_physical_cards_across_blind_transitions() {
    let mut game = Game::new(417);
    game.apply_action(GameAction::SelectBlind(BlindKind::Small))
        .unwrap();
    let played_id = game.hand.cards()[0].id();
    game.state.blind.score_requirement = 1;
    game.apply_action(GameAction::PlayCards(vec![0])).unwrap();
    assert_eq!(
        game.state.deck.cards().len()
            + game.state.deck.discarded_cards().len()
            + game.hand.cards().len(),
        52
    );
    game.apply_action(GameAction::LeaveShop).unwrap();
    game.apply_action(GameAction::SelectBlind(BlindKind::Big))
        .unwrap();
    assert_eq!(
        game.state.deck.cards().len()
            + game.state.deck.discarded_cards().len()
            + game.hand.cards().len(),
        52
    );
    let previously_played = game
        .state
        .deck
        .cards()
        .iter()
        .chain(game.hand.cards())
        .find(|card| card.id() == played_id)
        .unwrap();
    assert!(previously_played.was_played_in_ante(1));
}

#[test]
fn ox_uses_the_previous_ante_target_and_is_not_retargeted_by_current_plays() {
    let mut state = GameState::new(418);
    assert_eq!(state.ox_target_hand, PokerHand::HighCard);
    state.begin_blind_as(BlindKind::Small);
    let mut opening = hand_of([card(Rank::Two, Suit::Clubs), card(Rank::Two, Suit::Hearts)]);
    state.play_hand(&mut opening, vec![0, 1]).unwrap();
    state.begin_blind_as(BlindKind::Boss(BossBlinds::Flint));
    state.blind.score_requirement = 1;
    let mut finishing = hand_of([card(Rank::Two, Suit::Clubs)]);
    state.play_hand(&mut finishing, vec![0]).unwrap();
    assert_eq!(
        state.ox_target_hand,
        PokerHand::Pair,
        "tied counts prefer stronger hands"
    );
    state.begin_next_ante();
    state.jokers.add(Joker::create_joker(JokerKind::Matador));
    state.begin_blind_as(BlindKind::Boss(BossBlinds::Ox));
    state.money = 20;
    for _ in 0..2 {
        let mut hand = hand_of([card(Rank::Two, Suit::Clubs)]);
        state.play_hand(&mut hand, vec![0]).unwrap();
        assert_eq!(state.money, 20);
    }
    assert_eq!(state.most_played_hand(), Some(PokerHand::HighCard));
    let mut pair = hand_of([card(Rank::Two, Suit::Clubs), card(Rank::Two, Suit::Hearts)]);
    state.play_hand(&mut pair, vec![0, 1]).unwrap();
    assert_eq!(state.money, 8);
}

#[test]
fn copied_matador_does_not_pay_when_the_copy_or_target_is_debuffed() {
    for debuff_copy in [false, true] {
        let mut state = GameState::new(419);
        state.jokers.add(Joker::create_joker(JokerKind::Blueprint));
        state.jokers.add(Joker::create_joker(JokerKind::Matador));
        state.begin_blind_as(BlindKind::Boss(BossBlinds::Flint));
        state
            .jokers
            .get_mut(if debuff_copy { 0 } else { 1 })
            .unwrap()
            .set_debuffed(true);
        let mut hand = hand_of([card(Rank::Two, Suit::Hearts)]);
        state.play_hand(&mut hand, vec![0]).unwrap();
        assert_eq!(state.money, if debuff_copy { 8 } else { 0 });
    }
}

#[test]
fn whole_hand_debuff_skips_before_scoring_growth_but_still_consumes_after_play_decay() {
    let mut state = boss_state(BossBlinds::Psychich);
    state.jokers.add(Joker::create_joker(JokerKind::Green));
    state.jokers.add(Joker::create_joker(JokerKind::IceCream));
    let mut first = hand_of([card(Rank::Two, Suit::Hearts)]);
    state.play_hand(&mut first, vec![0]).unwrap();
    state.blind.disable_boss_ability();
    let mut second = hand_of([card(Rank::Two, Suit::Hearts)]);
    let score = state.play_hand(&mut second, vec![0]).unwrap();
    assert_eq!((score.chips, score.mult), (102, 1.0));
}

#[test]
fn face_down_boss_draws_are_seeded_and_retained_cards_are_not_rerolled() {
    for boss in [
        BossBlinds::Wheel,
        BossBlinds::Mark,
        BossBlinds::House,
        BossBlinds::Fish,
    ] {
        let mut first = boss_state(boss);
        let mut second = boss_state(boss);
        let mut hand_a = Hand::new();
        let mut hand_b = Hand::new();
        first.deal_cards(&mut hand_a);
        second.deal_cards(&mut hand_b);
        first
            .discard_selected_cards(&mut hand_a, vec![0, 1])
            .unwrap();
        second
            .discard_selected_cards(&mut hand_b, vec![0, 1])
            .unwrap();
        let snapshot = |hand: &Hand| {
            hand.cards()
                .iter()
                .map(|card| (card.id(), card.is_face_down(), card.is_debuffed()))
                .collect::<Vec<_>>()
        };
        assert_eq!(snapshot(&hand_a), snapshot(&hand_b), "{boss:?}");
        let retained = snapshot(&hand_a);
        first.deal_cards(&mut hand_a); // No space: no face-down draw rolls.
        assert_eq!(snapshot(&hand_a), retained, "{boss:?}");
    }
}

#[test]
fn mr_bones_save_does_not_repeat_on_a_later_losing_blind() {
    let mut game = final_hand_game(420, 18, 1, false);
    assert_eq!(
        game.apply_action(GameAction::PlayCards(vec![0])),
        Ok(DecisionPoint::Shop)
    );
    game.apply_action(GameAction::LeaveShop).unwrap();
    game.apply_action(GameAction::SelectBlind(BlindKind::Big))
        .unwrap();
    assert!(!game.state.blind.prevent_death);
    game.state.blind.hands_remaining = 1;
    game.state.blind.score_requirement = u64::MAX;
    assert_eq!(
        game.apply_action(GameAction::PlayCards(vec![0])),
        Ok(DecisionPoint::RunComplete)
    );
}

#[test]
fn whole_hand_debuff_preserves_the_order_of_non_scoring_card_references() {
    let mut state = boss_state(BossBlinds::Psychich);
    let mut hand = hand_of([card(Rank::King, Suit::Hearts), card(Rank::Two, Suit::Clubs)]);
    let score = state.play_hand(&mut hand, vec![0, 1]).unwrap();
    assert_eq!(
        score
            .scoring_data
            .non_scoring_played_cards
            .iter()
            .map(|card| card.index)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert!(score.scoring_data.scoring_held_cards.is_empty());
}
