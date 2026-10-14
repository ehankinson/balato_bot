use super::*;
use crate::core::enums::{Rank, Suit};
use crate::core::joker_types::{JokerEdition, JokerKind};
use rand::RngExt;

fn card(rank: Rank, suit: Suit, enhancement: Enhancement, seal: Seal) -> Card {
    Card::new(rank, suit, enhancement, Edition::None, seal)
}

fn hand_of(cards: impl IntoIterator<Item = Card>) -> Hand {
    let mut hand = Hand::new();
    for card in cards {
        hand.add_card(card);
    }
    hand
}

fn state_with(kinds: &[JokerKind]) -> GameState {
    let mut state = GameState::new(601);
    state.begin_blind_as(BlindKind::Small);
    state.blind.score_requirement = u64::MAX;
    state.money = 0;
    for &kind in kinds {
        state.jokers.add(Joker::create_joker(kind));
    }
    state
}

#[test]
fn played_income_retriggers_reach_bull_and_bootstraps_in_both_orders() {
    for reversed in [false, true] {
        let mut kinds = vec![
            JokerKind::GoldenTicket,
            JokerKind::RoughGem,
            JokerKind::Bull,
            JokerKind::Bootstraps,
            JokerKind::HangingChad,
        ];
        if reversed {
            kinds.reverse();
        }
        let mut state = state_with(&kinds);
        let mut hand = hand_of([card(
            Rank::King,
            Suit::Diamonds,
            Enhancement::Gold,
            Seal::Red,
        )]);
        let result = state.play_hand(&mut hand, vec![0]).unwrap();
        // Four activations earn $5 each; after-hand scoring sees all $20.
        assert_eq!((result.chips, result.mult, result.score), (85, 9.0, 765.0));
        assert_eq!(state.money, 20);
        assert_eq!(state.deck.discarded_cards().len(), 1);
    }
}

#[test]
fn copied_business_card_pays_per_activation_and_non_faces_do_not_roll() {
    for face in [false, true] {
        let mut state = state_with(&[
            JokerKind::Blueprint,
            JokerKind::BusinessCard,
            JokerKind::OopsAll6s,
        ]);
        state.blind.hands_remaining = 1;
        state.rng = ChaCha12Rng::seed_from_u64(611);
        let mut expected_rng = ChaCha12Rng::seed_from_u64(611);
        let mut hand = hand_of([card(
            if face { Rank::Queen } else { Rank::Seven },
            Suit::Clubs,
            Enhancement::None,
            Seal::Red,
        )]);
        let score = state.play_hand(&mut hand, vec![0]).unwrap();
        assert_eq!(state.money, if face { 8 } else { 0 });
        assert_eq!(score.chips, if face { 25 } else { 19 });
        assert_eq!(state.rng.random::<u64>(), expected_rng.random::<u64>());
    }
}

#[test]
fn business_card_seeded_rolls_repeat_and_oops_changes_the_odds() {
    let mut saw_failure = false;
    let mut saw_success = false;
    for doubled in [false, true] {
        for seed in 0..30 {
            let mut state = state_with(if doubled {
                &[JokerKind::BusinessCard, JokerKind::OopsAll6s]
            } else {
                &[JokerKind::BusinessCard]
            });
            state.blind.hands_remaining = 1;
            state.rng = ChaCha12Rng::seed_from_u64(seed);
            let mut oracle = ChaCha12Rng::seed_from_u64(seed);
            let successes = if doubled {
                2
            } else {
                (0..2).filter(|_| oracle.random_bool(0.5)).count()
            };
            let mut hand = hand_of([card(Rank::Jack, Suit::Hearts, Enhancement::None, Seal::Red)]);
            let result = state.play_hand(&mut hand, vec![0]).unwrap();
            assert_eq!(
                (result.chips, result.mult, state.money),
                (25, 1.0, successes as i16 * 2)
            );
            assert_eq!(state.rng.random::<u64>(), oracle.random::<u64>());
            saw_failure |= successes == 0;
            saw_success |= successes == 2;
        }
    }
    assert!(saw_failure && saw_success);
}

#[test]
fn played_economy_ignores_kickers_and_debuffed_cards_or_jokers() {
    for condition in 0..4 {
        let mut state = state_with(&[
            JokerKind::GoldenTicket,
            JokerKind::BusinessCard,
            JokerKind::RoughGem,
            JokerKind::OopsAll6s,
        ]);
        if condition == 1 {
            state.jokers.add(Joker::create_joker(JokerKind::Splash));
        }
        if condition == 2 {
            for index in 0..3 {
                state.jokers.get_mut(index).unwrap().set_debuffed(true);
            }
            state.jokers.add(Joker::create_joker(JokerKind::Splash));
        }
        if condition == 3 {
            state.begin_blind_as(BlindKind::Boss(BossBlinds::Plante));
            state.blind.score_requirement = u64::MAX;
            state.jokers.add(Joker::create_joker(JokerKind::Splash));
        }
        let mut hand = hand_of([
            card(Rank::Two, Suit::Clubs, Enhancement::None, Seal::None),
            card(Rank::Two, Suit::Spades, Enhancement::None, Seal::None),
            card(Rank::King, Suit::Diamonds, Enhancement::Gold, Seal::Red),
        ]);
        let result = state.play_hand(&mut hand, vec![0, 1, 2]).unwrap();
        assert_eq!(state.money, if condition == 1 { 14 } else { 0 });
        assert_eq!(
            result.chips,
            if condition == 1 || condition == 2 {
                34
            } else {
                14
            }
        );
    }
}

#[test]
fn reserved_parking_income_repeats_for_red_mime_and_copies() {
    let mut state = state_with(&[
        JokerKind::Blueprint,
        JokerKind::ReservedParking,
        JokerKind::Mime,
        JokerKind::OopsAll6s,
        JokerKind::Bull,
    ]);
    let mut hand = hand_of([
        card(Rank::Two, Suit::Clubs, Enhancement::None, Seal::None),
        card(Rank::King, Suit::Hearts, Enhancement::Steel, Seal::Red),
        card(Rank::Seven, Suit::Clubs, Enhancement::None, Seal::Red),
    ]);
    let result = state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!(state.money, 6);
    assert_eq!((result.chips, result.mult, result.score), (19, 3.375, 64.0));
}

#[test]
fn reserved_parking_respects_card_and_joker_debuffs_and_pareidolia() {
    for condition in 0..4 {
        let mut state = state_with(&[JokerKind::ReservedParking, JokerKind::OopsAll6s]);
        if condition == 1 {
            state.jokers.add(Joker::create_joker(JokerKind::Pareidolia));
        }
        if condition == 2 {
            state.jokers.get_mut(0).unwrap().set_debuffed(true);
        }
        if condition == 3 {
            state.jokers.add(Joker::create_joker(JokerKind::Pareidolia));
            state.begin_blind_as(BlindKind::Boss(BossBlinds::Plante));
            state.blind.score_requirement = u64::MAX;
        }
        let mut hand = hand_of([
            card(Rank::Ace, Suit::Clubs, Enhancement::None, Seal::None),
            card(Rank::King, Suit::Clubs, Enhancement::None, Seal::None),
            card(Rank::Seven, Suit::Clubs, Enhancement::None, Seal::None),
        ]);
        state.play_hand(&mut hand, vec![0]).unwrap();
        assert_eq!(
            state.money,
            match condition {
                0 => 1,
                1 => 2,
                _ => 0,
            }
        );
    }
}

#[test]
fn reserved_parking_seeded_retrigger_rolls_are_independent() {
    for seed in 0..30 {
        let mut state = state_with(&[JokerKind::ReservedParking, JokerKind::Mime]);
        state.blind.hands_remaining = 1;
        state.rng = ChaCha12Rng::seed_from_u64(seed);
        let mut oracle = ChaCha12Rng::seed_from_u64(seed);
        let successes = (0..3).filter(|_| oracle.random_bool(0.5)).count();
        let mut hand = hand_of([
            card(Rank::Ace, Suit::Clubs, Enhancement::None, Seal::None),
            card(Rank::King, Suit::Clubs, Enhancement::None, Seal::Red),
        ]);
        state.play_hand(&mut hand, vec![0]).unwrap();
        assert_eq!(state.money, successes as i16);
        assert_eq!(state.rng.random::<u64>(), oracle.random::<u64>());
    }
}

#[test]
fn vagabond_sees_played_and_held_income_before_its_phase() {
    for held in [false, true] {
        let mut state = state_with(if held {
            &[
                JokerKind::Vagabond,
                JokerKind::ReservedParking,
                JokerKind::OopsAll6s,
            ]
        } else {
            &[JokerKind::Vagabond, JokerKind::GoldenTicket]
        });
        state.money = 4;
        let mut hand = hand_of([
            card(
                Rank::King,
                Suit::Clubs,
                if held {
                    Enhancement::None
                } else {
                    Enhancement::Gold
                },
                Seal::None,
            ),
            card(Rank::Queen, Suit::Clubs, Enhancement::None, Seal::None),
        ]);
        state.play_hand(&mut hand, vec![0]).unwrap();
        assert_eq!(state.money, if held { 5 } else { 8 });
        assert!(state.consumables.is_empty());
    }
}

#[test]
fn vagabond_money_boundary_copy_capacity_and_debuffs() {
    for (money, expected) in [(-1, 2), (4, 2), (5, 0)] {
        let mut state = state_with(&[JokerKind::Blueprint, JokerKind::Vagabond]);
        state.money = money;
        let mut hand = hand_of([card(Rank::Two, Suit::Clubs, Enhancement::None, Seal::Red)]);
        let result = state.play_hand(&mut hand, vec![0]).unwrap();
        assert_eq!(state.consumables.len(), expected);
        assert_eq!((result.chips, result.mult, state.money), (9, 1.0, money));
    }
    for disabled in [false, true] {
        let mut state = state_with(&[JokerKind::Blueprint, JokerKind::Vagabond]);
        state
            .add_consumable(Consumable::Planet(Planet::Pluto))
            .unwrap();
        if disabled {
            state.jokers.get_mut(1).unwrap().set_debuffed(true);
        }
        let mut hand = hand_of([card(Rank::Two, Suit::Clubs, Enhancement::None, Seal::None)]);
        state.play_hand(&mut hand, vec![0]).unwrap();
        assert_eq!(state.consumables.len(), if disabled { 1 } else { 2 });
    }
}

#[test]
fn vagabond_and_matador_preserve_after_hand_joker_order() {
    for reversed in [false, true] {
        let mut state = state_with(if reversed {
            &[JokerKind::Matador, JokerKind::Vagabond]
        } else {
            &[JokerKind::Vagabond, JokerKind::Matador]
        });
        state.begin_blind_as(BlindKind::Boss(BossBlinds::Flint));
        state.blind.score_requirement = u64::MAX;
        let mut hand = hand_of([card(Rank::King, Suit::Clubs, Enhancement::None, Seal::None)]);
        let result = state.play_hand(&mut hand, vec![0]).unwrap();
        assert_eq!((result.chips, result.mult, state.money), (13, 1.0, 8));
        assert_eq!(state.consumables.len(), usize::from(!reversed));
    }
}

#[test]
fn economy_joker_editions_score_at_their_owned_positions() {
    for reversed in [false, true] {
        let mut state = state_with(if reversed {
            &[JokerKind::Cavendish, JokerKind::GoldenTicket]
        } else {
            &[JokerKind::GoldenTicket, JokerKind::Cavendish]
        });
        state
            .jokers
            .get_mut(usize::from(reversed))
            .unwrap()
            .set_edition(JokerEdition::Holographic);
        let mut hand = hand_of([card(Rank::King, Suit::Clubs, Enhancement::Gold, Seal::Red)]);
        let result = state.play_hand(&mut hand, vec![0]).unwrap();
        assert_eq!(result.mult, if reversed { 13.0 } else { 33.0 });
        assert_eq!(state.money, 8);
    }
}

#[test]
fn eight_ball_generates_per_scoring_eight_and_retriggers_fill_capacity() {
    let mut state = state_with(&[
        JokerKind::EightBall,
        JokerKind::HangingChad,
        JokerKind::OopsAll6s,
        JokerKind::OopsAll6s,
    ]);
    state.consumable_capacity = 4;
    let mut hand = hand_of([card(Rank::Eight, Suit::Clubs, Enhancement::None, Seal::Red)]);
    let result = state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!((result.chips, result.mult, result.score), (37, 1.0, 37.0));
    assert_eq!(state.consumables.len(), 4);
    assert!(
        state
            .consumables
            .iter()
            .all(|owned| matches!(owned.consumable, Consumable::Tarot(_)))
    );
}

#[test]
fn eight_ball_copies_activate_and_later_generators_see_capacity() {
    let mut state = state_with(&[
        JokerKind::Blueprint,
        JokerKind::EightBall,
        JokerKind::Vagabond,
        JokerKind::OopsAll6s,
        JokerKind::OopsAll6s,
    ]);
    state.consumable_capacity = 1;
    let mut hand = hand_of([card(
        Rank::Eight,
        Suit::Clubs,
        Enhancement::None,
        Seal::None,
    )]);
    state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!(state.consumables.len(), 1);
}

#[test]
fn eight_ball_skips_non_eights_stone_kickers_debuffs_and_full_capacity() {
    for condition in 0..6 {
        let mut state = state_with(&[
            JokerKind::EightBall,
            JokerKind::OopsAll6s,
            JokerKind::OopsAll6s,
        ]);
        state.blind.hands_remaining = 1;
        if condition == 4 {
            state.consumable_capacity = 0;
            state.jokers.get_mut(1).unwrap().set_debuffed(true);
            state.jokers.get_mut(2).unwrap().set_debuffed(true);
        }
        if condition == 5 {
            state.jokers.get_mut(0).unwrap().set_debuffed(true);
        }
        if condition == 3 {
            state.begin_blind_as(BlindKind::Boss(BossBlinds::Club));
            state.blind.hands_remaining = 1;
        }
        let mut cards = vec![card(
            if condition == 0 {
                Rank::Seven
            } else {
                Rank::Eight
            },
            Suit::Clubs,
            if condition == 1 {
                Enhancement::Stone
            } else {
                Enhancement::None
            },
            Seal::Red,
        )];
        if condition == 2 {
            cards.push(card(Rank::Ace, Suit::Hearts, Enhancement::None, Seal::None));
        }
        let mut hand = hand_of(cards);
        state.rng = ChaCha12Rng::seed_from_u64(612);
        let mut oracle = ChaCha12Rng::seed_from_u64(612);
        let positions = (0..hand.hand_size()).collect();
        state.play_hand(&mut hand, positions).unwrap();
        assert!(state.consumables.is_empty());
        assert_eq!(state.rng.random::<u64>(), oracle.random::<u64>());
    }
}

#[test]
fn eight_ball_seeded_probability_and_generation_are_repeatable() {
    let mut saw_success = false;
    let mut saw_failure = false;
    for seed in 0..30 {
        let mut state = state_with(&[JokerKind::EightBall]);
        state.blind.hands_remaining = 1;
        state.rng = ChaCha12Rng::seed_from_u64(seed);
        let mut oracle = ChaCha12Rng::seed_from_u64(seed);
        let success = oracle.random_bool(0.25);
        let mut hand = hand_of([card(
            Rank::Eight,
            Suit::Clubs,
            Enhancement::None,
            Seal::None,
        )]);
        state.play_hand(&mut hand, vec![0]).unwrap();
        assert_eq!(state.consumables.len(), usize::from(success));
        saw_success |= success;
        saw_failure |= !success;
    }
    assert!(saw_success && saw_failure);
}

fn straight_cards(ace: bool, flush: bool) -> Vec<Card> {
    let ranks = if ace {
        [Rank::Ten, Rank::Jack, Rank::Queen, Rank::King, Rank::Ace]
    } else {
        [Rank::Two, Rank::Three, Rank::Four, Rank::Five, Rank::Six]
    };
    ranks
        .into_iter()
        .enumerate()
        .map(|(index, rank)| {
            card(
                rank,
                if flush || index != 1 {
                    Suit::Clubs
                } else {
                    Suit::Hearts
                },
                Enhancement::None,
                Seal::None,
            )
        })
        .collect()
}

#[test]
fn superposition_requires_scoring_ace_and_accepts_straight_flush() {
    for ace in [false, true] {
        for flush in [false, true] {
            let mut state = state_with(&[JokerKind::Superposition]);
            let mut hand = hand_of(straight_cards(ace, flush));
            let result = state.play_hand(&mut hand, vec![0, 1, 2, 3, 4]).unwrap();
            assert_eq!(
                result.hand,
                if flush {
                    PokerHand::StraightFlush
                } else {
                    PokerHand::Straight
                }
            );
            assert_eq!(state.consumables.len(), usize::from(ace));
        }
    }
}

#[test]
fn seance_requires_straight_flush_and_emits_spectral() {
    for flush in [false, true] {
        let mut state = state_with(&[JokerKind::Seance]);
        let mut hand = hand_of(straight_cards(false, flush));
        state.play_hand(&mut hand, vec![0, 1, 2, 3, 4]).unwrap();
        assert_eq!(state.consumables.len(), usize::from(flush));
        if flush {
            assert!(matches!(
                state.consumables[0].consumable,
                Consumable::Spectral(_)
            ));
        }
    }
}

#[test]
fn generation_categories_reserve_capacity_in_after_hand_joker_order() {
    for reversed in [false, true] {
        let mut state = state_with(if reversed {
            &[JokerKind::Seance, JokerKind::Superposition]
        } else {
            &[JokerKind::Superposition, JokerKind::Seance]
        });
        state.consumable_capacity = 1;
        let mut hand = hand_of(straight_cards(true, true));
        let result = state.play_hand(&mut hand, vec![0, 1, 2, 3, 4]).unwrap();
        assert_eq!(
            (result.chips, result.mult, result.score),
            (151, 8.0, 1208.0)
        );
        assert_eq!(state.consumables.len(), 1);
        assert!(if reversed {
            matches!(state.consumables[0].consumable, Consumable::Spectral(_))
        } else {
            matches!(state.consumables[0].consumable, Consumable::Tarot(_))
        });
    }
}

#[test]
fn debuffed_copied_generators_and_whole_hand_debuff_do_not_generate() {
    for kind in [
        JokerKind::Vagabond,
        JokerKind::Superposition,
        JokerKind::Seance,
    ] {
        for whole_hand in [false, true] {
            let mut state = state_with(&[JokerKind::Blueprint, kind]);
            if whole_hand {
                state.begin_blind_as(BlindKind::Boss(BossBlinds::Mouth));
                state.blind.current_hand = Some(PokerHand::HighCard);
                state.blind.played_hand_types.push(PokerHand::HighCard);
            } else {
                state.jokers.get_mut(1).unwrap().set_debuffed(true);
            }
            let mut hand = hand_of(straight_cards(true, true));
            state.play_hand(&mut hand, vec![0, 1, 2, 3, 4]).unwrap();
            assert!(state.consumables.is_empty());
        }
    }
}

#[test]
fn delayed_gratification_pays_once_after_win_and_not_after_earlier_hands() {
    let mut state = state_with(&[JokerKind::DelayedGratification, JokerKind::Bootstraps]);
    let mut hand = hand_of([
        card(Rank::Two, Suit::Clubs, Enhancement::None, Seal::None),
        card(Rank::King, Suit::Hearts, Enhancement::None, Seal::None),
    ]);
    let first = state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!((first.mult, state.money), (1.0, 0));
    state.blind.score_requirement = 1;
    let second = state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!(second.mult, 1.0);
    assert_eq!(
        state.money, 14,
        "$8 delayed payout + $3 reward + $1 interest + $2 unused hands"
    );
    assert!(state.finish_blind(&hand).is_none());
    assert_eq!(state.money, 14);
}

#[test]
fn delayed_gratification_does_not_pay_after_used_discard_or_loss() {
    for lost in [false, true] {
        let mut state = state_with(&[JokerKind::DelayedGratification]);
        let mut hand = hand_of([card(Rank::Two, Suit::Clubs, Enhancement::None, Seal::None)]);
        if lost {
            state.blind.hands_remaining = 1;
        } else {
            state.discard_selected_cards(&mut hand, vec![0]).unwrap();
            state.blind.score_requirement = 1;
        }
        state.play_hand(&mut hand, vec![0]).unwrap();
        assert_eq!(state.money, if lost { 0 } else { 6 });
    }
}

#[test]
fn normal_game_action_records_score_and_enters_shop_after_economy_and_generation() {
    let mut game = Game::new(613);
    game.apply_action(GameAction::SelectBlind(BlindKind::Small))
        .unwrap();
    game.state.money = 0;
    game.state.blind.score_requirement = 1;
    game.state
        .jokers
        .add(Joker::create_joker(JokerKind::GoldenTicket));
    game.state
        .jokers
        .add(Joker::create_joker(JokerKind::RoughGem));
    game.state.jokers.add(Joker::create_joker(JokerKind::Bull));
    game.state
        .jokers
        .add(Joker::create_joker(JokerKind::Vagabond));
    let played = &mut game.hand.cards_mut()[0];
    played.set_rank(Rank::King);
    played.set_suit(Suit::Diamonds);
    played.set_enhancement(Enhancement::Gold);
    played.set_seal(Seal::Red);
    game.apply_action(GameAction::PlayCards(vec![0])).unwrap();
    assert_eq!(game.state.blind.chips_scored, 45);
    assert_eq!(
        game.state.money, 18,
        "$10 card income + $3 reward + $2 interest + $3 unused hands"
    );
    assert!(game.state.consumables.is_empty());
    assert_eq!(game.decision_point(), DecisionPoint::Shop);
}

#[test]
fn eight_ball_oops_doubles_its_generation_probability() {
    for doubled in [false, true] {
        for seed in 0..30 {
            let mut state = state_with(if doubled {
                &[JokerKind::EightBall, JokerKind::OopsAll6s]
            } else {
                &[JokerKind::EightBall]
            });
            state.blind.hands_remaining = 1;
            state.rng = ChaCha12Rng::seed_from_u64(seed);
            let mut oracle = ChaCha12Rng::seed_from_u64(seed);
            let success = oracle.random_bool(if doubled { 0.5 } else { 0.25 });
            let mut hand = hand_of([card(
                Rank::Eight,
                Suit::Clubs,
                Enhancement::None,
                Seal::None,
            )]);
            state.play_hand(&mut hand, vec![0]).unwrap();
            assert_eq!(state.consumables.len(), usize::from(success));
        }
    }
}

#[test]
fn copied_after_hand_generators_emit_once_per_ability_not_per_retrigger() {
    for kind in [JokerKind::Superposition, JokerKind::Seance] {
        let mut state = state_with(&[JokerKind::Blueprint, kind, JokerKind::HangingChad]);
        let mut hand = hand_of(straight_cards(true, true));
        hand.cards_mut()[0].set_seal(Seal::Red);
        let result = state.play_hand(&mut hand, vec![0, 1, 2, 3, 4]).unwrap();
        assert_eq!(
            (result.chips, result.mult, result.score),
            (181, 8.0, 1448.0)
        );
        assert_eq!(state.consumables.len(), 2);
        assert!(
            state
                .consumables
                .iter()
                .all(|owned| if kind == JokerKind::Seance {
                    matches!(owned.consumable, Consumable::Spectral(_))
                } else {
                    matches!(owned.consumable, Consumable::Tarot(_))
                })
        );
    }
}

#[test]
fn generated_outputs_and_rng_are_identical_for_repeated_seeded_hands() {
    let mut states = [
        state_with(&[JokerKind::EightBall, JokerKind::Vagabond]),
        state_with(&[JokerKind::EightBall, JokerKind::Vagabond]),
    ];
    let mut hands = [
        hand_of([card(Rank::Eight, Suit::Clubs, Enhancement::None, Seal::Red)]),
        hand_of([card(Rank::Eight, Suit::Clubs, Enhancement::None, Seal::Red)]),
    ];
    let left = states[0].play_hand(&mut hands[0], vec![0]).unwrap();
    let right = states[1].play_hand(&mut hands[1], vec![0]).unwrap();
    assert_eq!(left, right);
    assert_eq!(
        states[0]
            .consumables
            .iter()
            .map(|owned| owned.consumable)
            .collect::<Vec<_>>(),
        states[1]
            .consumables
            .iter()
            .map(|owned| owned.consumable)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        hands[0].cards().iter().map(Card::id).collect::<Vec<_>>(),
        hands[1].cards().iter().map(Card::id).collect::<Vec<_>>()
    );
    assert_eq!(states[0].rng.random::<u64>(), states[1].rng.random::<u64>());
}

#[test]
fn delayed_gratification_copy_is_incompatible_and_hook_does_not_count_as_used_discard() {
    let mut state = state_with(&[JokerKind::Blueprint, JokerKind::DelayedGratification]);
    state.begin_blind_as(BlindKind::Boss(BossBlinds::Hook));
    state.blind.score_requirement = 1;
    let mut hand = hand_of([
        card(Rank::Two, Suit::Clubs, Enhancement::None, Seal::None),
        card(Rank::Three, Suit::Clubs, Enhancement::None, Seal::None),
        card(Rank::Four, Suit::Clubs, Enhancement::None, Seal::None),
    ]);
    state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!(state.blind.discards_used, 0);
    assert_eq!(state.deck.discarded_cards().len(), 3);
    assert_eq!(
        state.money, 17,
        "$8 Delayed Gratification + $5 boss + $1 interest + $3 unused hands"
    );
}

#[test]
fn winning_game_action_keeps_generated_tarots_through_the_shop_transition() {
    let mut game = Game::new(614);
    game.apply_action(GameAction::SelectBlind(BlindKind::Small))
        .unwrap();
    game.state.money = 4;
    game.state.blind.score_requirement = 1;
    game.state
        .jokers
        .add(Joker::create_joker(JokerKind::Vagabond));
    game.state
        .jokers
        .add(Joker::create_joker(JokerKind::Superposition));
    for (index, rank) in [Rank::Ten, Rank::Jack, Rank::Queen, Rank::King, Rank::Ace]
        .into_iter()
        .enumerate()
    {
        game.hand.cards_mut()[index].set_rank(rank);
        game.hand.cards_mut()[index].set_suit(Suit::Clubs);
    }
    game.apply_action(GameAction::PlayCards(vec![0, 1, 2, 3, 4]))
        .unwrap();
    assert_eq!(game.state.blind.chips_scored, 1208);
    assert_eq!(game.state.money, 10);
    assert_eq!(game.state.consumables.len(), 2);
    assert!(
        game.state
            .consumables
            .iter()
            .all(|owned| matches!(owned.consumable, Consumable::Tarot(_)))
    );
    assert_eq!(game.hand.cards().len(), 3);
    assert_eq!(game.state.deck.discarded_cards().len(), 5);
    assert_eq!(game.decision_point(), DecisionPoint::Shop);
}
