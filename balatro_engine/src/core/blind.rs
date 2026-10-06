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

/// State that exists only while the current blind is active.
pub(crate) struct BlindState {
    pub(crate) kind: BlindKind,
    pub(crate) stake: Stakes,
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

    pub(crate) fn begin(&mut self, kind: BlindKind, ante: u8, hands: u8, discards: u8) {
        self.kind = kind;
        self.score_requirement = score_requirements(self.stake, ante)[kind.score_index()];
        self.hands_remaining = hands;
        self.discards_remaining = discards;
        self.discards_used = 0;
        self.hands_played = 0;
        self.chips_scored = 0;
        self.boss_ability_disabled = false;
        self.prevent_death = false;
        self.current_hand = None;
        self.target_suit = None;
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
