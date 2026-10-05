use super::*;

use crate::core::card::Card;
use crate::core::enums::{Edition, Enhancement, PokerHand, Rank, Seal, Suit};
use crate::core::game::GameState;
use crate::core::joker_types::{GenerateType, JokerKind};

fn card(rank: Rank, enhancement: Enhancement) -> Card {
    Card::new(rank, Suit::Spades, enhancement, Edition::None, Seal::None)
}

#[test]
fn runner_updates_after_a_straight_and_ignores_other_hands() {
    let mut jokers = Jokers::new();
    jokers.add(Joker::create_joker(JokerKind::Runner));
    let played = card(Rank::Seven, Enhancement::None);
    let mut state = GameState::new(10);

    let straight = jokers.update(
        UpdateEvent::HandCompleted {
            played_cards: std::slice::from_ref(&played),
            hand_type: Some(PokerHand::Straight),
            hands_remaining: 3,
        },
        &mut state,
    );
    let pair = jokers.update(
        UpdateEvent::HandCompleted {
            played_cards: std::slice::from_ref(&played),
            hand_type: Some(PokerHand::Pair),
            hands_remaining: 2,
        },
        &mut state,
    );

    assert_eq!(straight.chips, 15);
    assert_eq!(pair.chips, 0);
}

#[test]
fn glass_and_canio_update_only_for_their_destroyed_card_conditions() {
    let mut jokers = Jokers::new();
    jokers.add(Joker::create_joker(JokerKind::Glass));
    jokers.add(Joker::create_joker(JokerKind::Canio));
    let glass = card(Rank::Queen, Enhancement::Glass);
    let number = card(Rank::Seven, Enhancement::Glass);
    let mut state = GameState::new(11);

    let effect = jokers.update(UpdateEvent::CardDestroyed { card: &glass }, &mut state);
    let no_face = jokers.update(UpdateEvent::CardDestroyed { card: &number }, &mut state);

    assert!(effect.x_mult > 1.0);
    assert_eq!(no_face.x_mult, 2.5);
}

#[test]
fn green_joker_tracks_hands_and_discards_in_place() {
    let mut jokers = Jokers::new();
    jokers.add(Joker::create_joker(JokerKind::Green));
    let played = card(Rank::Seven, Enhancement::None);
    let mut state = GameState::new(12);

    let after_hand = jokers.update(
        UpdateEvent::HandCompleted {
            played_cards: std::slice::from_ref(&played),
            hand_type: Some(PokerHand::Pair),
            hands_remaining: 3,
        },
        &mut state,
    );
    let after_discard = jokers.update(UpdateEvent::DiscardActionCompleted, &mut state);

    assert_eq!(after_hand.add_mult, 1);
    assert_eq!(after_discard.add_mult, 0);
}

#[test]
fn update_events_preserve_left_to_right_collection_order_when_removing() {
    let mut jokers = Jokers::new();
    jokers.add(Joker::create_joker(JokerKind::CeremonialDagger));
    jokers.add(Joker::create_joker(JokerKind::Joker));
    let mut state = GameState::new(13);

    let effect = jokers.update(UpdateEvent::BlindSelected { is_boss: false }, &mut state);

    assert_eq!(effect.add_mult, 2);
    assert_eq!(jokers.as_slice().len(), 1);
}

#[test]
fn lifecycle_updates_use_the_correct_boundaries() {
    let mut certificate = Jokers::new();
    certificate.add(Joker::create_joker(JokerKind::Certificate));
    let mut state = GameState::new(20);

    let before_round =
        certificate.update(UpdateEvent::BlindSelected { is_boss: false }, &mut state);
    let round_start = certificate.update(UpdateEvent::RoundStarted, &mut state);

    assert_eq!(before_round.generated, 0);
    assert_eq!(round_start.generated_type, Some(GenerateType::SealCard));

    let mut rocket = Jokers::new();
    rocket.add(Joker::create_joker(JokerKind::Rocket));
    let mut state = GameState::new(21);
    rocket.update(UpdateEvent::BossBlindCompleted, &mut state);
    assert_eq!(
        rocket.update(UpdateEvent::RoundCompleted, &mut state).money,
        3
    );

    let mut burglar = Jokers::new();
    burglar.add(Joker::create_joker(JokerKind::Burglar));
    let effect = burglar.update(UpdateEvent::BlindSelected { is_boss: false }, &mut state);
    assert_eq!(effect.hands, 3);
    assert_eq!(effect.discards, -4);
}

#[test]
fn seltzer_retrigger_expires_after_ten_hands() {
    let mut jokers = Jokers::new();
    jokers.add(Joker::create_joker(JokerKind::Seltzer));
    let played = card(Rank::Seven, Enhancement::None);
    let mut state = GameState::new(22);

    jokers.update(UpdateEvent::BlindSelected { is_boss: false }, &mut state);
    for _ in 0..9 {
        assert_eq!(
            jokers
                .trigger(
                    TriggerEvent::PlayedCard {
                        card: &played,
                        played_cards: std::slice::from_ref(&played),
                        held_cards: &[],
                        card_index: 0,
                        hand_type: Some(PokerHand::HighCard),
                        hands_remaining: 4,
                    },
                    &mut state
                )
                .retriggers,
            1
        );
        jokers.update(
            UpdateEvent::HandCompleted {
                played_cards: std::slice::from_ref(&played),
                hand_type: Some(PokerHand::HighCard),
                hands_remaining: 3,
            },
            &mut state,
        );
    }

    jokers.update(
        UpdateEvent::HandCompleted {
            played_cards: std::slice::from_ref(&played),
            hand_type: Some(PokerHand::HighCard),
            hands_remaining: 2,
        },
        &mut state,
    );
    assert_eq!(
        jokers
            .trigger(
                TriggerEvent::PlayedCard {
                    card: &played,
                    played_cards: std::slice::from_ref(&played),
                    held_cards: &[],
                    card_index: 0,
                    hand_type: Some(PokerHand::HighCard),
                    hands_remaining: 2,
                },
                &mut state
            )
            .retriggers,
        0
    );
}

#[test]
fn sold_and_death_lifecycle_events_apply_their_one_shot_effects() {
    let mut jokers = Jokers::new();
    jokers.add(Joker::create_joker(JokerKind::Luchador));
    let mut state = GameState::new(23);
    let effect = jokers.sell(0, &mut state);
    assert!(effect.disable_boss_blind);
    assert!(state.boss_blind_disabled);

    let mut bones = Jokers::new();
    bones.add(Joker::create_joker(JokerKind::MrBones));
    let effect = bones.update(
        UpdateEvent::AfterPlayerDeath {
            chips_scored: 25,
            required_chips: 100,
        },
        &mut state,
    );
    assert!(effect.prevent_death);
    assert!(state.prevent_death);
    assert!(bones.as_slice().is_empty());
}

#[test]
fn perkeo_updates_at_shop_close_and_invisible_waits_two_rounds() {
    let mut perkeo = Jokers::new();
    perkeo.add(Joker::create_joker(JokerKind::Perkeo));
    let mut state = GameState::new(24);
    let effect = perkeo.update(UpdateEvent::ShopClosed, &mut state);
    assert_eq!(
        effect.generated_type,
        Some(GenerateType::NegativeConsumable)
    );

    let mut invisible = Jokers::new();
    invisible.add(Joker::create_joker(JokerKind::Invisible));
    invisible.update(UpdateEvent::RoundCompleted, &mut state);
    assert_eq!(invisible.sell(0, &mut state).generated, 0);

    let mut invisible = Jokers::new();
    invisible.add(Joker::create_joker(JokerKind::Invisible));
    invisible.update(UpdateEvent::RoundCompleted, &mut state);
    invisible.update(UpdateEvent::RoundCompleted, &mut state);
    assert_eq!(
        invisible.sell(0, &mut state).generated_type,
        Some(GenerateType::Joker)
    );
}

#[test]
fn gift_card_increases_owned_joker_sell_values_at_round_end() {
    let mut jokers = Jokers::new();
    jokers.add(Joker::create_joker(JokerKind::GiftCard));
    jokers.add(Joker::create_joker(JokerKind::Joker));
    let mut state = GameState::new(25);

    let effect = jokers.update(UpdateEvent::RoundCompleted, &mut state);

    assert_eq!(effect.sell_value_bonus, 1);
    assert_eq!(jokers.jokers[1].sell_value, 2);
    assert_eq!(state.consumable_sell_value_bonus, 1);
}

#[test]
fn generation_updates_emit_the_expected_kind_and_amount() {
    let mut marble = Jokers::new();
    marble.add(Joker::create_joker(JokerKind::Marble));
    let mut state = GameState::new(26);
    let effect = marble.update(UpdateEvent::BlindSelected { is_boss: false }, &mut state);
    assert_eq!(effect.generated, 1);
    assert_eq!(effect.generated_type, Some(GenerateType::StoneCard));

    let mut riff_raff = Jokers::new();
    riff_raff.add(Joker::create_joker(JokerKind::RiffRaff));
    let effect = riff_raff.update(UpdateEvent::BlindSelected { is_boss: false }, &mut state);
    assert_eq!(effect.generated, 2);
    assert_eq!(effect.generated_type, Some(GenerateType::Joker));

    let mut vagabond = Jokers::new();
    vagabond.add(Joker::create_joker(JokerKind::Vagabond));
    state.money = 4;
    let played = card(Rank::Seven, Enhancement::None);
    let effect = vagabond.trigger(
        TriggerEvent::AfterHand {
            played_cards: std::slice::from_ref(&played),
            held_cards: &[],
            hand_type: Some(PokerHand::HighCard),
            hands_remaining: 3,
            discards_remaining: 4,
            joker_count: 1,
        },
        &mut state,
    );
    assert_eq!(effect.generated, 1);
    assert_eq!(effect.generated_type, Some(GenerateType::Tarot));
}

#[test]
fn discard_and_hand_lifecycle_updates_use_exact_boundaries() {
    let mut hit_the_road = Jokers::new();
    hit_the_road.add(Joker::create_joker(JokerKind::HitTheRoad));
    let mut state = GameState::new(27);
    let jack = card(Rank::Jack, Enhancement::None);
    let seven = card(Rank::Seven, Enhancement::None);
    let discarded = [jack, seven];
    let effect = hit_the_road.update(
        UpdateEvent::CardsDiscarded {
            cards: &discarded,
        },
        &mut state,
    );
    assert_eq!(effect.x_mult, 1.5);
    hit_the_road.update(UpdateEvent::RoundStarted, &mut state);
    let effect = hit_the_road.update(
        UpdateEvent::CardsDiscarded { cards: &[] },
        &mut state,
    );
    assert_eq!(effect.x_mult, 1.0);

    let mut ramen = Jokers::new();
    ramen.add(Joker::create_joker(JokerKind::Ramen));
    let effect = ramen.update(
        UpdateEvent::CardsDiscarded {
            cards: &discarded,
        },
        &mut state,
    );
    assert_eq!(effect.x_mult, 1.98);
    let many_cards: Vec<Card> = (0..200)
        .map(|_| card(Rank::Seven, Enhancement::None))
        .collect();
    let effect = ramen.update(
        UpdateEvent::CardsDiscarded {
            cards: &many_cards,
        },
        &mut state,
    );
    assert_eq!(effect.x_mult, 1.0);

    let mut square = Jokers::new();
    square.add(Joker::create_joker(JokerKind::SquareJoker));
    let four = [
        card(Rank::Two, Enhancement::None),
        card(Rank::Three, Enhancement::None),
        card(Rank::Four, Enhancement::None),
        card(Rank::Five, Enhancement::None),
    ];
    let three = &four[..3];
    assert_eq!(
        square
            .update(
                UpdateEvent::HandCompleted {
                    played_cards: &four,
                    hand_type: Some(PokerHand::HighCard),
                    hands_remaining: 3,
                },
                &mut state,
            )
            .chips,
        4
    );
    assert_eq!(
        square
            .update(
                UpdateEvent::HandCompleted {
                    played_cards: three,
                    hand_type: Some(PokerHand::HighCard),
                    hands_remaining: 2,
                },
                &mut state,
            )
            .chips,
        0
    );
}

#[test]
fn ride_the_bus_resets_on_a_face_card() {
    let mut jokers = Jokers::new();
    jokers.add(Joker::create_joker(JokerKind::RideTheBus));
    let mut state = GameState::new(28);
    let number = card(Rank::Seven, Enhancement::None);
    let face = card(Rank::Queen, Enhancement::None);

    let first = jokers.update(
        UpdateEvent::HandCompleted {
            played_cards: std::slice::from_ref(&number),
            hand_type: Some(PokerHand::HighCard),
            hands_remaining: 3,
        },
        &mut state,
    );
    assert_eq!(first.add_mult, 1);

    let reset = jokers.update(
        UpdateEvent::HandCompleted {
            played_cards: std::slice::from_ref(&face),
            hand_type: Some(PokerHand::HighCard),
            hands_remaining: 2,
        },
        &mut state,
    );
    assert_eq!(reset.add_mult, 0);
}

#[test]
fn scoring_updates_apply_thresholds_and_clamps() {
    let played = card(Rank::Seven, Enhancement::None);

    let mut popcorn = Jokers::new();
    popcorn.add(Joker::create_joker(JokerKind::Popcorn));
    let mut state = GameState::new(29);
    for _ in 0..5 {
        popcorn.update(UpdateEvent::RoundCompleted, &mut state);
    }
    let effect = popcorn.update(UpdateEvent::RoundCompleted, &mut state);
    assert_eq!(effect.add_mult, 0);

    let mut ice_cream = Jokers::new();
    ice_cream.add(Joker::create_joker(JokerKind::IceCream));
    let effect = ice_cream.update(
        UpdateEvent::HandCompleted {
            played_cards: std::slice::from_ref(&played),
            hand_type: Some(PokerHand::HighCard),
            hands_remaining: 3,
        },
        &mut state,
    );
    assert_eq!(effect.chips, 95);

    let mut constellation = Jokers::new();
    constellation.add(Joker::create_joker(JokerKind::Constellation));
    let effect = constellation.update(UpdateEvent::PlanetCardUsed, &mut state);
    assert_eq!(effect.x_mult, 1.1);

    let mut fortune_teller = Jokers::new();
    fortune_teller.add(Joker::create_joker(JokerKind::FortuneTeller));
    let effect = fortune_teller.update(UpdateEvent::TarotCardUsed, &mut state);
    assert_eq!(effect.add_mult, 1);
}
