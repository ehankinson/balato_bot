pub(crate) use crate::calculation::execute_hand::ScoreResult;
use crate::calculation::execute_hand::execute_hand as execute_hand_impl;
use crate::calculation::poker::determine_poker_hand;
pub(crate) use crate::core::boss_blind::BlindPlayError;
use crate::core::card::Card;
use crate::core::enums::{BossBlinds, Enhancement, PokerHand, Stakes, Suit};
use crate::core::game::GameState;
use crate::core::hand::Hand;
use crate::core::joker_types::JokerModifiers;

/// A card reference used while scoring. The ID contains the card properties;
/// the source index preserves which occurrence was played when cards share an
/// identical ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScoringCard {
    pub(crate) index: usize,
    pub(crate) id: u16,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct ScoringData {
    pub(crate) scoring_played_cards: Vec<ScoringCard>,
    pub(crate) non_scoring_played_cards: Vec<ScoringCard>,
    pub(crate) scoring_held_cards: Vec<ScoringCard>,
    pub(crate) non_scoring_held_cards: Vec<ScoringCard>,
}

impl ScoringData {
    pub(crate) fn from_played_hand(
        hand: &Hand,
        selected_indices: &[usize],
        modifiers: JokerModifiers,
    ) -> Option<(PokerHand, ScoringData)> {
        if selected_indices
            .iter()
            .enumerate()
            .any(|(position, index)| selected_indices[..position].contains(index))
        {
            return None;
        }
        let selected_cards = selected_indices
            .iter()
            .map(|&index| hand.card(index))
            .collect::<Option<Vec<_>>>()?;
        let ids = selected_cards
            .iter()
            .map(|card| card.id())
            .collect::<Vec<_>>();
        let detection_ids = selected_cards
            .iter()
            .map(|card| {
                if card.is_debuffed() && card.enhancement() == Enhancement::Wild {
                    card.id() & !(0x0f << 5)
                } else {
                    card.id()
                }
            })
            .collect::<Vec<_>>();
        let result = determine_poker_hand(&detection_ids, modifiers)?;
        let scoring_indices = result.scoring_card_indices;

        let mut data = ScoringData::default();
        for (selected_index, (&hand_index, &id)) in
            selected_indices.iter().zip(ids.iter()).enumerate()
        {
            let card = ScoringCard {
                index: hand_index,
                id,
            };
            if modifiers.splash || scoring_indices.contains(&selected_index) {
                data.scoring_played_cards.push(card);
            } else {
                data.non_scoring_played_cards.push(card);
            }
        }

        Some((result.hand, data))
    }

    pub(crate) fn from_card_ids(played_ids: &[u16], held_ids: &[u16]) -> ScoringData {
        ScoringData {
            non_scoring_played_cards: scoring_cards(played_ids),
            non_scoring_held_cards: scoring_cards(held_ids),
            ..ScoringData::default()
        }
    }
}

pub(crate) fn score_played_hand(
    hand: &Hand,
    selected_indices: &[usize],
    modifiers: JokerModifiers,
) -> Option<(PokerHand, ScoringData)> {
    ScoringData::from_played_hand(hand, selected_indices, modifiers)
}

pub(crate) fn execute_hand(
    state: &mut GameState,
    hand: &mut Hand,
    selected_indices: &[usize],
    played_cards_out: &mut Vec<Card>,
) -> Option<ScoreResult> {
    state.blind.validate_play(selected_indices).ok()?;
    if selected_indices.is_empty()
        || selected_indices
            .iter()
            .enumerate()
            .any(|(position, &index)| {
                hand.card(index).is_none() || selected_indices[..position].contains(&index)
            })
    {
        return None;
    }
    let modifiers = state.jokers.modifiers();
    for card in hand.cards_mut() {
        card.set_debuffed(state.blind.card_is_debuffed(card, modifiers));
    }
    let (hand_type, mut scoring_data) = score_played_hand(hand, selected_indices, modifiers)?;

    state.counters.poker_hands_played[hand_type as usize - 1] += 1;
    let hand_debuffed = state
        .blind
        .hand_is_debuffed(selected_indices.len(), hand_type);
    let mut boss_triggered = hand_debuffed || state.blind.flint_active();
    if state.blind.arm_active() {
        boss_triggered |= state.downgrade_hand_score(hand_type);
    }
    if state.blind.ox_active() && state.ox_target_hand == hand_type {
        state.money = 0;
        boss_triggered = true;
    }
    if state.blind.tooth_active() {
        state.money = state.money.saturating_sub(selected_indices.len() as i16);
        state.notify_boss_ability(false);
    }

    let old_forced = state.blind.forced_card_index;
    let mut played_cards =
        hand.select_cards(selected_indices.iter().map(|&index| index as u8).collect());
    state.blind.forced_card_index = old_forced
        .filter(|index| !selected_indices.contains(index))
        .map(|index| {
            index
                - selected_indices
                    .iter()
                    .filter(|&&selected| selected < index)
                    .count()
        });
    for (index, card) in played_cards.iter_mut().enumerate() {
        if card.is_debuffed()
            && scoring_data
                .scoring_played_cards
                .iter()
                .any(|scoring| scoring.index == selected_indices[index])
        {
            boss_triggered = true;
        }
        card.set_face_down(false);
        card.mark_played(state.counters.ante);
    }
    scoring_data.scoring_held_cards = scoring_cards(&card_ids(hand.cards()));
    let result = if hand_debuffed {
        scoring_data
            .non_scoring_played_cards
            .append(&mut scoring_data.scoring_played_cards);
        scoring_data
            .non_scoring_played_cards
            .sort_unstable_by_key(|card| card.index);
        scoring_data
            .non_scoring_held_cards
            .append(&mut scoring_data.scoring_held_cards);
        state.notify_boss_ability(true);
        ScoreResult {
            hand: hand_type,
            chips: 0,
            mult: 0.0,
            score: 0.0,
            scoring_data,
        }
    } else {
        execute_hand_impl(
            state,
            &mut played_cards,
            hand,
            selected_indices,
            hand_type,
            scoring_data,
            boss_triggered,
        )
    };
    state.blind.record_play(hand_type);
    *played_cards_out = played_cards;
    Some(result)
}

fn scoring_cards(ids: &[u16]) -> Vec<ScoringCard> {
    ids.iter()
        .enumerate()
        .map(|(index, &id)| ScoringCard { index, id })
        .collect()
}

pub(crate) fn card_ids(cards: &[Card]) -> Vec<u16> {
    cards.iter().map(Card::id).collect()
}

/// The kind of blind currently being played.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum BlindKind {
    Small,
    Big,
    Boss(BossBlinds),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum BlindSkipError {
    BossBlindCannotBeSkipped,
}

impl BlindKind {
    pub(crate) const fn is_boss(self) -> bool {
        matches!(self, BlindKind::Boss(_))
    }
}

/// Blind configuration and state for the current blind lifecycle.
///
/// The per-blind hand and discard limits persist between blind transitions;
/// the remaining counters are reset from them when a new blind begins.
pub(crate) struct BlindState {
    pub(crate) kind: BlindKind,
    pub(crate) stake: Stakes,
    pub(crate) ante_scaling: u64,
    pub(crate) hands_per_blind: u8,
    pub(crate) discards_per_blind: u8,
    pub(crate) hands_remaining: u8,
    pub(crate) discards_remaining: u8,
    pub(crate) discards_used: u8,
    pub(crate) hands_played: u32,
    pub(crate) chips_scored: u64,
    pub(crate) score_requirement: u64,
    pub(crate) finished: bool,
    pub(crate) boss_ability_disabled: bool,
    pub(crate) prevent_death: bool,
    pub(crate) boss_removed_hands: u8,
    pub(crate) boss_removed_discards: u8,
    pub(crate) current_hand: Option<PokerHand>,
    pub(crate) target_suit: Option<Suit>,
    pub(crate) hand_size_modifier: i8,
    pub(crate) first_action_taken: bool,
    pub(crate) played_hand_types: Vec<PokerHand>,
    pub(crate) forced_card_index: Option<usize>,
    pub(crate) face_down_draws: bool,
    pub(crate) face_down_face_cards: bool,
    pub(crate) serpent_draws: bool,
    pub(crate) crimson_heart_prepped: bool,
    pub(crate) verdant_leaf_active: bool,
    pub(crate) ante: u8,
}

impl BlindState {
    pub(crate) fn new() -> BlindState {
        BlindState {
            kind: BlindKind::Small,
            stake: Stakes::White,
            ante_scaling: 1,
            hands_per_blind: 4,
            discards_per_blind: 3,
            hands_remaining: 4,
            discards_remaining: 3,
            discards_used: 0,
            hands_played: 0,
            chips_scored: 0,
            score_requirement: 0,
            finished: false,
            boss_ability_disabled: false,
            prevent_death: false,
            boss_removed_hands: 0,
            boss_removed_discards: 0,
            current_hand: None,
            target_suit: None,
            hand_size_modifier: 0,
            first_action_taken: false,
            played_hand_types: Vec::new(),
            forced_card_index: None,
            face_down_draws: false,
            face_down_face_cards: false,
            serpent_draws: false,
            crimson_heart_prepped: false,
            verdant_leaf_active: false,
            ante: 1,
        }
    }

    pub(crate) fn begin(&mut self, kind: BlindKind, ante: u8) {
        self.kind = kind;
        self.ante = ante.max(1);
        self.score_requirement = kind
            .score_requirement(self.stake, self.ante, false)
            .saturating_mul(self.ante_scaling);
        self.finished = false;
        self.hands_remaining = self.hands_per_blind;
        self.discards_remaining = self.discards_per_blind;
        self.discards_used = 0;
        self.hands_played = 0;
        self.chips_scored = 0;
        self.boss_ability_disabled = false;
        self.prevent_death = false;
        self.boss_removed_hands = 0;
        self.boss_removed_discards = 0;
        self.current_hand = None;
        self.target_suit = None;
        self.hand_size_modifier = 0;
        self.first_action_taken = false;
        self.played_hand_types.clear();
        self.forced_card_index = None;
        self.face_down_draws = false;
        self.face_down_face_cards = false;
        self.serpent_draws = false;
        self.crimson_heart_prepped = false;
        self.verdant_leaf_active = false;

        self.apply_boss_start_rules();
    }

    pub(crate) fn validate_play(&self, selected_indices: &[usize]) -> Result<(), BlindPlayError> {
        if self.hands_remaining == 0 {
            return Err(BlindPlayError::NoHandsRemaining);
        }
        if selected_indices.len() > 5 {
            return Err(BlindPlayError::TooManyCards);
        }

        self.validate_boss_play(selected_indices)?;
        Ok(())
    }

    pub(crate) fn record_play(&mut self, hand_type: PokerHand) {
        self.current_hand = Some(hand_type);
        self.played_hand_types.push(hand_type);
        self.hands_played = self.hands_played.saturating_add(1);
        self.hands_remaining = self.hands_remaining.saturating_sub(1);
        self.first_action_taken = true;
        self.record_boss_play();
    }

    pub(crate) fn record_discard(&mut self) {
        self.discards_used += 1;
        self.first_action_taken = true;
        self.discards_remaining = self.discards_remaining.saturating_sub(1);
        self.record_boss_discard();
    }

    pub(crate) fn adjust_hands_per_blind(&mut self, amount: i8) {
        self.hands_per_blind = adjust_per_blind_amount(self.hands_per_blind, amount);
    }

    pub(crate) fn adjust_discards_per_blind(&mut self, amount: i8) {
        self.discards_per_blind = adjust_per_blind_amount(self.discards_per_blind, amount);
    }

    pub(crate) fn has_beaten_target(&self) -> bool {
        self.chips_scored >= self.score_requirement
    }
}

fn adjust_per_blind_amount(current: u8, amount: i8) -> u8 {
    if amount >= 0 {
        current.saturating_add(amount as u8)
    } else {
        current.saturating_sub(amount.unsigned_abs())
    }
}
