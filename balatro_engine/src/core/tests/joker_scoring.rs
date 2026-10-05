use super::*;

use crate::core::card::Card;
use crate::core::enums::{Edition, Enhancement, PokerHand, Rank, Seal, Suit};
use crate::core::game::GameState;
use crate::core::joker_types::JokerKind;

fn card(rank: Rank, suit: Suit) -> Card {
    Card::new(
        rank,
        suit,
        Enhancement::None,
        Edition::None,
        Seal::None,
    )
}

fn played_effect(kind: JokerKind, card: &Card, hand_type: Option<PokerHand>) -> JokerEffect {
    let mut jokers = Jokers::new();
    jokers.add(Joker::create_joker(kind));
    let mut state = GameState::new(100);
    jokers.trigger(
        TriggerEvent::PlayedCard {
            card,
            played_cards: std::slice::from_ref(card),
            held_cards: &[],
            card_index: 0,
            hand_type,
            hands_remaining: 4,
        },
        &mut state,
    )
}

fn hand_effect(
    kind: JokerKind,
    played_cards: &[Card],
    held_cards: &[Card],
    hand_type: Option<PokerHand>,
) -> JokerEffect {
    let mut jokers = Jokers::new();
    jokers.add(Joker::create_joker(kind));
    let mut state = GameState::new(101);
    jokers.trigger(
        TriggerEvent::AfterHand {
            played_cards,
            held_cards,
            hand_type,
            hands_remaining: 3,
            discards_remaining: 4,
            joker_count: 1,
        },
        &mut state,
    )
}

fn hand_effect_with_state(
    kind: JokerKind,
    played_cards: &[Card],
    held_cards: &[Card],
    hand_type: Option<PokerHand>,
    state: &mut GameState,
    joker_count: u8,
) -> JokerEffect {
    let mut jokers = Jokers::new();
    jokers.add(Joker::create_joker(kind));
    jokers.trigger(
        TriggerEvent::AfterHand {
            played_cards,
            held_cards,
            hand_type,
            hands_remaining: 3,
            discards_remaining: state.discards_remaining,
            joker_count,
        },
        state,
    )
}

#[test]
fn suit_jokers_score_only_their_matching_suit() {
    for (kind, suit) in [
        (JokerKind::Greedy, Suit::Diamonds),
        (JokerKind::Lusty, Suit::Hearts),
        (JokerKind::Wrathful, Suit::Spades),
        (JokerKind::Gluttonous, Suit::Clubs),
    ] {
        let matching = card(Rank::Seven, suit);
        let non_matching = card(
            Rank::Seven,
            match suit {
                Suit::Hearts => Suit::Spades,
                _ => Suit::Hearts,
            },
        );
        assert_eq!(played_effect(kind, &matching, None).add_mult, 3);
        assert_eq!(played_effect(kind, &non_matching, None).add_mult, 0);
    }
}

#[test]
fn poker_hand_jokers_accept_only_their_configured_hand() {
    for (kind, required_hand, add_mult, chips) in [
        (JokerKind::Jolly, PokerHand::Pair, 8, 0),
        (JokerKind::Zany, PokerHand::ThreeOfAKind, 12, 0),
        (JokerKind::Mad, PokerHand::TwoPair, 10, 0),
        (JokerKind::Crazy, PokerHand::Straight, 12, 0),
        (JokerKind::Droll, PokerHand::Flush, 10, 0),
        (JokerKind::Sly, PokerHand::Pair, 0, 50),
        (JokerKind::Wily, PokerHand::ThreeOfAKind, 0, 100),
        (JokerKind::Clever, PokerHand::TwoPair, 0, 80),
        (JokerKind::Devious, PokerHand::Straight, 0, 100),
        (JokerKind::Crafty, PokerHand::Flush, 0, 80),
    ] {
        let played = [card(Rank::Seven, Suit::Spades)];
        let matching = hand_effect(kind, &played, &[], Some(required_hand));
        let non_matching = hand_effect(kind, &played, &[], Some(PokerHand::HighCard));
        assert_eq!(matching.add_mult, add_mult);
        assert_eq!(matching.chips, chips);
        assert_eq!(non_matching, JokerEffect::default());
    }
}

#[test]
fn half_joker_uses_the_four_card_boundary() {
    let three = [
        card(Rank::Two, Suit::Spades),
        card(Rank::Three, Suit::Spades),
        card(Rank::Four, Suit::Spades),
    ];
    let four = [
        card(Rank::Two, Suit::Spades),
        card(Rank::Three, Suit::Spades),
        card(Rank::Four, Suit::Spades),
        card(Rank::Five, Suit::Spades),
    ];

    assert_eq!(hand_effect(JokerKind::Half, &three, &[], None).add_mult, 20);
    assert_eq!(hand_effect(JokerKind::Half, &four, &[], None).add_mult, 0);
}

#[test]
fn rank_based_jokers_reject_adjacent_non_matching_ranks() {
    let ace = card(Rank::Ace, Suit::Spades);
    let two = card(Rank::Two, Suit::Spades);
    let odd = card(Rank::Seven, Suit::Spades);
    let even = card(Rank::Eight, Suit::Spades);

    assert_eq!(played_effect(JokerKind::Fibonacci, &ace, None).add_mult, 8);
    assert_eq!(played_effect(JokerKind::Fibonacci, &two, None).add_mult, 8);
    assert_eq!(played_effect(JokerKind::Fibonacci, &odd, None).add_mult, 0);
    assert_eq!(played_effect(JokerKind::EvenSteven, &even, None).add_mult, 4);
    assert_eq!(played_effect(JokerKind::OddTodd, &odd, None).chips, 31);
    assert_eq!(played_effect(JokerKind::Scholar, &ace, None).add_mult, 4);
    assert_eq!(played_effect(JokerKind::Scholar, &ace, None).chips, 20);
    assert_eq!(played_effect(JokerKind::Scholar, &two, None), JokerEffect::default());
}

#[test]
fn face_and_queen_king_jokers_have_negative_cases() {
    let king = card(Rank::King, Suit::Hearts);
    let queen = card(Rank::Queen, Suit::Hearts);
    let number = card(Rank::Seven, Suit::Hearts);

    assert_eq!(played_effect(JokerKind::SmileyFace, &king, None).add_mult, 5);
    assert_eq!(played_effect(JokerKind::SmileyFace, &number, None).add_mult, 0);
    assert_eq!(played_effect(JokerKind::Triboulet, &king, None).x_mult, 2.0);
    assert_eq!(played_effect(JokerKind::Triboulet, &queen, None).x_mult, 2.0);
    assert_eq!(played_effect(JokerKind::Triboulet, &number, None).x_mult, 1.0);
}

#[test]
fn held_card_scoring_uses_the_held_cards_collection() {
    let steel = Card::new(
        Rank::Seven,
        Suit::Spades,
        Enhancement::Steel,
        Edition::None,
        Seal::None,
    );
    let steel_played = [card(Rank::King, Suit::Hearts)];
    let baron_card = card(Rank::King, Suit::Hearts);

    assert_eq!(
        hand_effect(JokerKind::Steel, &steel_played, &[steel], None).x_mult,
        1.2
    );
    assert_eq!(
        hand_effect(JokerKind::Steel, &steel_played, &[], None).x_mult,
        1.0
    );
    assert_eq!(
        played_effect(JokerKind::Baron, &baron_card, None).x_mult,
        1.5
    );
}

#[test]
fn special_scoring_jokers_respect_state_boundaries() {
    let played = [card(Rank::Seven, Suit::Spades)];

    let mut no_discards = GameState::new(102);
    no_discards.discards_remaining = 0;
    assert_eq!(
        hand_effect_with_state(
            JokerKind::MysticSummit,
            &played,
            &[],
            None,
            &mut no_discards,
            1,
        )
        .add_mult,
        15
    );

    let mut with_discards = GameState::new(103);
    with_discards.discards_remaining = 1;
    assert_eq!(
        hand_effect_with_state(
            JokerKind::MysticSummit,
            &played,
            &[],
            None,
            &mut with_discards,
            1,
        )
        .add_mult,
        0
    );

    let mut banner_state = GameState::new(104);
    banner_state.discards_remaining = 2;
    assert_eq!(
        hand_effect_with_state(
            JokerKind::Banner,
            &played,
            &[],
            None,
            &mut banner_state,
            1,
        )
        .chips,
        60
    );

    let mut money_state = GameState::new(105);
    money_state.money = 10;
    assert_eq!(
        hand_effect_with_state(
            JokerKind::Bull,
            &played,
            &[],
            None,
            &mut money_state,
            1,
        )
        .chips,
        20
    );
    assert_eq!(
        hand_effect_with_state(
            JokerKind::Bootstraps,
            &played,
            &[],
            None,
            &mut money_state,
            1,
        )
        .add_mult,
        4
    );
}

#[test]
fn deck_and_lineup_scoring_jokers_use_current_counts() {
    let played = [card(Rank::Seven, Suit::Spades)];

    let mut state = GameState::new(106);
    state.deck_size = 52;
    assert_eq!(
        hand_effect_with_state(JokerKind::Blue, &played, &[], None, &mut state, 1).chips,
        26
    );

    state.starting_deck_size = 52;
    state.deck_size = 50;
    assert_eq!(
        hand_effect_with_state(JokerKind::Erosion, &played, &[], None, &mut state, 1).add_mult,
        8
    );

    state.empty_joker_slots = 2;
    assert_eq!(
        hand_effect_with_state(JokerKind::Stencil, &played, &[], None, &mut state, 1).x_mult,
        3.0
    );

    state.current_hand = Some(PokerHand::Pair);
    assert_eq!(
        hand_effect_with_state(
            JokerKind::CardSharp,
            &played,
            &[],
            Some(PokerHand::Pair),
            &mut state,
            1,
        )
        .x_mult,
        3.0
    );
    assert_eq!(
        hand_effect_with_state(
            JokerKind::CardSharp,
            &played,
            &[],
            Some(PokerHand::Flush),
            &mut state,
            1,
        )
        .x_mult,
        1.0
    );
}

#[test]
fn flower_pot_and_seeing_double_require_their_full_hand_conditions() {
    let four_suits = [
        card(Rank::Two, Suit::Hearts),
        card(Rank::Three, Suit::Diamonds),
        card(Rank::Four, Suit::Clubs),
        card(Rank::Five, Suit::Spades),
    ];
    let missing_suit = [
        card(Rank::Two, Suit::Hearts),
        card(Rank::Three, Suit::Diamonds),
        card(Rank::Four, Suit::Clubs),
    ];
    assert_eq!(
        hand_effect(JokerKind::FlowerPot, &four_suits, &[], None).x_mult,
        3.0
    );
    assert_eq!(
        hand_effect(JokerKind::FlowerPot, &missing_suit, &[], None).x_mult,
        1.0
    );

    let club_and_heart = [
        card(Rank::Two, Suit::Clubs),
        card(Rank::Three, Suit::Hearts),
    ];
    let all_clubs = [
        card(Rank::Two, Suit::Clubs),
        card(Rank::Three, Suit::Clubs),
    ];
    assert_eq!(
        hand_effect(JokerKind::SeeingDouble, &club_and_heart, &[], None).x_mult,
        2.0
    );
    assert_eq!(
        hand_effect(JokerKind::SeeingDouble, &all_clubs, &[], None).x_mult,
        1.0
    );
}

#[test]
fn basic_and_held_hand_scoring_have_safe_negative_cases() {
    let played = card(Rank::Seven, Suit::Spades);
    assert_eq!(
        hand_effect(JokerKind::Joker, std::slice::from_ref(&played), &[], None).add_mult,
        4
    );

    let mut debuffed = Jokers::new();
    debuffed.add(Joker::create_joker(JokerKind::Joker));
    debuffed.jokers[0].debuffed = true;
    let mut state = GameState::new(107);
    assert_eq!(
        debuffed.trigger(
            TriggerEvent::AfterHand {
                played_cards: std::slice::from_ref(&played),
                held_cards: &[],
                hand_type: None,
                hands_remaining: 3,
                discards_remaining: 4,
                joker_count: 1,
            },
            &mut state,
        ),
        JokerEffect::default()
    );

    let played_cards = [card(Rank::Seven, Suit::Spades)];
    let blackboard_cards = [card(Rank::Two, Suit::Spades)];
    let heart = [card(Rank::Three, Suit::Hearts)];
    assert_eq!(
        hand_effect(
            JokerKind::Blackboard,
            &played_cards,
            &blackboard_cards,
            None,
        )
        .x_mult,
        3.0
    );
    assert_eq!(
        hand_effect(JokerKind::Blackboard, &played_cards, &heart, None).x_mult,
        1.0
    );
}

#[test]
fn poker_hand_x_mult_jokers_match_only_their_poker_hand() {
    let played = [card(Rank::Seven, Suit::Spades)];
    for (kind, hand, expected) in [
        (JokerKind::Duo, PokerHand::Pair, 2.0),
        (JokerKind::Trio, PokerHand::ThreeOfAKind, 3.0),
        (JokerKind::Family, PokerHand::FourOfAKind, 4.0),
        (JokerKind::Order, PokerHand::Straight, 3.0),
        (JokerKind::Tribe, PokerHand::Flush, 2.0),
    ] {
        assert_eq!(hand_effect(kind, &played, &[], Some(hand)).x_mult, expected);
        assert_eq!(
            hand_effect(kind, &played, &[], Some(PokerHand::HighCard)).x_mult,
            1.0
        );
    }
}

#[test]
fn state_scaled_and_held_rank_jokers_use_exact_thresholds() {
    let queen = card(Rank::Queen, Suit::Hearts);
    let held = [card(Rank::Queen, Suit::Hearts)];
    let non_queen = [card(Rank::Seven, Suit::Hearts)];
    let seven = card(Rank::Seven, Suit::Hearts);
    let mut state = GameState::new(108);

    let mut shoot_the_moon = Jokers::new();
    shoot_the_moon.add(Joker::create_joker(JokerKind::ShootTheMoon));
    assert_eq!(
        shoot_the_moon
            .trigger(
                TriggerEvent::HeldCard {
                    card: &queen,
                    held_cards: &held,
                    card_index: 0,
                },
                &mut state,
            )
            .add_mult,
        13
    );
    assert_eq!(
        shoot_the_moon
            .trigger(
                TriggerEvent::HeldCard {
                    card: &non_queen[0],
                    held_cards: &non_queen,
                    card_index: 0,
                },
                &mut state,
            )
            .add_mult,
        0
    );

    state.enhanced_cards = 15;
    assert_eq!(
        hand_effect_with_state(
            JokerKind::DriversLicense,
            std::slice::from_ref(&seven),
            &[],
            None,
            &mut state,
            1,
        )
        .x_mult,
        1.0
    );
    state.enhanced_cards = 16;
    assert_eq!(
        hand_effect_with_state(
            JokerKind::DriversLicense,
            std::slice::from_ref(&seven),
            &[],
            None,
            &mut state,
            1,
        )
        .x_mult,
        3.0
    );

    state.blinds_skipped = 2;
    assert_eq!(
        hand_effect_with_state(
            JokerKind::Throwback,
            std::slice::from_ref(&seven),
            &[],
            None,
            &mut state,
            1,
        )
        .x_mult,
        1.5
    );
}
