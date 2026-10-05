use super::*;

use crate::core::card::Card;
use crate::core::enums::{Decks, Edition, Enhancement, Rank, Seal, Suit};
use crate::core::hand::Hand;
use crate::core::joker::Joker;
use crate::core::joker::TriggerEvent;
use crate::core::joker_types::JokerKind;

#[test]
fn game_state_owns_the_run_collections_and_deck_counts() {
    let state = GameState::new(1);

    assert_eq!(state.deck_size(), 52);
    assert_eq!(state.starting_deck_size(), 52);
    assert_eq!(state.hand_size(), 8);
    assert!(state.jokers.as_slice().is_empty());
    assert!(state.vouchers.is_empty());
    assert!(state.consumables.is_empty());
    assert_eq!(state.counters.hands_remaining, 4);
    assert_eq!(state.counters.discards_remaining, 4);
}

#[test]
fn deck_hand_size_modifiers_are_part_of_the_owned_deck() {
    let black_deck = GameState::with_deck(2, Decks::Black);
    let painted_deck = GameState::with_deck(3, Decks::Painted);

    assert_eq!(black_deck.hand_size(), 7);
    assert_eq!(painted_deck.hand_size(), 10);
}

#[test]
fn dealing_uses_the_seeded_game_rng() {
    let mut first_state = GameState::new(5);
    let mut second_state = GameState::new(5);
    let mut first_hand = Hand::new();
    let mut second_hand = Hand::new();

    first_state.deal_cards(&mut first_hand);
    second_state.deal_cards(&mut second_hand);

    let first_cards: Vec<u16> = first_hand.cards().iter().map(Card::id).collect();
    let second_cards: Vec<u16> = second_hand.cards().iter().map(Card::id).collect();
    assert_eq!(first_cards, second_cards);
}

#[test]
fn owned_jokers_dispatch_through_game_state_without_reordering() {
    let mut state = GameState::new(4);
    state.jokers.add(Joker::create_joker(JokerKind::Joker));
    let played_card = Card::new(
        Rank::Seven,
        Suit::Spades,
        Enhancement::None,
        Edition::None,
        Seal::None,
    );

    let effect = state.trigger_jokers(TriggerEvent::AfterHand {
        played_cards: std::slice::from_ref(&played_card),
        held_cards: &[],
        hand_type: None,
        hands_remaining: 3,
        discards_remaining: 4,
        joker_count: 1,
    });

    assert_eq!(effect.add_mult, 4);
    assert_eq!(state.jokers.as_slice().len(), 1);
}
