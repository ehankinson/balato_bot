use super::*;

use crate::core::card::Card;
use crate::core::enums::{Edition, Enhancement, PokerHand, Rank, Seal, Suit};
use crate::core::game::GameState;
use crate::core::joker_types::JokerKind;

fn card(rank: Rank, suit: Suit) -> Card {
    Card::new(rank, suit, Enhancement::None, Edition::None, Seal::None)
}

#[test]
fn suit_scoring_joker_only_scores_matching_played_cards() {
    let mut jokers = Jokers::new();
    jokers.add(Joker::create_joker(JokerKind::Greedy));
    let diamonds = card(Rank::Seven, Suit::Diamonds);
    let spades = card(Rank::Seven, Suit::Spades);
    let mut game_state = GameState::new(1);

    let matching = jokers.trigger(
        TriggerEvent::PlayedCard {
            card: &diamonds,
            played_cards: std::slice::from_ref(&diamonds),
            held_cards: &[],
            card_index: 0,
            hand_type: Some(PokerHand::HighCard),
            hands_remaining: 4,
        },
        &mut game_state,
    );
    let non_matching = jokers.trigger(
        TriggerEvent::PlayedCard {
            card: &spades,
            played_cards: std::slice::from_ref(&spades),
            held_cards: &[],
            card_index: 0,
            hand_type: Some(PokerHand::HighCard),
            hands_remaining: 4,
        },
        &mut game_state,
    );

    assert_eq!(matching.add_mult, 3);
    assert_eq!(non_matching.add_mult, 0);
}

#[test]
fn poker_hand_scoring_joker_checks_the_hand_type() {
    let mut jokers = Jokers::new();
    jokers.add(Joker::create_joker(JokerKind::Jolly));
    let played = card(Rank::Seven, Suit::Diamonds);
    let mut game_state = GameState::new(2);

    let pair = jokers.trigger(
        TriggerEvent::AfterHand {
            played_cards: std::slice::from_ref(&played),
            held_cards: &[],
            hand_type: Some(PokerHand::Pair),
            hands_remaining: 3,
            discards_remaining: 4,
            joker_count: 1,
        },
        &mut game_state,
    );
    let straight = jokers.trigger(
        TriggerEvent::AfterHand {
            played_cards: std::slice::from_ref(&played),
            held_cards: &[],
            hand_type: Some(PokerHand::Straight),
            hands_remaining: 3,
            discards_remaining: 4,
            joker_count: 1,
        },
        &mut game_state,
    );

    assert_eq!(pair.add_mult, 8);
    assert_eq!(straight.add_mult, 0);
}

#[test]
fn face_card_trigger_uses_the_card_condition() {
    let mut jokers = Jokers::new();
    jokers.add(Joker::create_joker(JokerKind::ScaryFace));
    let face = card(Rank::King, Suit::Spades);
    let number = card(Rank::Seven, Suit::Spades);
    let mut game_state = GameState::new(3);

    let face_effect = jokers.trigger(
        TriggerEvent::PlayedCard {
            card: &face,
            played_cards: std::slice::from_ref(&face),
            held_cards: &[],
            card_index: 0,
            hand_type: None,
            hands_remaining: 4,
        },
        &mut game_state,
    );
    let number_effect = jokers.trigger(
        TriggerEvent::PlayedCard {
            card: &number,
            played_cards: std::slice::from_ref(&number),
            held_cards: &[],
            card_index: 0,
            hand_type: None,
            hands_remaining: 4,
        },
        &mut game_state,
    );

    assert_eq!(face_effect.chips, 30);
    assert_eq!(number_effect.chips, 0);
}

#[test]
fn dynamic_joker_values_are_accumulated_without_allocating_results() {
    let mut jokers = Jokers::new();
    jokers.add(Joker::create_joker(JokerKind::Abstract));
    jokers.add(Joker::create_joker(JokerKind::Joker));
    jokers.add(Joker::create_joker(JokerKind::Joker));
    let played = card(Rank::Seven, Suit::Spades);
    let mut game_state = GameState::new(4);

    let effect = jokers.trigger(
        TriggerEvent::AfterHand {
            played_cards: std::slice::from_ref(&played),
            held_cards: &[],
            hand_type: Some(PokerHand::HighCard),
            hands_remaining: 3,
            discards_remaining: 4,
            joker_count: 3,
        },
        &mut game_state,
    );

    assert_eq!(effect.add_mult, 14);
}
