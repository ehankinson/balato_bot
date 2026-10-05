use super::*;

use crate::core::card::Card;
use crate::core::enums::{Edition, Enhancement, Rank, Seal, Suit};
use crate::core::game::GameState;
use crate::core::joker_types::JokerKind;

fn card(rank: Rank) -> Card {
    Card::new(
        rank,
        Suit::Spades,
        Enhancement::None,
        Edition::None,
        Seal::None,
    )
}

#[test]
fn sock_and_buskin_only_retriggers_face_cards() {
    let mut jokers = Jokers::new();
    jokers.add(Joker::create_joker(JokerKind::SockAndBuskin));
    let face = card(Rank::Queen);
    let low = card(Rank::Seven);
    let mut game_state = GameState::new(5);

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
    let low_effect = jokers.trigger(
        TriggerEvent::PlayedCard {
            card: &low,
            played_cards: std::slice::from_ref(&low),
            held_cards: &[],
            card_index: 0,
            hand_type: None,
            hands_remaining: 4,
        },
        &mut game_state,
    );

    assert_eq!(face_effect.retriggers, 1);
    assert_eq!(low_effect.retriggers, 0);
}

#[test]
fn hanging_chad_retriggers_the_first_played_card_twice() {
    let mut jokers = Jokers::new();
    jokers.add(Joker::create_joker(JokerKind::HangingChad));
    let played_cards = vec![card(Rank::Two), card(Rank::Three)];
    let first = &played_cards[0];
    let second = &played_cards[1];
    let mut game_state = GameState::new(6);

    let first_effect = jokers.trigger(
        TriggerEvent::PlayedCard {
            card: &first,
            played_cards: &played_cards,
            held_cards: &[],
            card_index: 0,
            hand_type: None,
            hands_remaining: 4,
        },
        &mut game_state,
    );
    let second_effect = jokers.trigger(
        TriggerEvent::PlayedCard {
            card: &second,
            played_cards: &played_cards,
            held_cards: &[],
            card_index: 1,
            hand_type: None,
            hands_remaining: 4,
        },
        &mut game_state,
    );

    assert_eq!(first_effect.retriggers, 2);
    assert_eq!(second_effect.retriggers, 0);
}
