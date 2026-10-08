use super::*;

use crate::core::blind::{BlindKind, calculate_score, score_played_hand};
use crate::core::card::Card;
use crate::core::enums::{BossBlinds, Decks, Edition, Enhancement, Rank, Seal, Stakes, Suit};
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
    assert_eq!(state.blind.hands_remaining, 4);
    assert_eq!(state.blind.discards_remaining, 4);
}

#[test]
fn blind_state_owns_the_active_blind_counters_and_kind() {
    let mut state = GameState::new(8);

    state.begin_blind_as(BlindKind::Boss(BossBlinds::Hook));

    assert!(state.blind.kind.is_boss());
    assert_eq!(state.blind.hands_remaining, 4);
    assert_eq!(state.blind.discards_remaining, 4);
    assert_eq!(state.blind.chips_scored, 0);
    assert_eq!(state.blind.stake, Stakes::White);
    assert_eq!(state.blind.score_requirement, 600);
}

#[test]
fn blind_score_requirement_uses_stake_ante_and_blind_kind() {
    let mut state = GameState::new(9);
    state.blind.stake = Stakes::Green;
    state.counters.ante = 2;

    state.begin_blind_as(BlindKind::Small);
    assert_eq!(state.blind.score_requirement, 900);
    state.begin_blind_as(BlindKind::Big);
    assert_eq!(state.blind.score_requirement, 1_350);
    state.begin_blind_as(BlindKind::Boss(BossBlinds::Hook));
    assert_eq!(state.blind.score_requirement, 1_800);
}

#[test]
fn blind_score_requirement_uses_endless_scaling_after_ante_eight() {
    let mut state = GameState::new(10);
    state.blind.stake = Stakes::Gold;
    state.counters.ante = 9;

    state.begin_blind_as(BlindKind::Small);

    let expected = crate::consts::ante_scores::endless_ante_stakes(Stakes::Gold, 9)[0] as u64;
    assert_eq!(state.blind.score_requirement, expected);
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
fn played_hand_partition_uses_the_poker_calculation() {
    let mut hand = Hand::new();
    hand.add_card(Card::new(
        Rank::Two,
        Suit::Spades,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));
    hand.add_card(Card::new(
        Rank::Five,
        Suit::Hearts,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));
    hand.add_card(Card::new(
        Rank::Ace,
        Suit::Clubs,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));

    let (hand_type, scoring) = score_played_hand(&hand, &[0, 1, 2]).unwrap();

    assert_eq!(hand_type, PokerHand::HighCard);
    assert_eq!(scoring.scoring_played_cards.len(), 1);
    assert_eq!(scoring.scoring_played_cards[0].index, 2);
    assert_eq!(scoring.non_scoring_played_cards.len(), 2);
    assert!(scoring.scoring_held_cards.is_empty());
    assert!(scoring.non_scoring_held_cards.is_empty());
}

#[test]
fn played_hand_scoring_uses_only_selected_cards() {
    let mut hand = Hand::new();
    for (rank, suit) in [
        (Rank::Two, Suit::Spades),
        (Rank::Five, Suit::Hearts),
        (Rank::Ace, Suit::Clubs),
        (Rank::Nine, Suit::Diamonds),
        (Rank::Seven, Suit::Hearts),
        (Rank::King, Suit::Spades),
        (Rank::Queen, Suit::Clubs),
        (Rank::Jack, Suit::Diamonds),
    ] {
        hand.add_card(Card::new(
            rank,
            suit,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ));
    }

    let (hand_type, scoring) = score_played_hand(&hand, &[0, 1, 2, 3, 4]).unwrap();

    assert_eq!(hand_type, PokerHand::HighCard);
    assert_eq!(scoring.scoring_played_cards[0].index, 2);
    assert_eq!(scoring.non_scoring_played_cards.len(), 4);
}

#[test]
fn selecting_cards_removes_them_and_preserves_the_remaining_hand() {
    let mut hand = Hand::new();
    for rank in [Rank::Two, Rank::Three, Rank::Four, Rank::Five] {
        hand.add_card(Card::new(
            rank,
            Suit::Spades,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ));
    }

    let selected = hand.select_cards(vec![0, 2]);

    assert_eq!(selected.len(), 2);
    assert_eq!(selected[0].rank(), Rank::Two);
    assert_eq!(selected[1].rank(), Rank::Four);
    assert_eq!(hand.hand_size(), 2);
    assert_eq!(hand.card(0).unwrap().rank(), Rank::Three);
    assert_eq!(hand.card(1).unwrap().rank(), Rank::Five);
}

#[test]
fn calculate_score_combines_hand_base_and_scoring_card_chips() {
    let mut state = GameState::new(31);
    let mut hand = Hand::new();
    hand.add_card(Card::new(
        Rank::Ace,
        Suit::Spades,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));

    let result = calculate_score(&mut state, &hand, &[0]).unwrap();

    assert_eq!(result.hand, PokerHand::HighCard);
    assert_eq!(result.chips, 16);
    assert_eq!(result.mult, 1.0);
    assert_eq!(result.score, 16.0);
    assert!(result.on_played_scoring_jokers.is_empty());
}

#[test]
fn calculate_score_finds_on_played_scoring_jokers() {
    let mut state = GameState::new(32);
    state.jokers.add(Joker::create_joker(JokerKind::Greedy));
    state
        .jokers
        .add(Joker::create_joker(JokerKind::GoldenTicket));

    let mut hand = Hand::new();
    hand.add_card(Card::new(
        Rank::Ace,
        Suit::Diamonds,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));

    let result = calculate_score(&mut state, &hand, &[0]).unwrap();

    assert_eq!(result.on_played_scoring_jokers, vec![0]);
    assert_eq!(result.mult, 4.0);
    assert_eq!(result.score, 64.0);
}

#[test]
fn cards_build_scoring_values_from_attributes_and_permanent_modifiers() {
    let mut card = Card::new(
        Rank::Ace,
        Suit::Spades,
        Enhancement::Bonus,
        Edition::Foil,
        Seal::None,
    );
    card.scoring_values_mut().chips += 7;

    let values = card.scoring_values();

    assert_eq!(values.chips, 98);
    assert_eq!(values.add_mult, 0);
    assert_eq!(values.x_mult, 1.0);
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
