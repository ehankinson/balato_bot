use crate::core::blind::{BlindKind, ScoringCard, ScoringData};
use crate::core::card::{Card, ScoringValues};
use crate::core::enums::{Enhancement, PokerHand};
use crate::core::game::GameState;
use crate::core::hand::Hand;
use crate::core::joker::{JokerScoringEffect, TriggerEvent};

#[derive(Debug)]
struct ScoreAccumulator {
    chips: u64,
    mult: f32,
}

impl ScoreAccumulator {
    fn apply(&mut self, chips: i32, add_mult: i32, x_mult: f32) {
        self.chips += chips as u64;
        self.mult += add_mult as f32;
        self.mult *= x_mult;
    }

    fn apply_card_values(&mut self, values: ScoringValues) {
        self.apply(values.chips, values.add_mult, values.x_mult);
    }

    fn apply_joker_scoring_effect(&mut self, effect: &JokerScoringEffect) {
        self.apply(effect.chips, effect.add_mult, effect.x_mult);
    }

    fn multiply_mult(&mut self, factor: f32) {
        self.mult *= factor;
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct ScoreResult {
    pub(crate) hand: PokerHand,
    pub(crate) chips: u64,
    pub(crate) mult: f32,
    pub(crate) score: f32,
    pub(crate) scoring_data: ScoringData,
}

pub(crate) fn execute_hand(
    state: &mut GameState,
    played_cards: &mut [Card],
    held_hand: &mut Hand,
    hand_indices: &[usize],
    hand_type: PokerHand,
    mut scoring_data: ScoringData,
    boss_ability_triggered: bool,
) -> ScoreResult {
    let mut hand_score = state.hand_score(hand_type);
    if state.blind.flint_active() {
        hand_score.chips = hand_score.chips.div_ceil(2);
        hand_score.mult = hand_score.mult.div_ceil(2);
        hand_score.mult = hand_score.mult.max(1);
    }
    let joker_count = state.jokers.as_slice().len() as u8;
    let joker_len = usize::from(joker_count);
    let held_trigger_count = 1usize.saturating_add(state.jokers.mime_count());
    let original_held_count = held_hand.cards().len();
    let mut score = ScoreAccumulator {
        chips: hand_score.chips as u64,
        mult: hand_score.mult as f32,
    };
    for joker_index in 0..joker_len {
        let effect = {
            let event = TriggerEvent::BeforePlayedCards {
                played_cards: &played_cards,
                held_cards: held_hand.cards(),
            };
            state.trigger_joker_at(joker_index, &event, joker_count)
        };
        score.apply(effect.chips, effect.add_mult, effect.x_mult);
        state.apply_scoring_joker_effect(&effect, played_cards, held_hand);
    }

    let scoring_card_indices = hand_indices
        .iter()
        .enumerate()
        .filter_map(|(card_index, hand_index)| {
            scoring_data
                .scoring_played_cards
                .iter()
                .any(|scoring_card| scoring_card.index == *hand_index)
                .then_some(card_index)
        })
        .collect::<Vec<_>>();
    for joker_index in 0..joker_len {
        let effect = {
            let event = TriggerEvent::HandPlayed {
                played_cards,
                scoring_card_indices: &scoring_card_indices,
                hand_type,
            };
            state.trigger_joker_at(joker_index, &event, joker_count)
        };
        state.apply_scoring_joker_effect(&effect, played_cards, held_hand);
    }
    for index in original_held_count..held_hand.cards().len() {
        scoring_data.scoring_held_cards.push(ScoringCard {
            index,
            id: held_hand.cards()[index].id(),
        });
    }

    for card_index in 0..played_cards.len() {
        let hand_index = hand_indices[card_index];
        let is_scoring_card = scoring_data
            .scoring_played_cards
            .iter()
            .any(|scoring_card| scoring_card.index == hand_index);

        if !is_scoring_card {
            continue;
        }

        let mut trigger_count = 1u8;
        for joker_index in 0..joker_len {
            let retriggers = {
                let event = TriggerEvent::PlayedCard {
                    card: &played_cards[card_index],
                    played_cards,
                    held_cards: held_hand.cards(),
                    card_index,
                    scoring_card_indices: &scoring_card_indices,
                    hand_type: Some(hand_type),
                    hands_remaining: state.blind.hands_remaining,
                };
                state.trigger_retrigger_joker_at(joker_index, &event)
            };
            trigger_count = trigger_count.saturating_add(retriggers);
        }

        for _ in 0..trigger_count {
            let card_values = played_cards[card_index].scoring_values();
            score.apply_card_values(card_values);

            for joker_index in 0..joker_len {
                if state.jokers.is_played_retrigger(joker_index) {
                    continue;
                }
                let effect = {
                    let event = TriggerEvent::PlayedCard {
                        card: &played_cards[card_index],
                        played_cards,
                        held_cards: held_hand.cards(),
                        card_index,
                        scoring_card_indices: &scoring_card_indices,
                        hand_type: Some(hand_type),
                        hands_remaining: state.blind.hands_remaining,
                    };
                    state.trigger_joker_at(joker_index, &event, joker_count)
                };
                score.apply(effect.chips, effect.add_mult, effect.x_mult);
                state.apply_scoring_joker_effect(&effect, played_cards, held_hand);
            }
        }
    }

    for card_index in 0..held_hand.cards().len() {
        if held_hand.cards()[card_index].is_debuffed() {
            continue;
        }
        if !scoring_data
            .scoring_held_cards
            .iter()
            .any(|scoring_card| scoring_card.index == card_index)
        {
            continue;
        }

        for _ in 0..held_trigger_count {
            if held_hand.cards()[card_index].enhancement() == Enhancement::Steel {
                score.multiply_mult(1.5);
            }

            for joker_index in 0..joker_len {
                if state.jokers.is_retrigger(joker_index) {
                    continue;
                }
                let effect = {
                    let event = TriggerEvent::HeldCard {
                        card: &held_hand.cards()[card_index],
                        held_cards: held_hand.cards(),
                        card_index,
                    };
                    state.trigger_joker_at(joker_index, &event, joker_count)
                };
                score.apply(effect.chips, effect.add_mult, effect.x_mult);
                state.apply_scoring_joker_effect(&effect, played_cards, held_hand);
            }
        }
    }

    for joker_index in 0..joker_len {
        let effect = {
            let event = TriggerEvent::AfterHand {
                played_cards,
                scoring_card_indices: &scoring_card_indices,
                held_cards: held_hand.cards(),
                hand_type: Some(hand_type),
                hands_remaining: state.blind.hands_remaining,
                discards_remaining: state.blind.discards_remaining,
                joker_count,
            };
            state.trigger_joker_at(joker_index, &event, joker_count)
        };
        score.apply(effect.chips, effect.add_mult, effect.x_mult);
        state.apply_scoring_joker_effect(&effect, played_cards, held_hand);

        if boss_ability_triggered && !state.blind.boss_ability_disabled {
            if let BlindKind::Boss(boss) = state.blind.kind {
                let event = TriggerEvent::BossBlindAbilityTriggered {
                    boss,
                    during_hand_scoring: true,
                };
                let effect = state.trigger_joker_at(joker_index, &event, joker_count);
                score.apply(effect.chips, effect.add_mult, effect.x_mult);
                state.apply_scoring_joker_effect(&effect, played_cards, held_hand);
            }
        }

        let edition = state
            .jokers
            .get(joker_index)
            .expect("after-hand scoring Joker index must remain valid")
            .edition_scoring_effect();
        score.apply_joker_scoring_effect(&edition);
    }

    ScoreResult {
        hand: hand_type,
        chips: score.chips,
        mult: score.mult,
        score: (score.chips as f32 * score.mult).floor(),
        scoring_data,
    }
}
