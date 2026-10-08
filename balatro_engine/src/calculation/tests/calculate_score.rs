use crate::calculation::calculate_score::calculate_score;
use crate::calculation::poker::determine_poker_hand;
use crate::core::blind::{ScoringCard, ScoringData};
use crate::core::card::Card;
use crate::core::enums::{Edition, Enhancement, PokerHand, Rank, Seal, Suit};
use crate::core::game::GameState;
use crate::core::joker::Joker;
use crate::core::joker_types::{JokerEdition, JokerKind};

fn card(rank: Rank, suit: Suit) -> Card {
    Card::new(rank, suit, Enhancement::None, Edition::None, Seal::None)
}

#[test]
fn played_card_retriggers_repeat_card_scoring() {
    let card = Card::new(
        Rank::Ace,
        Suit::Spades,
        Enhancement::None,
        Edition::None,
        Seal::None,
    );
    let mut state = GameState::new(1);
    state
        .jokers
        .add(Joker::create_joker(JokerKind::HangingChad));

    let scoring_data = ScoringData {
        scoring_played_cards: vec![ScoringCard {
            index: 0,
            id: card.id(),
        }],
        ..ScoringData::default()
    };
    let result = calculate_score(
        &mut state,
        std::slice::from_ref(&card),
        &[],
        &[0],
        PokerHand::HighCard,
        scoring_data,
    );

    assert_eq!(result.chips, 38);
    assert_eq!(result.mult, 1.0);
    assert_eq!(result.score, 38.0);
}

#[test]
fn mime_retriggers_all_held_scoring_jokers() {
    let played = Card::new(
        Rank::Ace,
        Suit::Spades,
        Enhancement::None,
        Edition::None,
        Seal::None,
    );
    let held = Card::new(
        Rank::Queen,
        Suit::Hearts,
        Enhancement::None,
        Edition::None,
        Seal::None,
    );
    let mut state = GameState::new(2);
    state.jokers.add(Joker::create_joker(JokerKind::Mime));
    state
        .jokers
        .add(Joker::create_joker(JokerKind::ShootTheMoon));

    let scoring_data = ScoringData {
        scoring_played_cards: vec![ScoringCard {
            index: 0,
            id: played.id(),
        }],
        scoring_held_cards: vec![ScoringCard {
            index: 0,
            id: held.id(),
        }],
        ..ScoringData::default()
    };
    let result = calculate_score(
        &mut state,
        std::slice::from_ref(&played),
        std::slice::from_ref(&held),
        &[0],
        PokerHand::HighCard,
        scoring_data,
    );

    assert_eq!(result.chips, 16);
    assert_eq!(result.mult, 27.0);
    assert_eq!(result.score, 432.0);
}

#[test]
fn steel_is_the_only_held_card_score_without_joker_intervention() {
    let played = Card::new(
        Rank::Ace,
        Suit::Spades,
        Enhancement::None,
        Edition::None,
        Seal::None,
    );
    let held = Card::new(
        Rank::Queen,
        Suit::Hearts,
        Enhancement::Steel,
        Edition::None,
        Seal::None,
    );
    let mut state = GameState::new(3);
    let scoring_data = ScoringData {
        scoring_played_cards: vec![ScoringCard {
            index: 0,
            id: played.id(),
        }],
        scoring_held_cards: vec![ScoringCard {
            index: 0,
            id: held.id(),
        }],
        ..ScoringData::default()
    };

    let result = calculate_score(
        &mut state,
        std::slice::from_ref(&played),
        std::slice::from_ref(&held),
        &[0],
        PokerHand::HighCard,
        scoring_data,
    );

    assert_eq!(result.chips, 16);
    assert_eq!(result.mult, 1.5);
    assert_eq!(result.score, 24.0);
}

#[test]
fn after_hand_scoring_jokers_apply_after_held_cards() {
    let played = Card::new(
        Rank::Ace,
        Suit::Spades,
        Enhancement::None,
        Edition::None,
        Seal::None,
    );
    let mut state = GameState::new(4);
    state.jokers.add(Joker::create_joker(JokerKind::Acrobat));

    let scoring_data = ScoringData {
        scoring_played_cards: vec![ScoringCard {
            index: 0,
            id: played.id(),
        }],
        ..ScoringData::default()
    };
    let result = calculate_score(
        &mut state,
        std::slice::from_ref(&played),
        &[],
        &[0],
        PokerHand::HighCard,
        scoring_data,
    );

    assert_eq!(result.chips, 16);
    assert_eq!(result.mult, 3.0);
    assert_eq!(result.score, 48.0);
}

#[test]
fn joker_editions_are_applied_in_the_after_hand_phase() {
    let played = Card::new(
        Rank::Queen,
        Suit::Spades,
        Enhancement::None,
        Edition::None,
        Seal::None,
    );
    let mut state = GameState::new(5);
    let mut sock_and_buskin = Joker::create_joker(JokerKind::SockAndBuskin);
    sock_and_buskin.set_edition(JokerEdition::Polychrome);
    state.jokers.add(sock_and_buskin);

    let scoring_data = ScoringData {
        scoring_played_cards: vec![ScoringCard {
            index: 0,
            id: played.id(),
        }],
        ..ScoringData::default()
    };
    let result = calculate_score(
        &mut state,
        std::slice::from_ref(&played),
        &[],
        &[0],
        PokerHand::HighCard,
        scoring_data,
    );

    assert_eq!(result.chips, 25);
    assert_eq!(result.mult, 1.5);
    assert_eq!(result.score, 37.0);
}

fn scoring_data_for(played: &[Card], held: &[Card]) -> (PokerHand, ScoringData) {
    let played_ids = played.iter().map(Card::id).collect::<Vec<_>>();
    let detected = determine_poker_hand(&played_ids).expect("test hand should be valid");
    let scoring_played_cards = detected
        .scoring_card_indices
        .iter()
        .map(|&index| ScoringCard {
            index,
            id: played[index].id(),
        })
        .collect();
    let scoring_held_cards = held
        .iter()
        .enumerate()
        .map(|(index, card)| ScoringCard {
            index,
            id: card.id(),
        })
        .collect();

    (
        detected.hand,
        ScoringData {
            scoring_played_cards,
            scoring_held_cards,
            ..ScoringData::default()
        },
    )
}

fn score_scenario(played: &[Card], held: &[Card], jokers: &[Joker]) -> f32 {
    let (hand_type, scoring_data) = scoring_data_for(played, held);
    let mut state = GameState::new(1000);
    for joker in jokers {
        state.jokers.add(joker.clone());
    }
    let hand_indices = (0..played.len()).collect::<Vec<_>>();
    calculate_score(
        &mut state,
        played,
        held,
        &hand_indices,
        hand_type,
        scoring_data,
    )
    .score
}

#[test]
fn scoring_scenarios_have_reviewable_final_scores() {
    let high_card = [card(Rank::Ace, Suit::Spades)];
    assert_eq!(score_scenario(&high_card, &[], &[]), 16.0, "base high card");

    let pair = [
        card(Rank::Seven, Suit::Spades),
        card(Rank::Seven, Suit::Hearts),
    ];
    assert_eq!(
        score_scenario(&pair, &[], &[Joker::create_joker(JokerKind::Jolly)]),
        240.0,
        "pair with Jolly Joker"
    );

    let suited_cards = [
        card(Rank::Ace, Suit::Diamonds),
        card(Rank::King, Suit::Diamonds),
        card(Rank::Queen, Suit::Diamonds),
    ];
    assert_eq!(
        score_scenario(
            &suited_cards,
            &[],
            &[
                Joker::create_joker(JokerKind::Greedy),
                Joker::create_joker(JokerKind::Lusty),
            ],
        ),
        64.0,
        "played suit Jokers"
    );

    let straight = [
        card(Rank::Two, Suit::Spades),
        card(Rank::Three, Suit::Hearts),
        card(Rank::Four, Suit::Clubs),
        card(Rank::Five, Suit::Diamonds),
        card(Rank::Six, Suit::Spades),
    ];
    assert_eq!(
        score_scenario(&straight, &[], &[Joker::create_joker(JokerKind::Crazy)],),
        800.0,
        "straight with Crazy Joker"
    );

    let held_steel = card(Rank::King, Suit::Hearts);
    let mut steel = held_steel.clone();
    steel.set_enhancement(Enhancement::Steel);
    assert_eq!(
        score_scenario(
            &high_card,
            &[steel],
            &[
                Joker::create_joker(JokerKind::Mime),
                Joker::create_joker(JokerKind::ShootTheMoon),
            ],
        ),
        36.0,
        "Steel held card with Mime and Shoot the Moon"
    );

    assert_eq!(
        score_scenario(&high_card, &[], &[Joker::create_joker(JokerKind::Acrobat)],),
        48.0,
        "after-hand Acrobat"
    );

    let mut poly_sock = Joker::create_joker(JokerKind::SockAndBuskin);
    poly_sock.set_edition(JokerEdition::Polychrome);
    let face = [card(Rank::Queen, Suit::Spades)];
    assert_eq!(
        score_scenario(&face, &[], &[poly_sock]),
        37.0,
        "Polychrome Sock and Buskin"
    );
}
