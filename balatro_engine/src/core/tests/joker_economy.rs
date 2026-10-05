use super::*;

use crate::core::card::Card;
use crate::core::enums::{Edition, Enhancement, PokerHand, Rank, Seal, Suit};
use crate::core::game::GameState;
use crate::core::joker_types::{GenerateType, JokerKind};

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

    jokers.trigger(
        TriggerEvent::AfterHand {
            played_cards: &[],
            held_cards: &[],
            hand_type: None,
            hands_remaining: 3,
            discards_remaining: 4,
            joker_count: 1,
        },
        &mut state,
    );
    assert_eq!(state.money, 0);

    jokers.trigger(TriggerEvent::BossBlindAbilityTriggered, &mut state);
    assert_eq!(state.money, 8);
}

#[test]
fn round_end_economy_jokers_pay_only_for_their_current_state() {
    let mut cloud_nine = Jokers::new();
    cloud_nine.add(Joker::create_joker(JokerKind::Cloud9));
    let mut state = GameState::new(17);
    state.nines_in_deck = 3;
    assert_eq!(
        cloud_nine
            .update(UpdateEvent::RoundCompleted, &mut state)
            .money,
        3
    );
    state.nines_in_deck = 0;
    assert_eq!(
        cloud_nine
            .update(UpdateEvent::RoundCompleted, &mut state)
            .money,
        0
    );

    let mut to_the_moon = Jokers::new();
    to_the_moon.add(Joker::create_joker(JokerKind::ToTheMoon));
    let mut interest_state = GameState::new(18);
    interest_state.money = 10;
    assert_eq!(
        to_the_moon
            .update(UpdateEvent::RoundCompleted, &mut interest_state)
            .money,
        2
    );
    let mut zero_state = GameState::new(19);
    assert_eq!(
        to_the_moon
            .update(UpdateEvent::RoundCompleted, &mut zero_state)
            .money,
        0
    );

    let mut golden = Jokers::new();
    golden.add(Joker::create_joker(JokerKind::Golden));
    let mut golden_state = GameState::new(20);
    assert_eq!(
        golden
            .update(UpdateEvent::RoundCompleted, &mut golden_state)
            .money,
        4
    );

    let mut satellite = Jokers::new();
    satellite.add(Joker::create_joker(JokerKind::Satellite));
    let mut satellite_state = GameState::new(21);
    satellite_state.unique_planet_cards_used = 2;
    assert_eq!(
        satellite
            .update(UpdateEvent::RoundCompleted, &mut satellite_state)
            .money,
        2
    );
}

#[test]
fn rocket_and_egg_accumulate_only_at_their_lifecycle_events() {
    let mut rocket = Jokers::new();
    rocket.add(Joker::create_joker(JokerKind::Rocket));
    let mut rocket_state = GameState::new(22);
    assert_eq!(
        rocket
            .update(UpdateEvent::RoundCompleted, &mut rocket_state)
            .money,
        1
    );
    rocket.update(UpdateEvent::BossBlindCompleted, &mut rocket_state);
    assert_eq!(
        rocket
            .update(UpdateEvent::RoundCompleted, &mut rocket_state)
            .money,
        3
    );
    let mut debuffed_rocket = Jokers::new();
    debuffed_rocket.add(Joker::create_joker(JokerKind::Rocket));
    debuffed_rocket.jokers[0].debuffed = true;
    assert_eq!(
        debuffed_rocket.update(UpdateEvent::RoundCompleted, &mut rocket_state),
        JokerEffect::default()
    );

    let mut egg = Jokers::new();
    egg.add(Joker::create_joker(JokerKind::Egg));
    let mut egg_state = GameState::new(23);
    let initial_value = egg.jokers[0].sell_value;
    egg.update(UpdateEvent::RoundCompleted, &mut egg_state);
    egg.update(UpdateEvent::RoundCompleted, &mut egg_state);
    assert_eq!(egg.jokers[0].sell_value, initial_value + 6);
    assert_eq!(
        egg.update(
            UpdateEvent::BlindSelected { is_boss: false },
            &mut egg_state
        ),
        JokerEffect::default()
    );
}

#[test]
fn todo_list_pays_for_the_selected_hand_only() {
    let mut jokers = Jokers::new();
    jokers.add(Joker::create_joker(JokerKind::ToDoList));
    let mut state = GameState::new(24);
    jokers.initialize(&mut state);
    let target = match &jokers.jokers[0].structure {
        JokerStructure::Normal(JokerData::Econ(data)) => data.poker_hand,
        _ => panic!("To Do List did not contain economic data"),
    };
    let target = target.expect("To Do List did not choose a target hand");
    let played = card(Rank::Seven, Suit::Spades, Enhancement::None);

    let paid = jokers.trigger(
        TriggerEvent::AfterHand {
            played_cards: std::slice::from_ref(&played),
            held_cards: &[],
            hand_type: Some(target),
            hands_remaining: 3,
            discards_remaining: 4,
            joker_count: 1,
        },
        &mut state,
    );
    assert_eq!(paid.money, 5);
    assert_eq!(state.money, 5);

    let wrong_hand = if target == PokerHand::Pair {
        PokerHand::Flush
    } else {
        PokerHand::Pair
    };
    assert_eq!(
        jokers
            .trigger(
                TriggerEvent::AfterHand {
                    played_cards: std::slice::from_ref(&played),
                    held_cards: &[],
                    hand_type: Some(wrong_hand),
                    hands_remaining: 2,
                    discards_remaining: 4,
                    joker_count: 1,
                },
                &mut state,
            )
            .money,
        0
    );
}

#[test]
fn sold_jokers_apply_their_own_sale_effect_and_respect_debuffs() {
    let mut diet_cola = Jokers::new();
    diet_cola.add(Joker::create_joker(JokerKind::DietCola));
    let mut state = GameState::new(25);
    let effect = diet_cola.sell(0, &mut state);
    assert_eq!(effect.generated, 1);
    assert_eq!(effect.generated_type, Some(GenerateType::DoubleTag));
    assert!(diet_cola.as_slice().is_empty());

    let mut debuffed_diet_cola = Jokers::new();
    debuffed_diet_cola.add(Joker::create_joker(JokerKind::DietCola));
    debuffed_diet_cola.jokers[0].debuffed = true;
    assert_eq!(
        debuffed_diet_cola.sell(0, &mut state),
        JokerEffect::default()
    );

    let mut golden = Jokers::new();
    golden.add(Joker::create_joker(JokerKind::Golden));
    golden.jokers[0].debuffed = true;
    assert_eq!(
        golden.update(UpdateEvent::RoundCompleted, &mut state),
        JokerEffect::default()
    );
}

#[test]
fn mail_in_rebate_has_a_zero_payout_for_nonmatching_and_empty_discards() {
    let mut jokers = Jokers::new();
    jokers.add(Joker::create_joker(JokerKind::MailInRebate));
    let mut state = GameState::new(26);
    jokers.initialize(&mut state);
    let target = match &jokers.jokers[0].structure {
        JokerStructure::Normal(JokerData::Econ(data)) => data.rank.as_ref().unwrap()[0],
        _ => panic!("Mail-In Rebate did not contain economic data"),
    };
    let non_matching = if target == Rank::Ace {
        Rank::Two
    } else {
        Rank::Ace
    };
    let cards = [card(non_matching, Suit::Hearts, Enhancement::None)];
    assert_eq!(
        jokers
            .trigger(TriggerEvent::Discard { cards: &cards }, &mut state)
            .money,
        0
    );
    assert_eq!(
        jokers
            .trigger(TriggerEvent::Discard { cards: &[] }, &mut state)
            .money,
        0
    );
}
