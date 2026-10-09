use crate::core::blind::ScoringData;
use crate::core::card::Card;
use crate::core::enums::{Enhancement, PokerHand};
use crate::core::game::GameState;
use crate::core::joker::{Jokers, TriggerEvent};

#[derive(Debug, PartialEq)]
pub(crate) struct ScoreResult {
    pub(crate) hand: PokerHand,
    pub(crate) chips: u64,
    pub(crate) mult: f32,
    pub(crate) score: f32,
    pub(crate) on_played_scoring_jokers: Vec<usize>,
    pub(crate) scoring_data: ScoringData,
}

pub(crate) fn calculate_score(
    state: &mut GameState,
    played_cards: &[Card],
    held_cards: &[Card],
    hand_indices: &[usize],
    hand_type: PokerHand,
    scoring_data: ScoringData,
) -> ScoreResult {
    let hand_score = state.hand_score(hand_type);
    let mut jokers = std::mem::replace(&mut state.jokers, Jokers::new());
    let on_played_scoring_jokers = jokers.on_played_scoring_indices().collect::<Vec<_>>();
    let on_held_scoring_jokers = jokers.on_held_scoring_indices().collect::<Vec<_>>();
    let after_hand_scoring_jokers = jokers.after_hand_scoring_indices().collect::<Vec<_>>();
    let played_retrigger_jokers = jokers.played_retrigger_indices().collect::<Vec<_>>();
    let held_trigger_count = 1usize.saturating_add(jokers.mime_count());
    let joker_count = jokers.as_slice().len() as u8;
    let mut chips = hand_score.chips as u64;
    let mut mult = hand_score.mult as f32;

    let before_played_event = TriggerEvent::BeforePlayedCards {
        played_cards,
        held_cards,
    };
    let before_played_jokers = jokers.before_played_indices().collect::<Vec<_>>();
    for joker_index in before_played_jokers {
        let effect = jokers
            .get_mut(joker_index)
            .expect("before-played Joker index must remain valid")
            .trigger(&before_played_event, state, joker_count);
        chips = apply_chip_effect(chips, effect.chips);
        mult += effect.add_mult as f32;
        mult *= effect.x_mult;
        Jokers::apply_effect(&effect, state);
    }

    for (card_index, (hand_index, card)) in hand_indices.iter().zip(played_cards).enumerate() {
        if !scoring_data
            .scoring_played_cards
            .iter()
            .any(|scoring_card| scoring_card.index == *hand_index)
        {
            continue;
        }

        let event = TriggerEvent::PlayedCard {
            card,
            played_cards,
            held_cards,
            card_index,
            hand_type: Some(hand_type),
            hands_remaining: state.blind.hands_remaining,
        };

        let mut trigger_count = 1u8;
        for &joker_index in &played_retrigger_jokers {
            let effect = jokers
                .get_mut(joker_index)
                .expect("retrigger Joker index must remain valid")
                .trigger_retrigger(&event, state);
            trigger_count = trigger_count.saturating_add(effect.retriggers);
        }

        for _ in 0..trigger_count {
            let card_values = card.scoring_values();
            chips = apply_chip_effect(chips, card_values.chips);
            mult += card_values.add_mult as f32;
            mult *= card_values.x_mult;

            for &joker_index in &on_played_scoring_jokers {
                let effect = jokers
                    .get_mut(joker_index)
                    .expect("scoring Joker index must remain valid")
                    .trigger_scoring(&event, state, joker_count);
                chips = apply_chip_effect(chips, effect.chips);
                mult += effect.add_mult as f32;
                mult *= effect.x_mult;
            }
        }
    }

    for (card_index, card) in held_cards.iter().enumerate() {
        if !scoring_data
            .scoring_held_cards
            .iter()
            .any(|scoring_card| scoring_card.index == card_index)
        {
            continue;
        }

        let event = TriggerEvent::HeldCard {
            card,
            held_cards,
            card_index,
        };
        for _ in 0..held_trigger_count {
            if card.enhancement() == Enhancement::Steel {
                mult *= 1.5;
            }

            for &joker_index in &on_held_scoring_jokers {
                let effect = jokers
                    .get_mut(joker_index)
                    .expect("held scoring Joker index must remain valid")
                    .trigger_held_scoring(&event, state);
                chips = apply_chip_effect(chips, effect.chips);
                mult += effect.add_mult as f32;
                mult *= effect.x_mult;
            }
        }
    }

    let after_hand_event = TriggerEvent::AfterHand {
        played_cards,
        held_cards,
        hand_type: Some(hand_type),
        hands_remaining: state.blind.hands_remaining,
        discards_remaining: state.blind.discards_remaining,
        joker_count,
    };
    for &joker_index in &after_hand_scoring_jokers {
        let effect = jokers
            .get_mut(joker_index)
            .expect("after-hand scoring Joker index must remain valid")
            .trigger_after_hand_scoring(&after_hand_event, state, joker_count);
        chips = apply_chip_effect(chips, effect.chips);
        mult += effect.add_mult as f32;
        mult *= effect.x_mult;

        let edition = jokers
            .get(joker_index)
            .expect("after-hand scoring Joker index must remain valid")
            .edition_scoring_effect();
        chips = apply_chip_effect(chips, edition.chips);
        mult += edition.add_mult as f32;
        mult *= edition.x_mult;
    }

    state.jokers = jokers;

    ScoreResult {
        hand: hand_type,
        chips,
        mult,
        score: (chips as f32 * mult).floor(),
        on_played_scoring_jokers,
        scoring_data,
    }
}

fn apply_chip_effect(chips: u64, effect: i32) -> u64 {
    if effect >= 0 {
        chips.saturating_add(effect as u64)
    } else {
        chips.saturating_sub(effect.unsigned_abs() as u64)
    }
}
