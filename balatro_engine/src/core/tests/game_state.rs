use super::*;

use crate::core::blind::{BlindKind, calculate_score, score_played_hand};
use crate::core::card::Card;
use crate::core::enums::{BossBlinds, Decks, Edition, Enhancement, Rank, Seal, Stakes, Suit};
use crate::core::hand::Hand;
use crate::core::joker::Joker;
use crate::core::joker::TriggerEvent;
use crate::core::joker_types::JokerKind;

fn card(rank: Rank, suit: Suit, enhancement: Enhancement, edition: Edition, seal: Seal) -> Card {
    Card::new(rank, suit, enhancement, edition, seal)
}

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
fn red_deck_adds_one_discard_to_the_white_stake_default() {
    let red = GameState::with_deck(101, Decks::Red);
    let blue = GameState::with_deck(102, Decks::Blue);

    assert_eq!(red.hands_per_blind(), 4);
    assert_eq!(red.discards_per_blind(), 4);
    assert_eq!(blue.hands_per_blind(), 4);
    assert_eq!(blue.discards_per_blind(), 3);
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
fn boss_blind_start_applies_target_and_resource_rules() {
    let mut state = GameState::new(11);
    state.counters.ante = 2;

    state.begin_blind_as(BlindKind::Boss(BossBlinds::Wall));
    assert_eq!(state.blind.score_requirement, 3_200);

    state.begin_blind_as(BlindKind::Boss(BossBlinds::Water));
    assert_eq!(state.blind.discards_remaining, 0);

    state.begin_blind_as(BlindKind::Boss(BossBlinds::Manacle));
    assert_eq!(state.hand_size(), 7);

    state.begin_blind_as(BlindKind::Boss(BossBlinds::Needle));
    assert_eq!(state.blind.hands_remaining, 1);
    assert_eq!(state.blind.score_requirement, 800);
}

#[test]
fn suit_boss_debuffs_cards_without_changing_hand_detection() {
    let mut state = GameState::new(12);
    state.begin_blind_as(BlindKind::Boss(BossBlinds::Club));
    let mut hand = Hand::new();
    hand.add_card(card(
        Rank::Ace,
        Suit::Clubs,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));

    let result = calculate_score(&mut state, &hand, &[0]).unwrap();

    assert_eq!(result.hand, PokerHand::HighCard);
    assert_eq!(result.chips, 5);
}

#[test]
fn psychic_rejects_a_play_with_fewer_than_five_cards() {
    let mut state = GameState::new(13);
    state.begin_blind_as(BlindKind::Boss(BossBlinds::Psychich));
    let mut hand = Hand::new();
    for rank in [Rank::Two, Rank::Three, Rank::Four, Rank::Five] {
        hand.add_card(card(
            rank,
            Suit::Spades,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ));
    }

    assert!(calculate_score(&mut state, &hand, &[0, 1, 2, 3]).is_none());
}

#[test]
fn eye_rejects_repeating_a_hand_type() {
    let mut state = GameState::new(14);
    state.begin_blind_as(BlindKind::Boss(BossBlinds::Eye));
    let mut hand = Hand::new();
    hand.add_card(card(
        Rank::Ace,
        Suit::Spades,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));

    assert!(calculate_score(&mut state, &hand, &[0]).is_some());
    assert!(calculate_score(&mut state, &hand, &[0]).is_none());
}

#[test]
fn house_and_wheel_draws_mark_cards_face_down() {
    let mut house = GameState::new(15);
    house.begin_blind_as(BlindKind::Boss(BossBlinds::House));
    let mut house_hand = Hand::new();
    house.deal_cards(&mut house_hand);
    assert!(house_hand.cards().iter().all(Card::is_face_down));

    let mut wheel = GameState::new(16);
    wheel.begin_blind_as(BlindKind::Boss(BossBlinds::Wheel));
    let mut wheel_hand = Hand::new();
    wheel.deal_cards(&mut wheel_hand);
    let mut saw_face_down = wheel_hand.cards().iter().any(Card::is_face_down);
    for _ in 0..32 {
        let mut card = card(
            Rank::Ace,
            Suit::Spades,
            Enhancement::None,
            Edition::None,
            Seal::None,
        );
        wheel.blind.prepare_drawn_card(&mut card, &mut wheel.rng);
        saw_face_down |= card.is_face_down();
    }
    assert!(saw_face_down);
}

#[test]
fn boss_pool_unlocks_by_ante_and_uses_showdowns_at_ante_eight() {
    let mut state = GameState::new(17);
    state.counters.ante = 1;
    assert!(!BossBlinds::Wall.available_at_ante(1));
    assert!(!BossBlinds::Ox.available_at_ante(5));
    let ordinary = state.random_boss_blind();
    assert!(ordinary.available_at_ante(1));

    state.counters.ante = 8;
    let showdown = state.random_boss_blind();
    assert!(BossBlinds::SHOWDOWN.contains(&showdown));
}

#[test]
fn blind_selector_contains_the_three_blind_choices_and_regenerates_by_ante() {
    let mut state = GameState::new(24);
    let initial = state.blind_selector;

    assert_eq!(initial.small().kind, BlindKind::Small);
    assert_eq!(initial.big().kind, BlindKind::Big);
    assert!(matches!(initial.boss().kind, BlindKind::Boss(_)));
    assert!(initial.small().skip_tag.is_some());
    assert!(initial.big().skip_tag.is_some());
    assert_eq!(initial.boss().skip_tag, None);

    state.begin_next_ante();
    assert_eq!(state.counters.ante, 2);
    assert_eq!(state.blind_selector.small().kind, BlindKind::Small);
    assert_eq!(state.blind_selector.big().kind, BlindKind::Big);

    state.counters.ante = 7;
    state.begin_next_ante();
    assert_eq!(state.counters.ante, 8);
    let boss = match state.blind_selector.boss().kind {
        BlindKind::Boss(boss) => boss,
        _ => panic!("selector boss choice must be a boss blind"),
    };
    assert!(BossBlinds::SHOWDOWN.contains(&boss));
}

#[test]
fn regular_and_finisher_boss_encounters_use_separate_histories() {
    let mut state = GameState::new(25);
    let regular_before = state.regular_boss_blinds.total();

    state.counters.ante = 7;
    state.begin_next_ante();

    assert_eq!(state.regular_boss_blinds.total(), regular_before);
    assert_eq!(state.finisher_boss_blinds.total(), 1);
}

#[test]
fn disabling_a_boss_restores_its_normal_target() {
    let mut state = GameState::new(18);
    state.counters.ante = 2;
    state.begin_blind_as(BlindKind::Boss(BossBlinds::Wall));
    assert_eq!(state.blind.score_requirement, 3_200);

    state.blind.disable_boss_ability();

    assert_eq!(state.blind.score_requirement, 1_600);
}

#[test]
fn play_hand_runs_hook_discard_and_refills_the_hand() {
    let mut state = GameState::new(19);
    state.begin_blind_as(BlindKind::Boss(BossBlinds::Hook));
    let mut hand = Hand::new();
    state.deal_cards(&mut hand);
    let starting_deck_size = state.deck_size();

    let score = state.play_hand(&mut hand, vec![0]).unwrap();

    assert_eq!(score.hand, PokerHand::HighCard);
    assert_eq!(hand.hand_size(), 8);
    assert_eq!(state.blind.hands_remaining, 3);
    assert_eq!(state.deck_size(), starting_deck_size - 3);
}

#[test]
fn play_hand_finishes_and_pays_when_the_target_is_reached() {
    let mut state = GameState::new(20);
    state.begin_blind_as(BlindKind::Small);
    state.blind.score_requirement = 1;
    let mut hand = Hand::new();
    state.deal_cards(&mut hand);

    state.play_hand(&mut hand, vec![0]).unwrap();

    assert!(state.blind.finished);
    assert!(state.blind.has_beaten_target());
    assert_eq!(state.money, 3);
}

#[test]
fn completing_a_boss_pays_its_reward_and_clears_pillar_marks() {
    let mut state = GameState::new(20);
    state.begin_blind_as(BlindKind::Boss(BossBlinds::Hook));
    state.money = 10;
    state.pillar_card_ids.push(123);
    state.blind.chips_scored = state.blind.score_requirement;

    assert_eq!(
        state.finish_blind().unwrap(),
        crate::core::game::BlindPayout {
            blind_reward: 5,
            interest: 2,
            total: 7,
        }
    );
    assert_eq!(state.money, 17);
    assert!(state.pillar_card_ids.is_empty());
}

#[test]
fn finish_blind_pays_small_and_big_blinds_with_capped_interest() {
    let mut small = GameState::new(22);
    small.money = 20;
    small.begin_blind_as(BlindKind::Small);
    small.blind.chips_scored = small.blind.score_requirement;
    assert_eq!(
        small.finish_blind().unwrap(),
        crate::core::game::BlindPayout {
            blind_reward: 3,
            interest: 4,
            total: 7,
        }
    );
    assert_eq!(small.money, 27);

    let mut big = GameState::new(23);
    big.money = 100;
    big.redeem_voucher(crate::core::enums::Vouchers::SeedMoney)
        .unwrap();
    big.begin_blind_as(BlindKind::Big);
    big.blind.chips_scored = big.blind.score_requirement;
    assert_eq!(
        big.finish_blind().unwrap(),
        crate::core::game::BlindPayout {
            blind_reward: 4,
            interest: 10,
            total: 14,
        }
    );
    assert_eq!(big.money, 114);
}

#[test]
fn crimson_heart_debuffs_a_joker_at_start_and_after_play() {
    let mut state = GameState::new(21);
    state.jokers.add(Joker::create_joker(JokerKind::Joker));
    state.jokers.add(Joker::create_joker(JokerKind::Greedy));
    state.begin_blind_as(BlindKind::Boss(BossBlinds::CrimsonHeart));
    assert_eq!(
        state
            .jokers
            .as_slice()
            .iter()
            .filter(|joker| joker.is_debuffed())
            .count(),
        1
    );

    let mut hand = Hand::new();
    hand.add_card(card(
        Rank::Ace,
        Suit::Spades,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));
    calculate_score(&mut state, &hand, &[0]).unwrap();
    assert_eq!(
        state
            .jokers
            .as_slice()
            .iter()
            .filter(|joker| joker.is_debuffed())
            .count(),
        1
    );
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
