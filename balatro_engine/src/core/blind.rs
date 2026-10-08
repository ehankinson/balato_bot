pub(crate) use crate::calculation::calculate_score::ScoreResult;
use crate::calculation::calculate_score::calculate_score as calculate_score_impl;
use crate::calculation::poker::determine_poker_hand;
use crate::consts::ante_scores::score_requirements;
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
    scoring_data.scoring_held_cards = scoring_cards(&card_ids(&held_cards));
    Some(calculate_score_impl(
        state,
        &played_cards,
        &held_cards,
        selected_indices,
        hand_type,
        scoring_data,
    ))
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
        }
    }

    pub(crate) fn begin(&mut self, kind: BlindKind, ante: u8) {
        self.kind = kind;
        self.score_requirement = score_requirements(self.stake, ante)[kind.score_index()];
        self.hands_remaining = self.hands_per_blind;
        self.discards_remaining = self.discards_per_blind;
        self.discards_used = 0;
        self.hands_played = 0;
        self.chips_scored = 0;
        self.boss_ability_disabled = false;
        self.prevent_death = false;
        self.current_hand = None;
        self.target_suit = None;
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

impl BlindKind {
    const fn score_index(self) -> usize {
        match self {
            BlindKind::Small => 0,
            BlindKind::Big => 1,
            BlindKind::Boss(_) => 2,
        }
    }
}
