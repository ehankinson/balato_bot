pub(crate) use crate::calculation::calculate_score::ScoreResult;
use crate::calculation::calculate_score::calculate_score as calculate_score_impl;
use crate::calculation::poker::determine_poker_hand;
pub(crate) use crate::core::boss_blind::BlindPlayError;
use crate::core::card::Card;
use crate::core::enums::{BossBlinds, PokerHand, Stakes, Suit};
use crate::core::game::GameState;
use crate::core::hand::Hand;

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
        let result = determine_poker_hand(&ids)?;
        let scoring_indices = result.scoring_card_indices;

        let mut data = ScoringData::default();
        for (selected_index, (&hand_index, &id)) in
            selected_indices.iter().zip(ids.iter()).enumerate()
        {
            let card = ScoringCard {
                index: hand_index,
                id,
            };
            if scoring_indices.contains(&selected_index) {
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
) -> Option<(PokerHand, ScoringData)> {
    ScoringData::from_played_hand(hand, selected_indices)
}

pub(crate) fn calculate_score(
    state: &mut GameState,
    hand: &Hand,
    selected_indices: &[usize],
) -> Option<ScoreResult> {
    let (hand_type, mut scoring_data) = score_played_hand(hand, selected_indices)?;
    state
        .blind
        .validate_play(selected_indices, hand_type)
        .ok()?;

    if state.blind.arm_active() {
        state.downgrade_hand_score(hand_type);
    }
    if state.blind.ox_active() && state.most_played_hand == Some(hand_type) {
        state.money = 0;
    }
    if state.blind.tooth_active() {
        state.money = state.money.saturating_sub(selected_indices.len() as i16);
    }

    let played_cards = selected_indices
        .iter()
        .map(|&index| hand.card(index).cloned())
        .collect::<Option<Vec<_>>>()?;
    let held_cards = hand
        .cards()
        .iter()
        .enumerate()
        .filter(|(index, _)| !selected_indices.contains(index))
        .map(|(_, card)| card.clone())
        .collect::<Vec<_>>();
    let mut played_cards = played_cards;
    for card in &mut played_cards {
        let pillar_debuffed = matches!(state.blind.kind, BlindKind::Boss(BossBlinds::Pillar))
            && state.pillar_card_ids.contains(&card.id());
        card.set_debuffed(state.blind.card_is_debuffed(card) || pillar_debuffed);
    }
    let mut held_cards = held_cards;
    for card in &mut held_cards {
        card.set_debuffed(state.blind.card_is_debuffed(card));
    }
    scoring_data.scoring_held_cards = scoring_cards(&card_ids(&held_cards));
    let result = calculate_score_impl(
        state,
        &played_cards,
        &held_cards,
        selected_indices,
        hand_type,
        scoring_data,
    );
    state
        .pillar_card_ids
        .extend(played_cards.iter().map(Card::id));
    state.blind.record_play(hand_type);
    if matches!(state.blind.kind, BlindKind::Boss(BossBlinds::CrimsonHeart))
        && !state.blind.boss_ability_disabled
    {
        state.jokers.disable_random(&mut state.rng);
    }
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
    pub(crate) hands_per_blind: u8,
    pub(crate) discards_per_blind: u8,
    pub(crate) hands_remaining: u8,
    pub(crate) discards_remaining: u8,
    pub(crate) discards_used: u8,
    pub(crate) hands_played: u32,
    pub(crate) chips_scored: u64,
    pub(crate) score_requirement: u64,
    pub(crate) boss_ability_disabled: bool,
    pub(crate) prevent_death: bool,
    pub(crate) current_hand: Option<PokerHand>,
    pub(crate) target_suit: Option<Suit>,
    pub(crate) hand_size_modifier: i8,
    pub(crate) first_action_taken: bool,
    pub(crate) played_hand_types: Vec<PokerHand>,
    pub(crate) forced_card_index: Option<usize>,
    pub(crate) face_down_draws: bool,
    pub(crate) face_down_face_cards: bool,
    pub(crate) serpent_draws: bool,
    pub(crate) verdant_leaf_active: bool,
    pub(crate) ante: u8,
}

impl BlindState {
    pub(crate) fn new() -> BlindState {
        BlindState {
            kind: BlindKind::Small,
            stake: Stakes::White,
            hands_per_blind: 4,
            discards_per_blind: 4,
            hands_remaining: 4,
            discards_remaining: 4,
            discards_used: 0,
            hands_played: 0,
            chips_scored: 0,
            score_requirement: 0,
            boss_ability_disabled: false,
            prevent_death: false,
            current_hand: None,
            target_suit: None,
            hand_size_modifier: 0,
            first_action_taken: false,
            played_hand_types: Vec::new(),
            forced_card_index: None,
            face_down_draws: false,
            face_down_face_cards: false,
            serpent_draws: false,
            verdant_leaf_active: false,
            ante: 1,
        }
    }

    pub(crate) fn begin(&mut self, kind: BlindKind, ante: u8) {
        self.kind = kind;
        self.ante = ante.max(1);
        self.score_requirement = kind.score_requirement(self.stake, self.ante, false);
        self.hands_remaining = self.hands_per_blind;
        self.discards_remaining = self.discards_per_blind;
        self.discards_used = 0;
        self.hands_played = 0;
        self.chips_scored = 0;
        self.boss_ability_disabled = false;
        self.prevent_death = false;
        self.current_hand = None;
        self.target_suit = None;
        self.hand_size_modifier = 0;
        self.first_action_taken = false;
        self.played_hand_types.clear();
        self.forced_card_index = None;
        self.face_down_draws = false;
        self.face_down_face_cards = false;
        self.serpent_draws = false;
        self.verdant_leaf_active = false;

        self.apply_boss_start_rules();
    }

    pub(crate) fn validate_play(
        &self,
        selected_indices: &[usize],
        hand_type: PokerHand,
    ) -> Result<(), BlindPlayError> {
        if self.hands_remaining == 0 {
            return Err(BlindPlayError::NoHandsRemaining);
        }

        self.validate_boss_play(selected_indices, hand_type)?;
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
        self.first_action_taken = true;
        self.record_boss_discard();
    }

    pub(crate) fn adjust_hands_per_blind(&mut self, amount: i8) {
        self.hands_per_blind = adjust_per_blind_amount(self.hands_per_blind, amount);
    }

    pub(crate) fn adjust_discards_per_blind(&mut self, amount: i8) {
        self.discards_per_blind = adjust_per_blind_amount(self.discards_per_blind, amount);
    }
}

fn adjust_per_blind_amount(current: u8, amount: i8) -> u8 {
    if amount >= 0 {
        current.saturating_add(amount as u8)
    } else {
        current.saturating_sub(amount.unsigned_abs())
    }
}
