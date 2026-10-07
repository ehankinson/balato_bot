use crate::consts::ante_scores::score_requirements;
use crate::core::enums::{BossBlinds, PokerHand, Stakes, Suit};

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
