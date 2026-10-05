use super::*;

use crate::core::card::Card;
use crate::core::enums::{Edition, Enhancement, Rank, Seal, Suit};
use crate::core::game::GameState;
use crate::core::joker_types::JokerKind;

fn card(rank: Rank, suit: Suit, enhancement: Enhancement) -> Card {
    Card::new(rank, suit, enhancement, Edition::None, Seal::None)
}

#[test]
fn mail_in_rebate_uses_the_currently_selected_rank() {
    let mut jokers = Jokers::new();
    jokers.add(Joker::create_joker(JokerKind::MailInRebate));
    let mut state = GameState::new(14);
    jokers.initialize(&mut state);

    let target = match &jokers.jokers[0].structure {
        JokerStructure::Normal(JokerData::Econ(data)) => data.rank.as_ref().unwrap()[0],
        _ => panic!("Mail-In Rebate did not contain economic data"),
    };
    let cards = vec![
        card(target, Suit::Hearts, Enhancement::None),
        card(target, Suit::Clubs, Enhancement::None),
        card(Rank::Ace, Suit::Spades, Enhancement::None),
    ];

    jokers.trigger(TriggerEvent::Discard { cards: &cards }, &mut state);

    assert_eq!(state.money, 10);
}

#[test]
fn golden_ticket_pays_for_gold_played_cards() {
    let mut jokers = Jokers::new();
    jokers.add(Joker::create_joker(JokerKind::GoldenTicket));
    let gold = card(Rank::Seven, Suit::Hearts, Enhancement::Gold);
    let plain = card(Rank::Seven, Suit::Hearts, Enhancement::None);
    let mut state = GameState::new(15);

    jokers.trigger(
        TriggerEvent::PlayedCard {
            card: &gold,
            played_cards: std::slice::from_ref(&gold),
            held_cards: &[],
            card_index: 0,
            hand_type: None,
            hands_remaining: 4,
        },
        &mut state,
    );
    let after_gold = state.money;
    jokers.trigger(
        TriggerEvent::PlayedCard {
            card: &plain,
            played_cards: std::slice::from_ref(&plain),
            held_cards: &[],
            card_index: 0,
            hand_type: None,
            hands_remaining: 4,
        },
        &mut state,
    );

    assert_eq!(after_gold, 4);
    assert_eq!(state.money, after_gold);
}

#[test]
fn matador_pays_only_when_a_boss_ability_triggers() {
    let mut jokers = Jokers::new();
    jokers.add(Joker::create_joker(JokerKind::Matador));
    let mut state = GameState::new(16);

    jokers.trigger(TriggerEvent::AfterHand {
        played_cards: &[],
        held_cards: &[],
        hand_type: None,
        hands_remaining: 3,
        discards_remaining: 4,
        joker_count: 1,
    }, &mut state);
    assert_eq!(state.money, 0);

    jokers.trigger(TriggerEvent::BossBlindAbilityTriggered, &mut state);
    assert_eq!(state.money, 8);
}
