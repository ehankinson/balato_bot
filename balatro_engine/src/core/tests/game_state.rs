use super::*;

use crate::core::blind::{BlindKind, execute_hand as execute_hand_into, score_played_hand};
use crate::core::card::Card;
use crate::core::enums::{
    BossBlinds, Consumable, Decks, Edition, Enhancement, PokerHand, Rank, Seal, SkipTag, Stakes,
    Suit,
};
use crate::core::hand::Hand;
use crate::core::joker::TriggerEvent;
use crate::core::joker::{GeneratedConsumable, Joker, JokerOperation};
use crate::core::joker_types::JokerKind;

fn card(rank: Rank, suit: Suit, enhancement: Enhancement, edition: Edition, seal: Seal) -> Card {
    Card::new(rank, suit, enhancement, edition, seal)
}

fn execute_hand(state: &mut GameState, hand: &mut Hand, indices: &[usize]) -> Option<ScoreResult> {
    let mut played_cards = Vec::new();
    execute_hand_into(state, hand, indices, &mut played_cards)
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
    assert_eq!(blue.hands_per_blind(), 5);
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
fn blind_selection_dispatches_joker_lifecycle_updates() {
    let mut state = GameState::new(111);
    state.jokers.add(Joker::create_joker(JokerKind::Burglar));

    state.begin_blind_as(BlindKind::Small);

    assert_eq!(state.blind.hands_remaining, 7);
    assert_eq!(state.blind.discards_remaining, 0);
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

    let result = execute_hand(&mut state, &mut hand, &[0]).unwrap();

    assert_eq!(result.hand, PokerHand::HighCard);
    assert_eq!(result.chips, 5);
}

#[test]
fn psychic_consumes_a_non_scoring_play_with_fewer_than_five_cards() {
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

    assert_eq!(
        execute_hand(&mut state, &mut hand, &[0, 1, 2, 3])
            .unwrap()
            .score,
        0.0
    );
}

#[test]
fn eye_consumes_a_repeated_hand_type_without_scoring() {
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

    state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!(state.play_hand(&mut hand, vec![0]).unwrap().score, 0.0);
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
        wheel
            .blind
            .prepare_drawn_card(&mut card, &mut wheel.rng, wheel.jokers.modifiers());
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
fn discarding_triggers_jokers_and_refills_the_hand() {
    let mut state = GameState::new(112);
    state.money = 0;
    state.jokers.add(Joker::create_joker(JokerKind::Faceless));
    let mut hand = Hand::new();
    for rank in [Rank::Jack, Rank::Queen, Rank::King] {
        hand.add_card(card(
            rank,
            Suit::Spades,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ));
    }
    state.deal_cards(&mut hand);

    let discarded = state.discard_selected_cards(&mut hand, vec![0, 1, 2]);

    assert_eq!(discarded.unwrap().len(), 3);
    assert_eq!(hand.hand_size(), 8);
    assert_eq!(state.money, 5);
    assert_eq!(state.blind.discards_remaining, 3);
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
    assert_eq!(state.money, 10);
}

#[test]
fn completing_a_boss_pays_its_reward_and_clears_pillar_marks() {
    let mut state = GameState::new(20);
    state.begin_blind_as(BlindKind::Boss(BossBlinds::Hook));
    state.money = 10;
    state
        .deck
        .card_mut(0)
        .unwrap()
        .mark_played(state.counters.ante);
    state.blind.chips_scored = state.blind.score_requirement;

    assert_eq!(
        state.finish_blind(&Hand::new()).unwrap(),
        crate::core::game::BlindPayout {
            blind_reward: 5,
            interest: 2,
            hand_money: 4,
            discard_money: 0,
            total: 11,
        }
    );
    assert_eq!(state.money, 21);
    assert!(
        !state
            .deck
            .card(0)
            .unwrap()
            .was_played_in_ante(state.counters.ante)
    );
}

#[test]
fn finishing_a_boss_dispatches_boss_and_round_completion_events() {
    let mut state = GameState::new(113);
    state.jokers.add(Joker::create_joker(JokerKind::Rocket));
    state.begin_blind_as(BlindKind::Boss(BossBlinds::Hook));
    state.blind.chips_scored = state.blind.score_requirement;

    state.finish_blind(&Hand::new());

    assert_eq!(state.money, 17);
}

#[test]
fn finish_blind_pays_small_and_big_blinds_with_capped_interest() {
    let mut small = GameState::new(22);
    small.money = 20;
    small.begin_blind_as(BlindKind::Small);
    small.blind.chips_scored = small.blind.score_requirement;
    assert_eq!(
        small.finish_blind(&Hand::new()).unwrap(),
        crate::core::game::BlindPayout {
            blind_reward: 3,
            interest: 4,
            hand_money: 4,
            discard_money: 0,
            total: 11,
        }
    );
    assert_eq!(small.money, 31);

    let mut big = GameState::new(23);
    big.money = 100;
    big.redeem_voucher(crate::core::enums::Vouchers::SeedMoney)
        .unwrap();
    big.begin_blind_as(BlindKind::Big);
    big.blind.chips_scored = big.blind.score_requirement;
    assert_eq!(
        big.finish_blind(&Hand::new()).unwrap(),
        crate::core::game::BlindPayout {
            blind_reward: 4,
            interest: 10,
            hand_money: 4,
            discard_money: 0,
            total: 18,
        }
    );
    assert_eq!(big.money, 118);
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
    execute_hand(&mut state, &mut hand, &[0]).unwrap();
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

    assert_eq!(black_deck.hand_size(), 8);
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

    let (hand_type, scoring) = score_played_hand(&hand, &[0, 1, 2], Default::default()).unwrap();

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

    let (hand_type, scoring) =
        score_played_hand(&hand, &[0, 1, 2, 3, 4], Default::default()).unwrap();

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
fn execute_hand_combines_hand_base_and_scoring_card_chips() {
    let mut state = GameState::new(31);
    let mut hand = Hand::new();
    hand.add_card(Card::new(
        Rank::Ace,
        Suit::Spades,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));

    let result = execute_hand(&mut state, &mut hand, &[0]).unwrap();

    assert_eq!(result.hand, PokerHand::HighCard);
    assert_eq!(result.chips, 16);
    assert_eq!(result.mult, 1.0);
    assert_eq!(result.score, 16.0);
}

#[test]
fn played_card_jokers_contribute_to_the_score() {
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

    let result = execute_hand(&mut state, &mut hand, &[0]).unwrap();

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

    assert_eq!(values.chips, 48);
    assert_eq!(card.edition_scoring_values().chips, 50);
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
        scoring_card_indices: &[0],
        held_cards: &[],
        hand_type: None,
        hands_remaining: 3,
        discards_remaining: 4,
        joker_count: 1,
    });

    assert_eq!(effect.add_mult, 4);
    assert_eq!(state.jokers.as_slice().len(), 1);
}

#[test]
fn blind_start_materializes_each_generated_type_from_multiple_jokers() {
    let mut state = GameState::new(400);
    state.jokers.add(Joker::create_joker(JokerKind::Marble));
    state.jokers.add(Joker::create_joker(JokerKind::RiffRaff));

    let effect = state.update_jokers(UpdateEvent::BlindSelected { is_boss: false });

    assert_eq!(effect.operations.len(), 2);
    assert_eq!(
        effect.operations[0],
        JokerOperation::GenerateStoneCard { count: 1 }
    );
    assert_eq!(
        effect.operations[1],
        JokerOperation::GenerateJoker {
            rarity: crate::core::joker_types::JokerRarity::Common,
            count: 2,
        }
    );
    assert_eq!(state.deck_size(), 53);
    assert!(
        state
            .deck
            .cards()
            .iter()
            .any(|card| card.enhancement() == Enhancement::Stone)
    );
    assert_eq!(state.jokers.as_slice().len(), 4);
}

#[test]
fn certificate_and_consumable_generators_materialize_in_game_state() {
    let mut state = GameState::new(401);
    state
        .jokers
        .add(Joker::create_joker(JokerKind::Certificate));

    state.update_jokers(UpdateEvent::RoundStarted);

    assert!(
        state
            .deck
            .cards()
            .iter()
            .any(|card| card.seal() != Seal::None)
    );

    state.jokers.add(Joker::create_joker(JokerKind::Vagabond));
    state.money = 4;
    state.trigger_jokers(TriggerEvent::AfterHand {
        played_cards: &[],
        scoring_card_indices: &[],
        held_cards: &[],
        hand_type: Some(PokerHand::HighCard),
        hands_remaining: 3,
        discards_remaining: 4,
        joker_count: 2,
    });

    assert_eq!(state.consumables.len(), 1);
    assert!(matches!(
        state.consumables[0].consumable,
        Consumable::Tarot(_)
    ));

    state.jokers.add(Joker::create_joker(JokerKind::Perkeo));
    state.update_jokers(UpdateEvent::ShopClosed);
    assert_eq!(state.consumables.len(), 2);
    assert!(
        state
            .consumables
            .iter()
            .any(|consumable| consumable.negative)
    );
}

#[test]
fn generated_card_copies_and_gold_enhancement_are_applied_during_hand_play() {
    let mut dna_state = GameState::new(402);
    dna_state.jokers.add(Joker::create_joker(JokerKind::DNA));
    let dna_card = Card::new(
        Rank::Ace,
        Suit::Spades,
        Enhancement::None,
        Edition::Foil,
        Seal::Red,
    );
    let dna_id = dna_card.id();
    let mut dna_hand = Hand::new();
    dna_hand.add_card(dna_card);

    dna_state.play_hand(&mut dna_hand, vec![0]).unwrap();

    let matching_cards = dna_state
        .deck
        .cards()
        .iter()
        .chain(dna_state.deck.discarded_cards())
        .chain(dna_hand.cards())
        .filter(|card| card.id() == dna_id)
        .count();
    assert_eq!(matching_cards, 2);

    let mut midas_state = GameState::new(403);
    midas_state
        .jokers
        .add(Joker::create_joker(JokerKind::MidasMask));
    let mut midas_hand = Hand::new();
    midas_hand.add_card(Card::new(
        Rank::Seven,
        Suit::Spades,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));
    midas_hand.add_card(Card::new(
        Rank::Seven,
        Suit::Diamonds,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));
    midas_hand.add_card(Card::new(
        Rank::Queen,
        Suit::Hearts,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));

    midas_state
        .play_hand(&mut midas_hand, vec![0, 1, 2])
        .unwrap();

    assert_eq!(
        midas_state.deck.discarded_cards()[2].enhancement(),
        Enhancement::None
    );

    let mut sixth_sense_state = GameState::new(406);
    sixth_sense_state
        .jokers
        .add(Joker::create_joker(JokerKind::SixthSense));
    let mut sixth_sense_hand = Hand::new();
    sixth_sense_hand.add_card(Card::new(
        Rank::Six,
        Suit::Hearts,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));

    sixth_sense_state
        .play_hand(&mut sixth_sense_hand, vec![0])
        .unwrap();

    assert_eq!(sixth_sense_state.consumables.len(), 1);
    assert!(matches!(
        sixth_sense_state.consumables[0].consumable,
        Consumable::Spectral(_)
    ));
}

#[test]
fn dna_copies_once_even_when_hanging_chad_retriggers_the_scored_card() {
    let mut state = GameState::new(412);
    state.begin_blind_as(BlindKind::Small);
    state.jokers.add(Joker::create_joker(JokerKind::DNA));
    state
        .jokers
        .add(Joker::create_joker(JokerKind::HangingChad));
    let original = Card::new(
        Rank::Ace,
        Suit::Spades,
        Enhancement::None,
        Edition::Foil,
        Seal::Red,
    );
    let id = original.id();
    let mut hand = Hand::new();
    hand.add_card(original);

    state.play_hand(&mut hand, vec![0]).unwrap();

    let matching_cards = state
        .deck
        .cards()
        .iter()
        .chain(state.deck.discarded_cards())
        .chain(hand.cards())
        .filter(|card| card.id() == id)
        .count();
    assert_eq!(matching_cards, 2);
}

#[test]
fn dna_card_copy_updates_hologram_before_the_same_hand_scores() {
    let mut state = GameState::new(417);
    state.begin_blind_as(BlindKind::Small);
    state.jokers.add(Joker::create_joker(JokerKind::DNA));
    state.jokers.add(Joker::create_joker(JokerKind::Hologram));
    let mut hand = Hand::new();
    hand.add_card(Card::new(
        Rank::Queen,
        Suit::Clubs,
        Enhancement::Gold,
        Edition::Foil,
        Seal::Blue,
    ));

    let result = state.play_hand(&mut hand, vec![0]).unwrap();

    assert_eq!(result.mult, 1.25);
    let copies = state
        .deck
        .cards()
        .iter()
        .chain(state.deck.discarded_cards())
        .chain(hand.cards())
        .filter(|card| {
            card.rank() == Rank::Queen
                && card.suit() == Suit::Clubs
                && card.enhancement() == Enhancement::Gold
                && card.edition() == Edition::Foil
                && card.seal() == Seal::Blue
        })
        .count();
    assert_eq!(copies, 2);
}

#[test]
fn dna_does_not_copy_a_multi_card_first_hand_or_a_later_single_card_hand() {
    let mut state = GameState::new(423);
    state.begin_blind_as(BlindKind::Small);
    state.jokers.add(Joker::create_joker(JokerKind::DNA));
    let first = Card::new(
        Rank::Queen,
        Suit::Clubs,
        Enhancement::Stone,
        Edition::Foil,
        Seal::Red,
    );
    let second = Card::new(
        Rank::King,
        Suit::Diamonds,
        Enhancement::Gold,
        Edition::Holographic,
        Seal::Blue,
    );
    let first_id = first.id();
    let second_id = second.id();
    let mut hand = Hand::new();
    hand.add_card(first);
    hand.add_card(second);

    state.play_hand(&mut hand, vec![0, 1]).unwrap();
    hand.add_card(Card::new(
        Rank::Jack,
        Suit::Clubs,
        Enhancement::Steel,
        Edition::Foil,
        Seal::Gold,
    ));
    let later_id = hand.cards().last().unwrap().id();
    let later_position = hand.hand_size() - 1;
    state.play_hand(&mut hand, vec![later_position]).unwrap();

    for id in [first_id, second_id, later_id] {
        let copies = state
            .deck
            .cards()
            .iter()
            .chain(state.deck.discarded_cards())
            .chain(hand.cards())
            .filter(|card| card.id() == id)
            .count();
        assert_eq!(copies, 1);
    }
}

#[test]
fn midas_and_ticket_handle_two_identical_physical_face_cards_separately() {
    let mut state = GameState::new(424);
    state.begin_blind_as(BlindKind::Small);
    state.jokers.add(Joker::create_joker(JokerKind::MidasMask));
    state
        .jokers
        .add(Joker::create_joker(JokerKind::GoldenTicket));
    let queen = Card::new(
        Rank::Queen,
        Suit::Hearts,
        Enhancement::None,
        Edition::None,
        Seal::None,
    );
    let mut hand = Hand::new();
    hand.add_card(queen.clone());
    hand.add_card(queen);
    let initial_money = state.money;

    state.play_hand(&mut hand, vec![0, 1]).unwrap();

    assert_eq!(state.money, initial_money + 8);
    assert_eq!(
        state
            .deck
            .discarded_cards()
            .iter()
            .filter(|card| card.rank() == Rank::Queen && card.suit() == Suit::Hearts)
            .filter(|card| card.enhancement() == Enhancement::Gold)
            .count(),
        2
    );
}

#[test]
fn ceremonial_dagger_removes_its_neighbor_before_that_joker_updates() {
    let mut state = GameState::new(427);
    state
        .jokers
        .add(Joker::create_joker(JokerKind::CeremonialDagger));
    state.jokers.add(Joker::create_joker(JokerKind::RiffRaff));
    state.jokers.add(Joker::create_joker(JokerKind::Egg));

    state.begin_blind_as(BlindKind::Small);

    assert_eq!(
        state
            .jokers
            .as_slice()
            .iter()
            .map(Joker::kind)
            .collect::<Vec<_>>(),
        vec![JokerKind::CeremonialDagger, JokerKind::Egg]
    );
    assert_eq!(state.deck_size(), 52);
}

#[test]
fn debuffed_blueprint_does_not_copy_a_lifecycle_effect() {
    let mut state = GameState::new(428);
    state.begin_blind_as(BlindKind::Small);
    state.jokers.add(Joker::create_joker(JokerKind::Blueprint));
    state.jokers.add(Joker::create_joker(JokerKind::Burglar));
    state.jokers.as_mut_slice()[0].set_debuffed(true);

    let effect = state.update_jokers(UpdateEvent::BlindSelected { is_boss: false });

    assert_eq!(effect.hands, 3);
    assert_eq!(effect.discards, -4);
}

#[test]
fn blueprint_copy_does_not_apply_holograms_permanent_growth_twice() {
    for kinds in [
        [JokerKind::Blueprint, JokerKind::Hologram, JokerKind::DNA],
        [JokerKind::DNA, JokerKind::Blueprint, JokerKind::Hologram],
    ] {
        let mut state = GameState::new(429);
        state.begin_blind_as(BlindKind::Small);
        for kind in kinds {
            state.jokers.add(Joker::create_joker(kind));
        }
        let mut hand = Hand::new();
        hand.add_card(card(
            Rank::Queen,
            Suit::Clubs,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ));

        let result = state.play_hand(&mut hand, vec![0]).unwrap();

        assert_eq!(result.mult, 1.5625);
    }
}

#[test]
fn dna_copy_is_held_for_current_hand_scoring() {
    let mut state = GameState::new(431);
    state.begin_blind_as(BlindKind::Small);
    state.jokers.add(Joker::create_joker(JokerKind::DNA));
    state.jokers.add(Joker::create_joker(JokerKind::Baron));
    let mut hand = Hand::new();
    hand.add_card(card(
        Rank::King,
        Suit::Clubs,
        Enhancement::Steel,
        Edition::None,
        Seal::None,
    ));
    for _ in 0..7 {
        hand.add_card(card(
            Rank::Two,
            Suit::Clubs,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ));
    }

    let result = state.play_hand(&mut hand, vec![0]).unwrap();

    assert_eq!(result.mult, 2.25);
}

#[test]
fn delayed_gratification_pays_at_round_end_not_after_each_hand() {
    let mut state = GameState::new(430);
    state.begin_blind_as(BlindKind::Small);
    state
        .jokers
        .add(Joker::create_joker(JokerKind::DelayedGratification));
    let money_before_play = state.money;
    let mut hand = Hand::new();
    hand.add_card(card(
        Rank::Ace,
        Suit::Spades,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));
    state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!(state.money, money_before_play);

    state.update_jokers(UpdateEvent::RoundCompleted { held_cards: &[] });
    assert_eq!(state.money, money_before_play + 8);
}

#[test]
fn scoring_editions_apply_once_at_their_jokers_position() {
    let mut state = GameState::new(425);
    state.begin_blind_as(BlindKind::Small);
    let mut cavendish = Joker::create_joker(JokerKind::Cavendish);
    cavendish.set_edition(crate::core::joker_types::JokerEdition::Polychrome);
    state.jokers.add(cavendish);
    let mut hand = Hand::new();
    hand.add_card(card(
        Rank::Ace,
        Suit::Spades,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));

    let result = state.play_hand(&mut hand, vec![0]).unwrap();

    assert_eq!(result.mult, 4.5);
}

#[test]
fn repeated_seeded_runs_produce_identical_joker_and_card_results() {
    fn run(
        seed: u64,
    ) -> (
        crate::calculation::execute_hand::ScoreResult,
        Vec<u16>,
        Vec<u16>,
        Vec<JokerKind>,
    ) {
        let mut state = GameState::new(seed);
        state.jokers.add(Joker::create_joker(JokerKind::Marble));
        state.jokers.add(Joker::create_joker(JokerKind::RiffRaff));
        state.jokers.add(Joker::create_joker(JokerKind::DNA));
        state.jokers.add(Joker::create_joker(JokerKind::Hologram));
        state.begin_blind_as(BlindKind::Small);
        let mut hand = Hand::new();
        hand.add_card(card(
            Rank::Queen,
            Suit::Clubs,
            Enhancement::None,
            Edition::Foil,
            Seal::Purple,
        ));
        let result = state.play_hand(&mut hand, vec![0]).unwrap();
        (
            result,
            state.deck.cards().iter().map(Card::id).collect(),
            state.deck.discarded_cards().iter().map(Card::id).collect(),
            state.jokers.as_slice().iter().map(Joker::kind).collect(),
        )
    }

    assert_eq!(run(426), run(426));
}

#[test]
#[ignore = "release-mode Joker execution benchmark"]
fn joker_execution_benchmark() {
    use std::hint::black_box;
    use std::time::Instant;

    const ITERATIONS: usize = 4_000;
    const REPETITIONS: usize = 5;
    let workloads: [(&str, &[JokerKind]); 5] = [
        ("none", &[]),
        (
            "ordinary",
            &[
                JokerKind::Greedy,
                JokerKind::Fibonacci,
                JokerKind::Bootstraps,
            ],
        ),
        (
            "copy-chain",
            &[JokerKind::Blueprint, JokerKind::Blueprint, JokerKind::Jolly],
        ),
        ("retrigger", &[JokerKind::HangingChad, JokerKind::Mime]),
        ("generation", &[JokerKind::DNA, JokerKind::Hologram]),
    ];

    for repetition in 0..REPETITIONS {
        for (name, kinds) in workloads {
            let mut cases = (0..ITERATIONS)
                .map(|iteration| {
                    let mut state = GameState::new(10_000 + iteration as u64);
                    for &kind in kinds {
                        state.jokers.add(Joker::create_joker(kind));
                    }
                    state.begin_blind_as(BlindKind::Small);
                    let mut hand = Hand::new();
                    hand.add_card(card(
                        Rank::Ace,
                        Suit::Spades,
                        Enhancement::None,
                        Edition::None,
                        Seal::None,
                    ));
                    hand.add_card(card(
                        Rank::Queen,
                        Suit::Hearts,
                        Enhancement::None,
                        Edition::None,
                        Seal::None,
                    ));
                    (state, hand)
                })
                .collect::<Vec<_>>();
            let started = Instant::now();
            for (state, hand) in &mut cases {
                let mut played_cards = Vec::new();
                let result =
                    crate::core::blind::execute_hand(state, hand, &[0], &mut played_cards).unwrap();
                black_box((result.score, state.money, state.deck_size()));
            }
            let nanos_per_hand = started.elapsed().as_nanos() / ITERATIONS as u128;
            println!("dispatch rep={repetition} workload={name} ns/hand={nanos_per_hand}");
        }

        let started = Instant::now();
        for iteration in 0..ITERATIONS {
            let mut state = GameState::new(20_000 + iteration as u64);
            state.jokers.add(Joker::create_joker(JokerKind::DNA));
            state.jokers.add(Joker::create_joker(JokerKind::Hologram));
            state.begin_blind_as(BlindKind::Small);
            let mut hand = Hand::new();
            hand.add_card(card(
                Rank::Ace,
                Suit::Spades,
                Enhancement::None,
                Edition::None,
                Seal::None,
            ));
            let result = state.play_hand(&mut hand, vec![0]).unwrap();
            black_box((result.score, state.money, state.deck_size()));
        }
        let nanos_per_hand = started.elapsed().as_nanos() / ITERATIONS as u128;
        println!("full-play rep={repetition} workload=generation ns/hand={nanos_per_hand}");
    }
}

#[test]
fn blueprint_copies_dna_once_per_activation_without_card_retrigger_multiplication() {
    let mut state = GameState::new(418);
    state.begin_blind_as(BlindKind::Small);
    state.jokers.add(Joker::create_joker(JokerKind::Blueprint));
    state.jokers.add(Joker::create_joker(JokerKind::DNA));
    state
        .jokers
        .add(Joker::create_joker(JokerKind::HangingChad));
    let original = Card::new(
        Rank::Queen,
        Suit::Clubs,
        Enhancement::Steel,
        Edition::Holographic,
        Seal::Gold,
    );
    let mut hand = Hand::new();
    hand.add_card(original.clone());

    state.play_hand(&mut hand, vec![0]).unwrap();

    let copies = state
        .deck
        .cards()
        .iter()
        .chain(state.deck.discarded_cards())
        .chain(hand.cards())
        .filter(|card| card.id() == original.id())
        .count();
    assert_eq!(copies, 3);
}

#[test]
fn a_debuffed_blueprint_or_dna_target_does_not_copy_a_card() {
    for debuff_blueprint in [true, false] {
        let mut state = GameState::new(if debuff_blueprint { 419 } else { 420 });
        state.begin_blind_as(BlindKind::Small);
        state.jokers.add(Joker::create_joker(JokerKind::Blueprint));
        state.jokers.add(Joker::create_joker(JokerKind::DNA));
        state.jokers.as_mut_slice()[usize::from(!debuff_blueprint)].set_debuffed(true);
        let original = Card::new(
            Rank::Queen,
            Suit::Clubs,
            Enhancement::Stone,
            Edition::Foil,
            Seal::Red,
        );
        let id = original.id();
        let mut hand = Hand::new();
        hand.add_card(original);

        state.play_hand(&mut hand, vec![0]).unwrap();

        let matching_cards = state
            .deck
            .cards()
            .iter()
            .chain(state.deck.discarded_cards())
            .chain(hand.cards())
            .filter(|card| card.id() == id)
            .count();
        assert_eq!(matching_cards, if debuff_blueprint { 2 } else { 1 });
    }
}

#[test]
fn stencil_uses_live_joker_occupancy_and_capacity_during_scoring() {
    let mut state = GameState::new(421);
    state.begin_blind_as(BlindKind::Small);
    state.jokers.add(Joker::create_joker(JokerKind::Stencil));
    state.jokers.add(Joker::create_joker(JokerKind::Mime));
    state.jokers.increase_capacity(2);
    let mut hand = Hand::new();
    hand.add_card(card(
        Rank::Ace,
        Suit::Spades,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));
    hand.add_card(card(
        Rank::Queen,
        Suit::Hearts,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));

    let result = state.play_hand(&mut hand, vec![0]).unwrap();

    assert_eq!(result.mult, 6.0);
}

#[test]
fn selling_luchador_clears_debuffs_on_the_owned_joker_collection() {
    let mut state = GameState::new(422);
    state.begin_blind_as(BlindKind::Boss(BossBlinds::Hook));
    state.jokers.add(Joker::create_joker(JokerKind::Luchador));
    state.jokers.add(Joker::create_joker(JokerKind::Joker));
    state.jokers.as_mut_slice()[1].set_debuffed(true);

    state.sell_joker(0).unwrap();

    assert!(state.blind.boss_ability_disabled);
    assert!(!state.jokers.as_slice()[0].is_debuffed());
}

#[test]
fn midas_enhancement_is_visible_to_golden_ticket_in_either_joker_order() {
    for seed in [413, 414] {
        let mut state = GameState::new(seed);
        state.begin_blind_as(BlindKind::Small);
        let midas_first = seed == 413;
        if midas_first {
            state.jokers.add(Joker::create_joker(JokerKind::MidasMask));
            state
                .jokers
                .add(Joker::create_joker(JokerKind::GoldenTicket));
        } else {
            state
                .jokers
                .add(Joker::create_joker(JokerKind::GoldenTicket));
            state.jokers.add(Joker::create_joker(JokerKind::MidasMask));
        }
        let mut hand = Hand::new();
        hand.add_card(Card::new(
            Rank::Queen,
            Suit::Hearts,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ));
        let initial_money = state.money;

        state.play_hand(&mut hand, vec![0]).unwrap();

        assert_eq!(state.money, initial_money + 4);
        assert_eq!(
            state.deck.discarded_cards()[0].enhancement(),
            Enhancement::Gold
        );
    }
}

#[test]
fn todo_list_pays_before_bootstraps_scoring_in_either_collection_order() {
    let mut todo_first = GameState::new(415);
    todo_first.begin_blind_as(BlindKind::Small);
    let mut todo_first_jokers = crate::core::joker::Jokers::new();
    todo_first_jokers.add(Joker::create_joker(JokerKind::ToDoList));
    todo_first_jokers.add(Joker::create_joker(JokerKind::Bootstraps));
    todo_first_jokers.initialize(&mut todo_first);
    todo_first.jokers = todo_first_jokers;
    let mut bootstraps_first = GameState::new(415);
    bootstraps_first.begin_blind_as(BlindKind::Small);
    let mut bootstraps_first_jokers = crate::core::joker::Jokers::new();
    bootstraps_first_jokers.add(Joker::create_joker(JokerKind::Bootstraps));
    bootstraps_first_jokers.add(Joker::create_joker(JokerKind::ToDoList));
    bootstraps_first_jokers.initialize(&mut bootstraps_first);
    bootstraps_first.jokers = bootstraps_first_jokers;

    let played = [card(
        Rank::Ace,
        Suit::Spades,
        Enhancement::None,
        Edition::None,
        Seal::None,
    )];
    let hand_types = [
        PokerHand::HighCard,
        PokerHand::Pair,
        PokerHand::ThreeOfAKind,
        PokerHand::FourOfAKind,
        PokerHand::FiveOfAKind,
        PokerHand::TwoPair,
        PokerHand::Straight,
        PokerHand::Flush,
        PokerHand::FullHouse,
        PokerHand::StraightFlush,
        PokerHand::FlushHouse,
        PokerHand::FlushFive,
    ];
    for state in [&mut todo_first, &mut bootstraps_first] {
        state.money = 1;
        let matching_hand = hand_types
            .iter()
            .copied()
            .find(|&hand_type| {
                state.trigger_jokers(TriggerEvent::HandPlayed {
                    played_cards: &played,
                    scoring_card_indices: &[0],
                    hand_type,
                });
                state.money == 5
            })
            .expect("seeded To Do List hand target found");
        let scoring = state.trigger_jokers(TriggerEvent::AfterHand {
            played_cards: &played,
            scoring_card_indices: &(0..(&played).len()).collect::<Vec<_>>(),
            held_cards: &[],
            hand_type: Some(matching_hand),
            hands_remaining: 3,
            discards_remaining: 4,
            joker_count: 2,
        });
        assert_eq!(state.money, 5);
        assert_eq!(scoring.add_mult, 2);
    }
}

#[test]
fn burnt_joker_levels_the_discarded_hand_and_diet_cola_materializes_a_double_tag() {
    let mut burnt_state = GameState::new(404);
    burnt_state
        .jokers
        .add(Joker::create_joker(JokerKind::BurntJoker));
    let mut hand = Hand::new();
    hand.add_card(Card::new(
        Rank::Ace,
        Suit::Spades,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));

    burnt_state
        .discard_selected_cards(&mut hand, vec![0])
        .unwrap();
    assert_eq!(burnt_state.hand_score(PokerHand::HighCard).chips, 15);

    let mut tag_state = GameState::new(405);
    tag_state
        .jokers
        .add(Joker::create_joker(JokerKind::DietCola));
    tag_state.sell_joker(0).unwrap();
    assert!(tag_state.tags.held().contains(&SkipTag::Double));

    let mut invisible_state = GameState::new(411);
    invisible_state
        .jokers
        .add(Joker::create_joker(JokerKind::Invisible));
    invisible_state
        .jokers
        .add(Joker::create_joker(JokerKind::Jolly));
    invisible_state.update_jokers(UpdateEvent::RoundCompleted { held_cards: &[] });
    invisible_state.update_jokers(UpdateEvent::RoundCompleted { held_cards: &[] });
    invisible_state.sell_joker(0).unwrap();
    assert_eq!(invisible_state.jokers.as_slice().len(), 2);
    assert!(
        invisible_state
            .jokers
            .as_slice()
            .iter()
            .all(|joker| joker.kind() == JokerKind::Jolly)
    );
}

#[test]
fn generated_planet_consumables_use_the_shared_game_state_path() {
    let mut state = GameState::new(407);
    let generated = [JokerOperation::GenerateConsumable {
        kind: GeneratedConsumable::Planet,
        count: 1,
    }];

    state.materialize_generated_effects(&generated, &mut [], None);

    assert_eq!(state.consumables.len(), 1);
    assert!(matches!(
        state.consumables[0].consumable,
        Consumable::Planet(_)
    ));
}

#[test]
fn generated_results_respect_capacity_and_seeded_randomness() {
    let mut full_joker_state = GameState::new(408);
    for _ in 0..4 {
        full_joker_state
            .jokers
            .add(Joker::create_joker(JokerKind::Joker));
    }
    full_joker_state
        .jokers
        .add(Joker::create_joker(JokerKind::RiffRaff));
    full_joker_state.update_jokers(UpdateEvent::BlindSelected { is_boss: false });
    assert_eq!(full_joker_state.jokers.as_slice().len(), 5);

    let mut full_consumable_state = GameState::new(409);
    full_consumable_state
        .add_consumable(Consumable::Tarot(crate::core::enums::Tarot::Hermit))
        .unwrap();
    full_consumable_state
        .add_consumable(Consumable::Planet(crate::core::enums::Planet::Pluto))
        .unwrap();
    full_consumable_state
        .jokers
        .add(Joker::create_joker(JokerKind::Vagabond));
    full_consumable_state.money = 4;
    full_consumable_state.trigger_jokers(TriggerEvent::AfterHand {
        played_cards: &[],
        scoring_card_indices: &[],
        held_cards: &[],
        hand_type: Some(PokerHand::HighCard),
        hands_remaining: 3,
        discards_remaining: 4,
        joker_count: 1,
    });
    assert_eq!(full_consumable_state.consumables.len(), 2);

    let mut left = GameState::new(410);
    let mut right = GameState::new(410);
    for state in [&mut left, &mut right] {
        state.jokers.add(Joker::create_joker(JokerKind::Marble));
        state.jokers.add(Joker::create_joker(JokerKind::RiffRaff));
        state.update_jokers(UpdateEvent::BlindSelected { is_boss: false });
    }
    let left_stone_cards = left
        .deck
        .cards()
        .iter()
        .filter(|card| card.enhancement() == Enhancement::Stone)
        .map(|card| (card.rank(), card.suit()))
        .collect::<Vec<_>>();
    let right_stone_cards = right
        .deck
        .cards()
        .iter()
        .filter(|card| card.enhancement() == Enhancement::Stone)
        .map(|card| (card.rank(), card.suit()))
        .collect::<Vec<_>>();
    assert_eq!(left_stone_cards, right_stone_cards);
    assert_eq!(
        left.jokers
            .as_slice()
            .iter()
            .map(Joker::kind)
            .collect::<Vec<_>>(),
        right
            .jokers
            .as_slice()
            .iter()
            .map(Joker::kind)
            .collect::<Vec<_>>(),
    );
}

#[test]
fn generated_jokers_use_the_last_open_slot_and_invisible_needs_a_copy_source() {
    let mut nearly_full = GameState::new(412);
    for _ in 0..3 {
        nearly_full
            .jokers
            .add(Joker::create_joker(JokerKind::Joker));
    }
    nearly_full
        .jokers
        .add(Joker::create_joker(JokerKind::RiffRaff));
    nearly_full.update_jokers(UpdateEvent::BlindSelected { is_boss: false });
    assert_eq!(nearly_full.jokers.as_slice().len(), 5);

    let mut invisible_alone = GameState::new(413);
    invisible_alone
        .jokers
        .add(Joker::create_joker(JokerKind::Invisible));
    invisible_alone.update_jokers(UpdateEvent::RoundCompleted { held_cards: &[] });
    invisible_alone.update_jokers(UpdateEvent::RoundCompleted { held_cards: &[] });
    invisible_alone.sell_joker(0).unwrap();
    assert!(invisible_alone.jokers.as_slice().is_empty());
}

#[test]
fn audit_followup_dna_copies_remain_in_actual_hand() {
    let mut state = GameState::new(820);
    state.begin_blind_as(BlindKind::Small);
    state.jokers.add(Joker::create_joker(JokerKind::Blueprint));
    state.jokers.add(Joker::create_joker(JokerKind::DNA));
    let mut hand = Hand::new();
    hand.add_card(card(
        Rank::Two,
        Suit::Clubs,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));
    for _ in 0..7 {
        hand.add_card(card(
            Rank::Three,
            Suit::Clubs,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ));
    }
    let initial_total =
        state.deck.cards().len() + state.deck.discarded_cards().len() + hand.cards().len();
    state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!(hand.hand_size(), 9, "8 held - 1 played + 2 DNA copies");
    assert_eq!(
        state.deck.cards().len() + state.deck.discarded_cards().len() + hand.cards().len(),
        initial_total + 2
    );
    assert_eq!(
        hand.cards()
            .iter()
            .filter(|card| card.rank() == Rank::Two)
            .count(),
        2
    );
}

#[test]
fn rightmost_dagger_does_not_consume_newly_generated_jokers() {
    let mut state = GameState::new(821);
    state.jokers.add(Joker::create_joker(JokerKind::RiffRaff));
    state
        .jokers
        .add(Joker::create_joker(JokerKind::CeremonialDagger));
    state.begin_blind_as(BlindKind::Small);
    assert_eq!(
        state.jokers.as_slice().len(),
        4,
        "Riff generates two; Dagger had no neighbor at blind selection"
    );
}

#[test]
fn audit_followup_blueprint_does_not_copy_golden_round_payout() {
    let mut state = GameState::new(822);
    state.begin_blind_as(BlindKind::Small);
    state.jokers.add(Joker::create_joker(JokerKind::Blueprint));
    state.jokers.add(Joker::create_joker(JokerKind::Golden));
    let before = state.money;
    state.update_jokers(UpdateEvent::RoundCompleted { held_cards: &[] });
    assert_eq!(state.money, before + 4);
}

#[test]
fn generated_jokers_do_not_join_the_lifecycle_event_that_created_them() {
    let mut generated_riff_raff = false;
    for seed in 0..128 {
        let mut state = GameState::new(seed);
        state.jokers.add(Joker::create_joker(JokerKind::RiffRaff));
        state.jokers.add(Joker::create_joker(JokerKind::Joker));
        state.begin_blind_as(BlindKind::Small);
        assert_eq!(state.jokers.as_slice().len(), 4);
        for joker in &state.jokers.as_slice()[2..] {
            generated_riff_raff |= joker.kind() == JokerKind::RiffRaff;
        }
    }
    assert!(generated_riff_raff);
}

#[test]
fn lifecycle_copy_generation_observes_capacity_after_each_activation() {
    let mut state = GameState::new(823);
    state.jokers.add(Joker::create_joker(JokerKind::Blueprint));
    state.jokers.add(Joker::create_joker(JokerKind::RiffRaff));
    state.begin_blind_as(BlindKind::Small);
    assert_eq!(state.jokers.as_slice().len(), 5);
    assert_eq!(state.jokers.as_slice()[0].kind(), JokerKind::Blueprint);
    assert_eq!(state.jokers.as_slice()[1].kind(), JokerKind::RiffRaff);
}

#[test]
fn copy_chains_and_brainstorm_reject_incompatible_payouts_and_can_retarget_scoring() {
    for kinds in [
        vec![
            JokerKind::Blueprint,
            JokerKind::Blueprint,
            JokerKind::Golden,
        ],
        vec![JokerKind::Golden, JokerKind::Brainstorm],
    ] {
        let mut state = GameState::new(824);
        state.money = 0;
        state.begin_blind_as(BlindKind::Small);
        for kind in kinds {
            state.jokers.add(Joker::create_joker(kind));
        }
        state.update_jokers(UpdateEvent::RoundCompleted { held_cards: &[] });
        assert_eq!(state.money, 4);
        let golden_index = state
            .jokers
            .as_slice()
            .iter()
            .position(|joker| joker.kind() == JokerKind::Golden)
            .unwrap();
        *state.jokers.get_mut(golden_index).unwrap() = Joker::create_joker(JokerKind::Joker);
        let mut hand = Hand::new();
        hand.add_card(card(
            Rank::Two,
            Suit::Clubs,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ));
        let result = state.play_hand(&mut hand, vec![0]).unwrap();
        assert_eq!(
            result.mult,
            1.0 + 4.0 * state.jokers.as_slice().len() as f32
        );
        assert_eq!(state.money, 4);
    }
}

#[test]
fn sale_effect_belongs_to_the_sold_source_including_compatible_copies() {
    for sold_index in [0, 1] {
        let mut state = GameState::new(825);
        state.jokers.add(Joker::create_joker(JokerKind::Blueprint));
        state.jokers.add(Joker::create_joker(JokerKind::DietCola));
        state.sell_joker(sold_index).unwrap();
        assert_eq!(
            state
                .tags
                .held()
                .iter()
                .filter(|&&tag| tag == SkipTag::Double)
                .count(),
            1
        );
        assert_eq!(state.jokers.as_slice().len(), 1);
    }
}

#[test]
fn independent_midas_does_not_change_non_scoring_face() {
    let mut state = GameState::new(801);
    state.begin_blind_as(BlindKind::Small);
    state.jokers.add(Joker::create_joker(JokerKind::MidasMask));
    let mut hand = Hand::new();
    for (rank, suit) in [
        (Rank::Two, Suit::Clubs),
        (Rank::Two, Suit::Diamonds),
        (Rank::King, Suit::Hearts),
    ] {
        hand.add_card(card(
            rank,
            suit,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ));
    }
    state.play_hand(&mut hand, vec![0, 1, 2]).unwrap();
    assert_eq!(
        state.deck.discarded_cards()[2].enhancement(),
        Enhancement::None
    );
}

#[test]
fn independent_destroyed_riff_raff_cannot_generate() {
    let mut state = GameState::new(802);
    state
        .jokers
        .add(Joker::create_joker(JokerKind::CeremonialDagger));
    state.jokers.add(Joker::create_joker(JokerKind::RiffRaff));
    state.begin_blind_as(BlindKind::Small);
    assert_eq!(state.jokers.as_slice().len(), 1);
}

#[test]
fn independent_dagger_removes_immediate_neighbor() {
    let mut state = GameState::new(803);
    for kind in [
        JokerKind::CeremonialDagger,
        JokerKind::Egg,
        JokerKind::Joker,
    ] {
        state.jokers.add(Joker::create_joker(kind));
    }
    state.begin_blind_as(BlindKind::Small);
    assert_eq!(
        state
            .jokers
            .as_slice()
            .iter()
            .map(Joker::kind)
            .collect::<Vec<_>>(),
        vec![JokerKind::CeremonialDagger, JokerKind::Joker]
    );
}

#[test]
fn independent_copy_does_not_double_hologram_growth() {
    let mut state = GameState::new(804);
    state.begin_blind_as(BlindKind::Small);
    for kind in [JokerKind::Blueprint, JokerKind::Hologram, JokerKind::DNA] {
        state.jokers.add(Joker::create_joker(kind));
    }
    let mut hand = Hand::new();
    hand.add_card(card(
        Rank::Two,
        Suit::Clubs,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));
    let result = state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!(result.mult, 1.5625);
}

#[test]
fn independent_debuffed_copy_does_not_activate_lifecycle() {
    let mut state = GameState::new(805);
    state.begin_blind_as(BlindKind::Small);
    state.jokers.add(Joker::create_joker(JokerKind::Blueprint));
    state.jokers.add(Joker::create_joker(JokerKind::Burglar));
    state.jokers.as_mut_slice()[0].set_debuffed(true);
    let before = state.blind.hands_remaining;
    state.update_jokers(UpdateEvent::BlindSelected { is_boss: false });
    assert_eq!(state.blind.hands_remaining, before + 3);
}

#[test]
fn independent_delayed_gratification_does_not_pay_before_round_end() {
    let mut state = GameState::new(806);
    state.begin_blind_as(BlindKind::Small);
    state
        .jokers
        .add(Joker::create_joker(JokerKind::DelayedGratification));
    let before = state.money;
    let mut hand = Hand::new();
    hand.add_card(card(
        Rank::Two,
        Suit::Clubs,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));
    state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!(state.money, before);
}

#[test]
fn independent_dna_copy_is_held_for_current_score() {
    let mut state = GameState::new(807);
    state.begin_blind_as(BlindKind::Small);
    state.jokers.add(Joker::create_joker(JokerKind::DNA));
    state.jokers.add(Joker::create_joker(JokerKind::Baron));
    let mut hand = Hand::new();
    hand.add_card(card(
        Rank::King,
        Suit::Clubs,
        Enhancement::Steel,
        Edition::None,
        Seal::None,
    ));
    for _ in 0..7 {
        hand.add_card(card(
            Rank::Two,
            Suit::Clubs,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ));
    }
    let result = state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!(result.mult, 2.25);
}

#[test]
fn independent_smiley_photograph_order_control() {
    for (kinds, expected_mult) in [
        ([JokerKind::SmileyFace, JokerKind::Photograph], 12.0),
        ([JokerKind::Photograph, JokerKind::SmileyFace], 7.0),
    ] {
        let mut state = GameState::new(808);
        state.begin_blind_as(BlindKind::Small);
        for kind in kinds {
            state.jokers.add(Joker::create_joker(kind));
        }
        let mut hand = Hand::new();
        hand.add_card(card(
            Rank::King,
            Suit::Clubs,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ));
        let result = state.play_hand(&mut hand, vec![0]).unwrap();
        assert_eq!(result.chips, 15);
        assert_eq!(result.mult, expected_mult);
    }
}

#[test]
fn independent_dna_copies_remain_in_actual_hand() {
    let mut state = GameState::new(820);
    state.begin_blind_as(BlindKind::Small);
    state.jokers.add(Joker::create_joker(JokerKind::Blueprint));
    state.jokers.add(Joker::create_joker(JokerKind::DNA));
    let mut hand = Hand::new();
    hand.add_card(card(
        Rank::Two,
        Suit::Clubs,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));
    for _ in 0..7 {
        hand.add_card(card(
            Rank::Three,
            Suit::Clubs,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ));
    }
    state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!(hand.hand_size(), 9, "8 held - 1 played + 2 DNA copies");
}

#[test]
fn independent_rightmost_dagger_does_not_consume_new_riff_jokers() {
    let mut state = GameState::new(821);
    state.jokers.add(Joker::create_joker(JokerKind::RiffRaff));
    state
        .jokers
        .add(Joker::create_joker(JokerKind::CeremonialDagger));
    state.begin_blind_as(BlindKind::Small);
    // Corrected after checking reports of this exact interaction:
    // https://www.reddit.com/r/balatro/comments/1m08hha
    assert_eq!(
        state.jokers.as_slice().len(),
        4,
        "Riff generates two; rightmost Dagger does not consume a new arrival"
    );
}

#[test]
fn independent_blueprint_does_not_copy_golden_round_payout() {
    let mut state = GameState::new(822);
    state.begin_blind_as(BlindKind::Small);
    state.jokers.add(Joker::create_joker(JokerKind::Blueprint));
    state.jokers.add(Joker::create_joker(JokerKind::Golden));
    let before = state.money;
    state.update_jokers(UpdateEvent::RoundCompleted { held_cards: &[] });
    assert_eq!(state.money, before + 4);
}

#[test]
fn independent_chad_uses_first_scoring_card() {
    let mut state = GameState::new(901);
    state.begin_blind_as(BlindKind::Small);
    state
        .jokers
        .add(Joker::create_joker(JokerKind::HangingChad));
    let mut hand = Hand::new();
    hand.add_card(card(
        Rank::Two,
        Suit::Clubs,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));
    hand.add_card(card(
        Rank::King,
        Suit::Hearts,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));
    let result = state.play_hand(&mut hand, vec![0, 1]).unwrap();
    assert_eq!((result.chips, result.mult, result.score), (35, 1.0, 35.0));
}

#[test]
fn independent_photograph_uses_first_scoring_face() {
    let mut state = GameState::new(902);
    state.begin_blind_as(BlindKind::Small);
    state.jokers.add(Joker::create_joker(JokerKind::Photograph));
    let mut hand = Hand::new();
    for rank in [Rank::Two, Rank::Three, Rank::Four, Rank::Five, Rank::King] {
        hand.add_card(card(
            rank,
            Suit::Hearts,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ));
    }
    let result = state.play_hand(&mut hand, vec![0, 1, 2, 3, 4]).unwrap();
    assert_eq!((result.chips, result.mult, result.score), (59, 8.0, 472.0));
}

#[test]
fn independent_debuffed_joker_does_not_apply_polychrome() {
    let mut state = GameState::new(903);
    state.begin_blind_as(BlindKind::Small);
    let mut joker = Joker::create_joker(JokerKind::Joker);
    joker.set_edition(crate::core::joker_types::JokerEdition::Polychrome);
    joker.set_debuffed(true);
    state.jokers.add(joker);
    let mut hand = Hand::new();
    hand.add_card(card(
        Rank::Two,
        Suit::Clubs,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));
    let result = state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!((result.mult, result.score), (1.0, 7.0));
}

#[test]
fn independent_plant_disables_held_steel_and_baron() {
    let mut state = GameState::new(904);
    state.begin_blind_as(BlindKind::Boss(BossBlinds::Plante));
    state.jokers.add(Joker::create_joker(JokerKind::Baron));
    let mut hand = Hand::new();
    hand.add_card(card(
        Rank::Two,
        Suit::Clubs,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));
    hand.add_card(card(
        Rank::King,
        Suit::Hearts,
        Enhancement::Steel,
        Edition::None,
        Seal::None,
    ));
    let result = state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!((result.mult, result.score), (1.0, 7.0));
}

#[test]
fn independent_dagger_retains_twice_victim_sell_value() {
    let mut state = GameState::new(905);
    let egg = Joker::create_joker(JokerKind::Egg);
    let expected_mult = 1.0 + f32::from(egg.sell_value()) * 2.0;
    state
        .jokers
        .add(Joker::create_joker(JokerKind::CeremonialDagger));
    state.jokers.add(egg);
    state.begin_blind_as(BlindKind::Small);
    let mut hand = Hand::new();
    hand.add_card(card(
        Rank::Two,
        Suit::Clubs,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));
    let result = state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!(result.mult, expected_mult);
}

#[test]
fn independent_golden_ticket_blueprint_pays_eight_control() {
    let mut state = GameState::new(906);
    state.begin_blind_as(BlindKind::Small);
    state.jokers.add(Joker::create_joker(JokerKind::Blueprint));
    state
        .jokers
        .add(Joker::create_joker(JokerKind::GoldenTicket));
    let before = state.money;
    let mut hand = Hand::new();
    hand.add_card(card(
        Rank::Two,
        Suit::Clubs,
        Enhancement::Gold,
        Edition::None,
        Seal::None,
    ));
    state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!(state.money, before + 8);
}

#[test]
fn independent_held_steel_and_baron_without_debuff_control() {
    let mut state = GameState::new(907);
    state.begin_blind_as(BlindKind::Small);
    state.jokers.add(Joker::create_joker(JokerKind::Baron));
    let mut hand = Hand::new();
    hand.add_card(card(
        Rank::Two,
        Suit::Clubs,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));
    hand.add_card(card(
        Rank::King,
        Suit::Hearts,
        Enhancement::Steel,
        Edition::None,
        Seal::None,
    ));
    let result = state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!(result.mult, 2.25);
}

#[test]
fn independent_baron_does_not_boost_played_king() {
    let mut state = GameState::new(908);
    state.begin_blind_as(BlindKind::Small);
    state.jokers.add(Joker::create_joker(JokerKind::Baron));
    let mut hand = Hand::new();
    hand.add_card(card(
        Rank::King,
        Suit::Hearts,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));
    let result = state.play_hand(&mut hand, vec![0]).unwrap();
    assert_eq!((result.mult, result.score), (1.0, 15.0));
}

#[test]
fn joker_scoring_editions_follow_owned_joker_debuffs() {
    use crate::core::joker_types::JokerEdition;
    for (edition, active_chips, active_mult) in [
        (JokerEdition::Foil, 57, 5.0),
        (JokerEdition::Holographic, 7, 15.0),
        (JokerEdition::Polychrome, 7, 7.5),
    ] {
        for debuffed in [false, true] {
            let mut state = GameState::new(909);
            state.begin_blind_as(BlindKind::Small);
            let mut joker = Joker::create_joker(JokerKind::Joker);
            joker.set_edition(edition);
            joker.set_debuffed(debuffed);
            state.jokers.add(joker);
            let mut hand = Hand::new();
            hand.add_card(card(
                Rank::Two,
                Suit::Clubs,
                Enhancement::None,
                Edition::None,
                Seal::None,
            ));
            let result = state.play_hand(&mut hand, vec![0]).unwrap();
            let expected = if debuffed {
                (7, 1.0)
            } else {
                (active_chips, active_mult)
            };
            assert_eq!(
                (result.chips, result.mult),
                expected,
                "{edition:?}, debuffed={debuffed}"
            );
        }
    }
}

#[test]
fn held_steel_and_baron_respect_rank_debuffs_and_mime_repetitions() {
    for rank in [Rank::Queen, Rank::King] {
        for plant in [false, true] {
            let mut state = GameState::new(910);
            state.begin_blind_as(if plant {
                BlindKind::Boss(BossBlinds::Plante)
            } else {
                BlindKind::Small
            });
            state.jokers.add(Joker::create_joker(JokerKind::Baron));
            state.jokers.add(Joker::create_joker(JokerKind::Mime));
            let mut hand = Hand::new();
            hand.add_card(card(
                Rank::Two,
                Suit::Clubs,
                Enhancement::None,
                Edition::None,
                Seal::None,
            ));
            hand.add_card(card(
                rank,
                Suit::Hearts,
                Enhancement::Steel,
                Edition::None,
                Seal::None,
            ));
            let result = state.play_hand(&mut hand, vec![0]).unwrap();
            let expected = if plant {
                1.0
            } else if rank == Rank::King {
                5.0625
            } else {
                2.25
            };
            assert_eq!(result.mult, expected, "{rank:?}, plant={plant}");
        }
    }
}

#[test]
fn chad_repeats_only_the_first_of_multiple_scoring_cards() {
    let mut state = GameState::new(911);
    state.begin_blind_as(BlindKind::Small);
    state
        .jokers
        .add(Joker::create_joker(JokerKind::HangingChad));
    let mut hand = Hand::new();
    for suit in [Suit::Hearts, Suit::Spades] {
        hand.add_card(card(
            Rank::King,
            suit,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ));
    }
    let result = state.play_hand(&mut hand, vec![0, 1]).unwrap();
    assert_eq!((result.chips, result.mult), (50, 2.0));
}

#[test]
fn photograph_ignores_excluded_faces_and_boosts_only_first_scoring_face() {
    for (ranks, suits, expected_mult) in [
        (
            vec![Rank::Jack, Rank::Queen, Rank::King],
            vec![Suit::Clubs, Suit::Hearts, Suit::Spades],
            2.0,
        ),
        (
            vec![Rank::Two, Rank::Three, Rank::Jack, Rank::Queen, Rank::King],
            vec![Suit::Hearts; 5],
            8.0,
        ),
    ] {
        let mut state = GameState::new(912);
        state.begin_blind_as(BlindKind::Small);
        state.jokers.add(Joker::create_joker(JokerKind::Photograph));
        let mut hand = Hand::new();
        for (rank, suit) in ranks.iter().zip(suits) {
            hand.add_card(card(
                *rank,
                suit,
                Enhancement::None,
                Edition::None,
                Seal::None,
            ));
        }
        let result = state
            .play_hand(&mut hand, (0..ranks.len()).collect())
            .unwrap();
        assert_eq!(result.mult, expected_mult);
    }
}

#[test]
fn dagger_growth_survives_later_hands_and_is_copied_without_duplicate_growth() {
    let mut state = GameState::new(913);
    state.jokers.add(Joker::create_joker(JokerKind::Blueprint));
    state
        .jokers
        .add(Joker::create_joker(JokerKind::CeremonialDagger));
    let egg = Joker::create_joker(JokerKind::Egg);
    let mut growth = f32::from(egg.sell_value()) * 2.0;
    state.jokers.add(egg);
    state.begin_blind_as(BlindKind::Small);
    for _ in 0..2 {
        let mut hand = Hand::new();
        hand.add_card(card(
            Rank::Two,
            Suit::Clubs,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ));
        assert_eq!(
            state.play_hand(&mut hand, vec![0]).unwrap().mult,
            1.0 + growth * 2.0
        );
    }
    let victim = Joker::create_joker(JokerKind::Joker);
    growth += f32::from(victim.sell_value()) * 2.0;
    state.jokers.add(victim);
    state.begin_blind_as(BlindKind::Big);
    let mut hand = Hand::new();
    hand.add_card(card(
        Rank::Two,
        Suit::Clubs,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));
    assert_eq!(
        state.play_hand(&mut hand, vec![0]).unwrap().mult,
        1.0 + growth * 2.0
    );
    state.begin_blind_as(BlindKind::Big);
    let mut hand = Hand::new();
    hand.add_card(card(
        Rank::Two,
        Suit::Clubs,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));
    assert_eq!(
        state.play_hand(&mut hand, vec![0]).unwrap().mult,
        1.0 + growth * 2.0,
        "no neighbor grants no further growth"
    );
}

#[test]
fn dagger_growth_exceeds_one_byte_and_debuffed_dagger_does_not_consume() {
    for debuffed in [false, true] {
        let mut state = GameState::new(914);
        state.begin_blind_as(BlindKind::Small);
        let mut dagger = Joker::create_joker(JokerKind::CeremonialDagger);
        dagger.set_debuffed(debuffed);
        state.jokers.add(dagger);
        state.jokers.add(Joker::create_joker(JokerKind::Egg));
        for _ in 0..90 {
            state.update_jokers(UpdateEvent::RoundCompleted { held_cards: &[] });
        }
        let growth = f32::from(state.jokers.get(1).unwrap().sell_value()) * 2.0;
        assert!(growth > 255.0);
        state.update_jokers(UpdateEvent::BlindSelected { is_boss: false });
        assert_eq!(state.jokers.as_slice().len(), if debuffed { 2 } else { 1 });
        let mut hand = Hand::new();
        hand.add_card(card(
            Rank::Two,
            Suit::Clubs,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ));
        let result = state.play_hand(&mut hand, vec![0]).unwrap();
        assert_eq!(result.mult, if debuffed { 1.0 } else { 1.0 + growth });
    }
}
