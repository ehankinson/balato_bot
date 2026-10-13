use super::*;

use crate::core::card::Card;
use crate::core::enums::{Consumable, Edition, Enhancement, PokerHand, Rank, Seal, Suit, Tarot};
use crate::core::game::GameState;
use crate::core::joker_types::JokerKind;

fn operation_count(operation: &crate::core::joker::JokerOperation) -> u8 {
    use crate::core::joker::JokerOperation::*;
    match operation {
        CopyPlayedCard { count, .. }
        | GenerateJoker { count, .. }
        | CopyRandomOwnedJoker { count }
        | GenerateConsumable { count, .. }
        | GenerateStoneCard { count }
        | GenerateSealedCard { count }
        | CopyNegativeConsumable { count }
        | AddDoubleTag { count } => *count,
        AddPlayedCardChips { .. } | EnhancePlayedCard { .. } | UpgradeHand { .. } => 1,
    }
}

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
            scoring_card_indices: &[0],
            hand_type: Some(PokerHand::Straight),
            hands_remaining: 3,
        },
        &mut state,
    );
    let pair = jokers.update(
        UpdateEvent::HandCompleted {
            played_cards: std::slice::from_ref(&played),
            scoring_card_indices: &[0],
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
            scoring_card_indices: &[0],
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

    assert_eq!(effect.add_mult, 0);
    assert_eq!(jokers.as_slice().len(), 1);
}

#[test]
fn todo_pays_four_before_bootstraps_scores_in_both_joker_orders() {
    let mut outcomes = Vec::new();
    for todo_first in [true, false] {
        let mut state = GameState::new(809);
        state.begin_blind_as(crate::core::blind::BlindKind::Small);
        state.money = 4;
        let mut todo = Joker::create_joker(JokerKind::ToDoList);
        if let JokerStructure::Normal(JokerData::Econ(data)) = &mut todo.structure {
            data.poker_hand = Some(PokerHand::HighCard);
        }
        let bootstraps = Joker::create_joker(JokerKind::Bootstraps);
        if todo_first {
            state.jokers.add(todo);
            state.jokers.add(bootstraps);
        } else {
            state.jokers.add(bootstraps);
            state.jokers.add(todo);
        }
        let mut hand = crate::core::hand::Hand::new();
        hand.add_card(card(Rank::Two, Enhancement::None));
        let result = state.play_hand(&mut hand, vec![0]).unwrap();
        outcomes.push((state.money, result.mult));
    }
    assert_eq!(outcomes, vec![(8, 3.0), (8, 3.0)]);
}

#[test]
fn lifecycle_updates_use_the_correct_boundaries() {
    let mut certificate = Jokers::new();
    certificate.add(Joker::create_joker(JokerKind::Certificate));
    let mut state = GameState::new(20);

    let before_round =
        certificate.update(UpdateEvent::BlindSelected { is_boss: false }, &mut state);
    let round_start = certificate.update(UpdateEvent::RoundStarted, &mut state);

    assert!(before_round.operations.is_empty());
    assert_eq!(
        round_start.operations[0],
        crate::core::joker::JokerOperation::GenerateSealedCard { count: 1 }
    );

    let mut rocket = Jokers::new();
    rocket.add(Joker::create_joker(JokerKind::Rocket));
    let mut state = GameState::new(21);
    rocket.update(UpdateEvent::BossBlindCompleted, &mut state);
    assert_eq!(
        rocket
            .update(UpdateEvent::RoundCompleted { held_cards: &[] }, &mut state)
            .money,
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
                        scoring_card_indices: &[0],
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
                scoring_card_indices: &[0],
                hand_type: Some(PokerHand::HighCard),
                hands_remaining: 3,
            },
            &mut state,
        );
    }

    jokers.update(
        UpdateEvent::HandCompleted {
            played_cards: std::slice::from_ref(&played),
            scoring_card_indices: &[0],
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
                    scoring_card_indices: &[0],
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
    assert!(state.blind.boss_ability_disabled);

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
    assert!(state.blind.prevent_death);
    assert!(bones.as_slice().is_empty());
}

#[test]
fn perkeo_updates_at_shop_close_and_invisible_waits_two_rounds() {
    let mut perkeo = Jokers::new();
    perkeo.add(Joker::create_joker(JokerKind::Perkeo));
    let mut state = GameState::new(24);
    let effect = perkeo.update(UpdateEvent::ShopClosed, &mut state);
    assert_eq!(
        effect.operations[0],
        crate::core::joker::JokerOperation::CopyNegativeConsumable { count: 1 }
    );

    let mut invisible = Jokers::new();
    invisible.add(Joker::create_joker(JokerKind::Invisible));
    invisible.update(UpdateEvent::RoundCompleted { held_cards: &[] }, &mut state);
    assert!(invisible.sell(0, &mut state).operations.is_empty());

    let mut invisible = Jokers::new();
    invisible.add(Joker::create_joker(JokerKind::Invisible));
    invisible.update(UpdateEvent::RoundCompleted { held_cards: &[] }, &mut state);
    invisible.update(UpdateEvent::RoundCompleted { held_cards: &[] }, &mut state);
    assert_eq!(
        invisible.sell(0, &mut state).operations[0],
        crate::core::joker::JokerOperation::CopyRandomOwnedJoker { count: 1 }
    );
}

#[test]
fn blueprint_and_brainstorm_delegate_economy_state_retrigger_and_generation_effects() {
    let gold = card(Rank::Ace, Enhancement::Gold);
    let mut economy = Jokers::new();
    economy.add(Joker::create_joker(JokerKind::Blueprint));
    economy.add(Joker::create_joker(JokerKind::GoldenTicket));
    let mut state = GameState::new(32);
    state.money = 0;
    economy.trigger(
        TriggerEvent::PlayedCard {
            card: &gold,
            played_cards: std::slice::from_ref(&gold),
            held_cards: &[],
            card_index: 0,
            scoring_card_indices: &[0],
            hand_type: Some(PokerHand::HighCard),
            hands_remaining: 4,
        },
        &mut state,
    );
    assert_eq!(state.money, 8);

    let mut game_state_copy = Jokers::new();
    game_state_copy.add(Joker::create_joker(JokerKind::Blueprint));
    game_state_copy.add(Joker::create_joker(JokerKind::Burglar));
    let mut state = GameState::new(34);
    state.money = 0;
    let effect = game_state_copy.update(UpdateEvent::BlindSelected { is_boss: false }, &mut state);
    assert_eq!(effect.hands, 6);
    assert_eq!(effect.discards, -4);
    assert_eq!(state.blind.hands_remaining, 10);
    assert_eq!(state.blind.discards_remaining, 0);

    let held = card(Rank::Queen, Enhancement::None);
    let mut retriggers = Jokers::new();
    retriggers.add(Joker::create_joker(JokerKind::Blueprint));
    retriggers.add(Joker::create_joker(JokerKind::Mime));
    let mut state = GameState::new(35);
    state.money = 0;
    let effect = retriggers.trigger(
        TriggerEvent::HeldCard {
            card: &held,
            held_cards: std::slice::from_ref(&held),
            card_index: 0,
        },
        &mut state,
    );
    assert_eq!(effect.retriggers, 2);
    assert_eq!(retriggers.mime_count(), 2);

    let mut generated = Jokers::new();
    generated.add(Joker::create_joker(JokerKind::Blueprint));
    generated.add(Joker::create_joker(JokerKind::Perkeo));
    let mut state = GameState::new(36);
    state.money = 0;
    state.consumable_capacity = 4;
    state
        .add_consumable(Consumable::Tarot(Tarot::Hermit))
        .unwrap();
    state.jokers = generated;
    let effect = state.update_jokers(UpdateEvent::ShopClosed);
    assert_eq!(effect.operations.iter().map(operation_count).sum::<u8>(), 2);
    assert_eq!(state.consumables.len(), 3);
    assert_eq!(
        state
            .consumables
            .iter()
            .filter(|item| item.negative)
            .count(),
        2
    );
}

#[test]
fn gift_card_increases_owned_joker_sell_values_at_round_end() {
    let mut jokers = Jokers::new();
    jokers.add(Joker::create_joker(JokerKind::GiftCard));
    jokers.add(Joker::create_joker(JokerKind::Joker));
    let mut state = GameState::new(25);

    let effect = jokers.update(UpdateEvent::RoundCompleted { held_cards: &[] }, &mut state);

    assert_eq!(effect.sell_value_bonus, 1);
    assert_eq!(jokers.jokers[1].sell_value(), 2);
    assert_eq!(state.counters.sell_value_bonus, 1);
}

#[test]
fn generation_updates_emit_the_expected_kind_and_amount() {
    let mut marble = Jokers::new();
    marble.add(Joker::create_joker(JokerKind::Marble));
    let mut state = GameState::new(26);
    let effect = marble.update(UpdateEvent::BlindSelected { is_boss: false }, &mut state);
    assert_eq!(
        effect.operations[0],
        crate::core::joker::JokerOperation::GenerateStoneCard { count: 1 }
    );

    let mut riff_raff = Jokers::new();
    riff_raff.add(Joker::create_joker(JokerKind::RiffRaff));
    let effect = riff_raff.update(UpdateEvent::BlindSelected { is_boss: false }, &mut state);
    assert_eq!(
        effect.operations[0],
        crate::core::joker::JokerOperation::GenerateJoker {
            rarity: crate::core::joker_types::JokerRarity::Common,
            count: 2,
        }
    );

    let mut vagabond = Jokers::new();
    vagabond.add(Joker::create_joker(JokerKind::Vagabond));
    state.money = 4;
    let played = card(Rank::Seven, Enhancement::None);
    let effect = vagabond.trigger(
        TriggerEvent::AfterHand {
            played_cards: std::slice::from_ref(&played),
            scoring_card_indices: &[0],
            held_cards: &[],
            hand_type: Some(PokerHand::HighCard),
            hands_remaining: 3,
            discards_remaining: 4,
            joker_count: 1,
        },
        &mut state,
    );
    assert_eq!(
        effect.operations[0],
        crate::core::joker::JokerOperation::GenerateConsumable {
            kind: crate::core::joker::GeneratedConsumable::Tarot,
            count: 1,
        }
    );
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
        UpdateEvent::CardsDiscarded { cards: &discarded },
        &mut state,
    );
    assert_eq!(effect.x_mult, 1.5);
    hit_the_road.update(UpdateEvent::RoundStarted, &mut state);
    let effect = hit_the_road.update(UpdateEvent::CardsDiscarded { cards: &[] }, &mut state);
    assert_eq!(effect.x_mult, 1.0);

    let mut ramen = Jokers::new();
    ramen.add(Joker::create_joker(JokerKind::Ramen));
    let effect = ramen.update(
        UpdateEvent::CardsDiscarded { cards: &discarded },
        &mut state,
    );
    assert_eq!(effect.x_mult, 1.98);
    let many_cards: Vec<Card> = (0..200)
        .map(|_| card(Rank::Seven, Enhancement::None))
        .collect();
    let effect = ramen.update(
        UpdateEvent::CardsDiscarded { cards: &many_cards },
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
                    scoring_card_indices: &(0..(&four).len()).collect::<Vec<_>>(),
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
                    scoring_card_indices: &(0..(three).len()).collect::<Vec<_>>(),
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
            scoring_card_indices: &[0],
            hand_type: Some(PokerHand::HighCard),
            hands_remaining: 3,
        },
        &mut state,
    );
    assert_eq!(first.add_mult, 1);

    let reset = jokers.update(
        UpdateEvent::HandCompleted {
            played_cards: std::slice::from_ref(&face),
            scoring_card_indices: &[0],
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
        popcorn.update(UpdateEvent::RoundCompleted { held_cards: &[] }, &mut state);
    }
    let effect = popcorn.update(UpdateEvent::RoundCompleted { held_cards: &[] }, &mut state);
    assert_eq!(effect.add_mult, 0);

    let mut ice_cream = Jokers::new();
    ice_cream.add(Joker::create_joker(JokerKind::IceCream));
    let effect = ice_cream.update(
        UpdateEvent::HandCompleted {
            played_cards: std::slice::from_ref(&played),
            scoring_card_indices: &[0],
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
    assert_eq!(
        constellation.update(UpdateEvent::TarotCardUsed, &mut state),
        JokerEffect::default()
    );

    let mut fortune_teller = Jokers::new();
    fortune_teller.add(Joker::create_joker(JokerKind::FortuneTeller));
    let effect = fortune_teller.update(UpdateEvent::TarotCardUsed, &mut state);
    assert_eq!(effect.add_mult, 1);
}

#[test]
fn shop_and_pack_updates_use_only_their_matching_event() {
    let mut red_card = Jokers::new();
    red_card.add(Joker::create_joker(JokerKind::RedCard));
    let mut state = GameState::new(30);
    assert_eq!(
        red_card
            .update(UpdateEvent::BoosterPackSkipped, &mut state)
            .add_mult,
        3
    );
    assert_eq!(
        red_card
            .update(UpdateEvent::BoosterPackOpened, &mut state)
            .add_mult,
        0
    );

    let mut hologram = Jokers::new();
    hologram.add(Joker::create_joker(JokerKind::Hologram));
    assert_eq!(
        hologram
            .update(UpdateEvent::DeckChanged { cards_added: 0 }, &mut state)
            .x_mult,
        1.0
    );
    assert_eq!(
        hologram
            .update(UpdateEvent::DeckChanged { cards_added: 1 }, &mut state)
            .x_mult,
        1.25
    );

    let mut lucky_cat = Jokers::new();
    lucky_cat.add(Joker::create_joker(JokerKind::LuckyCat));
    assert_eq!(
        lucky_cat
            .update(UpdateEvent::LuckyCardSucceeded, &mut state)
            .x_mult,
        1.25
    );
    assert_eq!(
        lucky_cat
            .update(UpdateEvent::TarotCardUsed, &mut state)
            .x_mult,
        1.0
    );

    let mut flash_card = Jokers::new();
    flash_card.add(Joker::create_joker(JokerKind::FlashCard));
    assert_eq!(
        flash_card
            .update(UpdateEvent::ShopRerolled, &mut state)
            .add_mult,
        2
    );
    assert_eq!(
        flash_card
            .update(UpdateEvent::ShopClosed, &mut state)
            .add_mult,
        0
    );
}

#[test]
fn round_lifecycle_updates_apply_their_boundaries_in_place() {
    let mut turtle_bean = Jokers::new();
    turtle_bean.add(Joker::create_joker(JokerKind::TurtleBean));
    let mut state = GameState::new(31);
    assert_eq!(
        turtle_bean
            .update(UpdateEvent::RoundCompleted { held_cards: &[] }, &mut state)
            .hand_size,
        0
    );
    // Bean's remaining bonus belongs to its owned state, not a permanent
    // adjustment to the deck's base hand size.
    assert_eq!(state.deck.hand_size(), 8);
    assert_eq!(turtle_bean.hand_size_modifier(), 4);
    assert_eq!(
        turtle_bean
            .update(UpdateEvent::BlindSelected { is_boss: false }, &mut state)
            .hand_size,
        0
    );

    let mut campfire = Jokers::new();
    campfire.add(Joker::create_joker(JokerKind::Campfire));
    let played = card(Rank::Seven, Enhancement::None);
    assert_eq!(
        campfire
            .update(UpdateEvent::AfterCardSold, &mut state)
            .x_mult,
        1.25
    );
    assert_eq!(
        campfire
            .trigger(
                TriggerEvent::AfterHand {
                    played_cards: std::slice::from_ref(&played),
                    scoring_card_indices: &[0],
                    held_cards: &[],
                    hand_type: Some(PokerHand::HighCard),
                    hands_remaining: 3,
                    discards_remaining: 4,
                    joker_count: 1,
                },
                &mut state,
            )
            .x_mult,
        1.25
    );
    campfire.update(UpdateEvent::BossBlindCompleted, &mut state);
    assert_eq!(
        campfire
            .trigger(
                TriggerEvent::AfterHand {
                    played_cards: std::slice::from_ref(&played),
                    scoring_card_indices: &[0],
                    held_cards: &[],
                    hand_type: Some(PokerHand::HighCard),
                    hands_remaining: 3,
                    discards_remaining: 4,
                    joker_count: 1,
                },
                &mut state,
            )
            .x_mult,
        1.0
    );

    let mut popcorn = Jokers::new();
    popcorn.add(Joker::create_joker(JokerKind::Popcorn));
    assert_eq!(
        popcorn
            .update(UpdateEvent::RoundCompleted { held_cards: &[] }, &mut state)
            .add_mult,
        16
    );
    assert_eq!(
        popcorn
            .update(UpdateEvent::BlindSelected { is_boss: false }, &mut state)
            .add_mult,
        0
    );
}

#[test]
fn obelisk_and_spare_trousers_use_hand_boundaries() {
    let mut obelisk = Jokers::new();
    obelisk.add(Joker::create_joker(JokerKind::Obelisk));
    let mut state = GameState::new(32);
    state.counters.poker_hands_played[PokerHand::Pair as usize - 1] = 1;
    let played = card(Rank::Seven, Enhancement::None);
    assert_eq!(
        obelisk
            .update(
                UpdateEvent::HandCompleted {
                    played_cards: std::slice::from_ref(&played),
                    scoring_card_indices: &[0],
                    hand_type: Some(PokerHand::Flush),
                    hands_remaining: 3,
                },
                &mut state,
            )
            .x_mult,
        1.2
    );
    assert_eq!(
        obelisk
            .update(
                UpdateEvent::HandCompleted {
                    played_cards: std::slice::from_ref(&played),
                    scoring_card_indices: &[0],
                    hand_type: Some(PokerHand::Pair),
                    hands_remaining: 2,
                },
                &mut state,
            )
            .x_mult,
        1.0
    );

    let mut spare_trousers = Jokers::new();
    spare_trousers.add(Joker::create_joker(JokerKind::SpareTrousers));
    assert_eq!(
        spare_trousers
            .update(
                UpdateEvent::HandCompleted {
                    played_cards: std::slice::from_ref(&played),
                    scoring_card_indices: &[0],
                    hand_type: Some(PokerHand::TwoPair),
                    hands_remaining: 3,
                },
                &mut state,
            )
            .add_mult,
        2
    );
    assert_eq!(
        spare_trousers
            .update(
                UpdateEvent::HandCompleted {
                    played_cards: std::slice::from_ref(&played),
                    scoring_card_indices: &[0],
                    hand_type: Some(PokerHand::Pair),
                    hands_remaining: 2,
                },
                &mut state,
            )
            .add_mult,
        0
    );
}

#[test]
fn castle_tracks_only_cards_matching_its_current_target_suit() {
    let mut castle = Jokers::new();
    castle.add(Joker::create_joker(JokerKind::Castle));
    let mut state = GameState::new(33);
    castle.update(UpdateEvent::RoundStarted, &mut state);
    let target = state
        .blind
        .target_suit
        .expect("Castle did not choose a target suit");
    let other = if target == Suit::Hearts {
        Suit::Spades
    } else {
        Suit::Hearts
    };
    let cards = [
        Card::new(
            Rank::Seven,
            other,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ),
        Card::new(
            Rank::Eight,
            target,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ),
        Card::new(
            Rank::Nine,
            other,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ),
    ];
    assert_eq!(
        castle
            .update(UpdateEvent::CardsDiscarded { cards: &cards }, &mut state)
            .chips,
        3
    );
    assert_eq!(
        castle
            .update(UpdateEvent::CardsDiscarded { cards: &[] }, &mut state)
            .chips,
        3
    );
}

#[test]
fn madness_and_death_updates_respect_boss_and_threshold_edges() {
    let mut madness = Jokers::new();
    madness.add(Joker::create_joker(JokerKind::Madness));
    let mut state = GameState::new(34);
    assert_eq!(
        madness.update(UpdateEvent::BlindSelected { is_boss: true }, &mut state),
        JokerEffect::default()
    );
    let effect = madness.update(UpdateEvent::BlindSelected { is_boss: false }, &mut state);
    assert_eq!(effect.x_mult, 1.5);
    assert!(effect.remove_random_joker);
    assert_eq!(madness.as_slice().len(), 1);

    let mut bones = Jokers::new();
    bones.add(Joker::create_joker(JokerKind::MrBones));
    assert_eq!(
        bones
            .update(
                UpdateEvent::AfterPlayerDeath {
                    chips_scored: 24,
                    required_chips: 100,
                },
                &mut state,
            )
            .prevent_death,
        false
    );
    let effect = bones.update(
        UpdateEvent::AfterPlayerDeath {
            chips_scored: 25,
            required_chips: 100,
        },
        &mut state,
    );
    assert!(effect.prevent_death);
    assert!(bones.as_slice().is_empty());
}

#[test]
fn debuffed_round_end_and_nonmatching_destroy_events_are_inactive() {
    let mut golden = Jokers::new();
    golden.add(Joker::create_joker(JokerKind::Golden));
    golden.jokers[0].debuffed = true;
    let mut state = GameState::new(35);
    assert_eq!(
        golden.update(UpdateEvent::RoundCompleted { held_cards: &[] }, &mut state),
        JokerEffect::default()
    );

    let mut glass = Jokers::new();
    glass.add(Joker::create_joker(JokerKind::Glass));
    let plain = card(Rank::Seven, Enhancement::None);
    assert_eq!(
        glass
            .update(UpdateEvent::CardDestroyed { card: &plain }, &mut state)
            .x_mult,
        1.0
    );
}

#[test]
fn lifecycle_edges_are_safe_for_singletons_floors_and_expiration() {
    let mut dagger = Jokers::new();
    dagger.add(Joker::create_joker(JokerKind::CeremonialDagger));
    let mut state = GameState::new(36);
    let effect = dagger.update(UpdateEvent::BlindSelected { is_boss: false }, &mut state);
    assert_eq!(effect.add_mult, 0);
    assert!(effect.remove_right_joker);
    assert_eq!(dagger.as_slice().len(), 1);

    let mut burglar = Jokers::new();
    burglar.add(Joker::create_joker(JokerKind::Burglar));
    state.blind.discards_remaining = 1;
    let effect = burglar.update(UpdateEvent::BlindSelected { is_boss: false }, &mut state);
    assert_eq!(effect.discards, -1);
    assert_eq!(state.blind.discards_remaining, 0);

    let mut green = Jokers::new();
    green.add(Joker::create_joker(JokerKind::Green));
    assert_eq!(
        green
            .update(UpdateEvent::DiscardActionCompleted, &mut state)
            .add_mult,
        0
    );

    let mut ice_cream = Jokers::new();
    ice_cream.add(Joker::create_joker(JokerKind::IceCream));
    for _ in 0..20 {
        ice_cream.update(
            UpdateEvent::HandCompleted {
                played_cards: &[],
                scoring_card_indices: &[],
                hand_type: Some(PokerHand::HighCard),
                hands_remaining: 1,
            },
            &mut state,
        );
    }
    assert_eq!(
        ice_cream
            .update(
                UpdateEvent::HandCompleted {
                    played_cards: &[],
                    scoring_card_indices: &[],
                    hand_type: Some(PokerHand::HighCard),
                    hands_remaining: 1,
                },
                &mut state
            )
            .chips,
        0
    );
}

#[test]
fn probability_and_removal_lifecycle_events_are_seeded_and_bounded() {
    let mut saw_gros_failure = false;
    let mut saw_gros_survival = false;
    let mut saw_cavendish_failure = false;
    let mut saw_cavendish_survival = false;
    let mut saw_hallucination = false;
    let mut saw_hallucination_failure = false;

    for seed in 0..20_000 {
        let mut gros = Jokers::new();
        gros.add(Joker::create_joker(JokerKind::GrosMichel));
        let mut gros_state = GameState::new(seed);
        if gros
            .update(
                UpdateEvent::RoundCompleted { held_cards: &[] },
                &mut gros_state,
            )
            .remove_self
        {
            saw_gros_failure = true;
        } else {
            saw_gros_survival = true;
        }

        let mut cavendish = Jokers::new();
        cavendish.add(Joker::create_joker(JokerKind::Cavendish));
        let mut cavendish_state = GameState::new(seed);
        if cavendish
            .update(
                UpdateEvent::RoundCompleted { held_cards: &[] },
                &mut cavendish_state,
            )
            .remove_self
        {
            saw_cavendish_failure = true;
        } else {
            saw_cavendish_survival = true;
        }

        let mut hallucination = Jokers::new();
        hallucination.add(Joker::create_joker(JokerKind::Hallucination));
        let mut hallucination_state = GameState::new(seed);
        if hallucination
            .update(UpdateEvent::BoosterPackOpened, &mut hallucination_state)
            .operations
            .iter()
            .map(operation_count)
            .sum::<u8>()
            > 0
        {
            saw_hallucination = true;
        } else {
            saw_hallucination_failure = true;
        }
    }

    assert!(saw_gros_failure && saw_gros_survival);
    assert!(saw_cavendish_failure && saw_cavendish_survival);
    assert!(saw_hallucination && saw_hallucination_failure);

    let mut hallucination = Jokers::new();
    hallucination.add(Joker::create_joker(JokerKind::Hallucination));
    let mut state = GameState::new(37);
    assert_eq!(
        hallucination.update(UpdateEvent::RoundCompleted { held_cards: &[] }, &mut state),
        JokerEffect::default()
    );
}

#[test]
fn target_rotation_and_unrelated_update_events_do_not_cross_trigger() {
    let mut ancient = Jokers::new();
    ancient.add(Joker::create_joker(JokerKind::Ancient));
    let mut state = GameState::new(38);
    ancient.update(UpdateEvent::RoundStarted, &mut state);
    let first_suit = match &ancient.jokers[0].structure {
        JokerStructure::Normal(JokerData::Scoring(data)) => data.suit,
        _ => panic!("Ancient Joker did not contain scoring data"),
    };
    assert!(first_suit.is_some());
    assert_eq!(
        ancient.update(UpdateEvent::PlanetCardUsed, &mut state),
        JokerEffect::default()
    );
    ancient.update(UpdateEvent::RoundCompleted { held_cards: &[] }, &mut state);
    let second_suit = match &ancient.jokers[0].structure {
        JokerStructure::Normal(JokerData::Scoring(data)) => data.suit,
        _ => panic!("Ancient Joker did not contain scoring data"),
    };
    assert!(second_suit.is_some());

    let mut constellation = Jokers::new();
    constellation.add(Joker::create_joker(JokerKind::Constellation));
    assert_eq!(
        constellation.update(UpdateEvent::TarotCardUsed, &mut state),
        JokerEffect::default()
    );
}

#[test]
fn yorick_levels_only_after_reaching_the_discard_threshold() {
    let mut yorick = Jokers::new();
    yorick.add(Joker::create_joker(JokerKind::Yorick));
    let mut state = GameState::new(39);
    let discarded: Vec<Card> = (0..22)
        .map(|_| card(Rank::Seven, Enhancement::None))
        .collect();
    assert_eq!(
        yorick
            .update(
                UpdateEvent::CardsDiscarded { cards: &discarded },
                &mut state,
            )
            .x_mult,
        1.0
    );
    let one = [card(Rank::Seven, Enhancement::None)];
    assert_eq!(
        yorick
            .update(UpdateEvent::CardsDiscarded { cards: &one }, &mut state)
            .x_mult,
        2.0
    );
    assert_eq!(
        yorick
            .update(UpdateEvent::CardsDiscarded { cards: &[] }, &mut state)
            .x_mult,
        1.0
    );
}

#[test]
fn independent_todo_pays_four_before_bootstraps_in_both_orders() {
    let mut outcomes = Vec::new();
    for todo_first in [true, false] {
        let mut state = GameState::new(809);
        state.begin_blind_as(crate::core::blind::BlindKind::Small);
        state.money = 4;
        let mut todo = Joker::create_joker(JokerKind::ToDoList);
        if let JokerStructure::Normal(JokerData::Econ(data)) = &mut todo.structure {
            data.poker_hand = Some(PokerHand::HighCard);
        }
        let bootstraps = Joker::create_joker(JokerKind::Bootstraps);
        if todo_first {
            state.jokers.add(todo);
            state.jokers.add(bootstraps);
        } else {
            state.jokers.add(bootstraps);
            state.jokers.add(todo);
        }
        let mut hand = crate::core::hand::Hand::new();
        hand.add_card(card(Rank::Two, Enhancement::None));
        let result = state.play_hand(&mut hand, vec![0]).unwrap();
        outcomes.push((state.money, result.mult));
    }
    assert_eq!(outcomes, vec![(8, 3.0), (8, 3.0)]);
}
