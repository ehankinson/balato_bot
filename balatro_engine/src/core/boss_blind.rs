use crate::consts::ante_scores::score_requirements;
use crate::core::blind::{BlindKind, BlindState};
use crate::core::card::Card;
use crate::core::enums::{BossBlinds, Enhancement, PokerHand, Stakes, Suit};
use crate::core::joker_types::JokerModifiers;
use rand::prelude::RngExt;
use rand::rngs::StdRng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RegularBossBlindHistory {
    appearances: [u16; 23],
}

impl RegularBossBlindHistory {
    pub(crate) const fn new() -> RegularBossBlindHistory {
        RegularBossBlindHistory {
            appearances: [0; 23],
        }
    }

    pub(crate) fn count(&self, boss: BossBlinds) -> u16 {
        self.appearances[boss as usize - 1]
    }

    pub(crate) fn record(&mut self, boss: BossBlinds) {
        let count = &mut self.appearances[boss as usize - 1];
        *count = count.saturating_add(1);
    }

    pub(crate) fn total(&self) -> u16 {
        self.appearances.iter().sum()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FinisherBossBlindHistory {
    appearances: [u16; 5],
}

impl FinisherBossBlindHistory {
    pub(crate) const fn new() -> FinisherBossBlindHistory {
        FinisherBossBlindHistory {
            appearances: [0; 5],
        }
    }

    pub(crate) fn count(&self, boss: BossBlinds) -> u16 {
        self.appearances[boss as usize - BossBlinds::AmberAcorn as usize]
    }

    pub(crate) fn record(&mut self, boss: BossBlinds) {
        let count = &mut self.appearances[boss as usize - BossBlinds::AmberAcorn as usize];
        *count = count.saturating_add(1);
    }

    pub(crate) fn total(&self) -> u16 {
        self.appearances.iter().sum()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BossBlindRule {
    pub(crate) minimum_ante: u8,
    pub(crate) target_multiplier: u8,
    pub(crate) reward: u8,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum BlindPlayError {
    NoHandsRemaining,
    TooManyCards,
    ForcedCardNotSelected,
}

impl BossBlinds {
    pub(crate) const ALL: [BossBlinds; 28] = [
        BossBlinds::Hook,
        BossBlinds::Ox,
        BossBlinds::House,
        BossBlinds::Wall,
        BossBlinds::Wheel,
        BossBlinds::Arm,
        BossBlinds::Club,
        BossBlinds::Fish,
        BossBlinds::Psychich,
        BossBlinds::Goad,
        BossBlinds::Water,
        BossBlinds::Window,
        BossBlinds::Manacle,
        BossBlinds::Eye,
        BossBlinds::Mouth,
        BossBlinds::Plante,
        BossBlinds::Serpent,
        BossBlinds::Pillar,
        BossBlinds::Needle,
        BossBlinds::Head,
        BossBlinds::Tooth,
        BossBlinds::Flint,
        BossBlinds::Mark,
        BossBlinds::AmberAcorn,
        BossBlinds::VerdantLeaf,
        BossBlinds::VioletVessel,
        BossBlinds::CrimsonHeart,
        BossBlinds::CeruleanBell,
    ];

    pub(crate) const SHOWDOWN: [BossBlinds; 5] = [
        BossBlinds::AmberAcorn,
        BossBlinds::VerdantLeaf,
        BossBlinds::VioletVessel,
        BossBlinds::CrimsonHeart,
        BossBlinds::CeruleanBell,
    ];

    pub(crate) const fn available_at_ante(self, ante: u8) -> bool {
        self.rule().minimum_ante <= ante && !self.is_showdown()
    }

    pub(crate) const fn rule(self) -> BossBlindRule {
        match self {
            BossBlinds::Hook => BossBlindRule::ordinary(1),
            BossBlinds::Ox => BossBlindRule::ordinary(6),
            BossBlinds::House
            | BossBlinds::Wheel
            | BossBlinds::Arm
            | BossBlinds::Fish
            | BossBlinds::Water
            | BossBlinds::Mouth
            | BossBlinds::Flint
            | BossBlinds::Mark => BossBlindRule::ordinary(2),
            BossBlinds::Needle => BossBlindRule {
                minimum_ante: 2,
                target_multiplier: 1,
                reward: 5,
            },
            BossBlinds::Wall => BossBlindRule {
                minimum_ante: 2,
                target_multiplier: 4,
                reward: 5,
            },
            BossBlinds::Club
            | BossBlinds::Psychich
            | BossBlinds::Goad
            | BossBlinds::Window
            | BossBlinds::Manacle
            | BossBlinds::Pillar
            | BossBlinds::Head => BossBlindRule::ordinary(1),
            BossBlinds::Eye | BossBlinds::Tooth => BossBlindRule::ordinary(3),
            BossBlinds::Plante => BossBlindRule::ordinary(4),
            BossBlinds::Serpent => BossBlindRule::ordinary(5),
            BossBlinds::AmberAcorn
            | BossBlinds::VerdantLeaf
            | BossBlinds::CrimsonHeart
            | BossBlinds::CeruleanBell => BossBlindRule::showdown(8, 2),
            BossBlinds::VioletVessel => BossBlindRule::showdown(8, 6),
        }
    }

    pub(crate) const fn is_showdown(self) -> bool {
        matches!(
            self,
            BossBlinds::AmberAcorn
                | BossBlinds::VerdantLeaf
                | BossBlinds::VioletVessel
                | BossBlinds::CrimsonHeart
                | BossBlinds::CeruleanBell
        )
    }
}

impl BossBlindRule {
    const fn ordinary(minimum_ante: u8) -> BossBlindRule {
        BossBlindRule {
            minimum_ante,
            target_multiplier: 2,
            reward: 5,
        }
    }

    const fn showdown(minimum_ante: u8, target_multiplier: u8) -> BossBlindRule {
        BossBlindRule {
            minimum_ante,
            target_multiplier,
            reward: 8,
        }
    }
}

impl BlindState {
    pub(crate) fn apply_boss_start_rules(&mut self) {
        let BlindKind::Boss(boss) = self.kind else {
            return;
        };

        match boss {
            BossBlinds::Water => {
                self.boss_removed_discards = self.discards_remaining;
                self.discards_remaining = 0;
            }
            BossBlinds::Manacle => self.hand_size_modifier = -1,
            BossBlinds::Needle => {
                self.boss_removed_hands = self.hands_remaining.saturating_sub(1);
                self.hands_remaining = 1;
            }
            BossBlinds::House => self.face_down_draws = true,
            BossBlinds::Mark => self.face_down_face_cards = true,
            BossBlinds::VerdantLeaf => self.verdant_leaf_active = true,
            _ => {}
        }
    }

    pub(crate) fn validate_boss_play(
        &self,
        selected_indices: &[usize],
    ) -> Result<(), BlindPlayError> {
        let BlindKind::Boss(boss) = self.kind else {
            return Ok(());
        };
        if self.boss_ability_disabled {
            return Ok(());
        }

        match boss {
            BossBlinds::CeruleanBell
                if self
                    .forced_card_index
                    .is_some_and(|index| !selected_indices.contains(&index)) =>
            {
                Err(BlindPlayError::ForcedCardNotSelected)
            }
            _ => Ok(()),
        }
    }

    pub(crate) fn hand_is_debuffed(&self, selected_count: usize, hand: PokerHand) -> bool {
        if self.boss_ability_disabled {
            return false;
        }
        match self.kind {
            BlindKind::Boss(BossBlinds::Psychich) => selected_count < 5,
            BlindKind::Boss(BossBlinds::Eye) => self.played_hand_types.contains(&hand),
            BlindKind::Boss(BossBlinds::Mouth) => self
                .played_hand_types
                .first()
                .is_some_and(|first| *first != hand),
            _ => false,
        }
    }

    pub(crate) fn record_boss_play(&mut self) {
        if self.boss_ability_disabled {
            return;
        }
        self.crimson_heart_prepped = matches!(self.kind, BlindKind::Boss(BossBlinds::CrimsonHeart));
        self.face_down_draws = matches!(self.kind, BlindKind::Boss(BossBlinds::Fish));
        self.serpent_draws = matches!(self.kind, BlindKind::Boss(BossBlinds::Serpent));
    }

    pub(crate) fn record_boss_discard(&mut self) {
        if self.boss_ability_disabled {
            return;
        }
        self.face_down_draws = false;
        self.serpent_draws = matches!(self.kind, BlindKind::Boss(BossBlinds::Serpent));
    }

    pub(crate) fn on_joker_sold(&mut self) {
        if matches!(self.kind, BlindKind::Boss(BossBlinds::VerdantLeaf)) {
            self.verdant_leaf_active = false;
        }
    }

    pub(crate) fn forced_discard_count(&self) -> usize {
        if self.boss_ability_disabled {
            0
        } else if matches!(self.kind, BlindKind::Boss(BossBlinds::Hook)) {
            2
        } else {
            0
        }
    }

    pub(crate) fn draw_amount(&self, requested: usize) -> usize {
        if !self.boss_ability_disabled && self.serpent_draws {
            3
        } else {
            requested
        }
    }

    pub(crate) fn prepare_drawn_card(
        &self,
        card: &mut Card,
        rng: &mut StdRng,
        modifiers: JokerModifiers,
    ) {
        card.set_debuffed(self.card_is_debuffed(card, modifiers));
        if self.boss_ability_disabled {
            card.set_face_down(false);
            return;
        }
        let face_down = match self.kind {
            BlindKind::Boss(BossBlinds::House) => !self.first_action_taken,
            BlindKind::Boss(BossBlinds::Wheel) => {
                (rng.random_range(0..7u8) as f32) < modifiers.probability_multiplier
            }
            BlindKind::Boss(BossBlinds::Fish) => self.face_down_draws,
            BlindKind::Boss(BossBlinds::Mark) => modifiers.is_face(card),
            _ => false,
        };
        card.set_face_down(face_down);
    }

    pub(crate) fn choose_forced_card(&mut self, hand_size: usize, rng: &mut StdRng) {
        if self
            .forced_card_index
            .is_some_and(|index| index < hand_size)
        {
            return;
        }
        self.forced_card_index = match self.kind {
            BlindKind::Boss(BossBlinds::CeruleanBell) if !self.boss_ability_disabled => {
                (hand_size > 0).then(|| rng.random_range(0..hand_size))
            }
            _ => None,
        };
    }

    pub(crate) fn reward(&self) -> u8 {
        match self.kind {
            BlindKind::Small => 3,
            BlindKind::Big => 4,
            BlindKind::Boss(boss) => boss.rule().reward,
        }
    }

    pub(crate) fn disable_boss_ability(&mut self) {
        if self.boss_ability_disabled {
            return;
        }
        self.hands_remaining = self.hands_remaining.saturating_add(self.boss_removed_hands);
        self.discards_remaining = self
            .discards_remaining
            .saturating_add(self.boss_removed_discards);
        self.boss_removed_hands = 0;
        self.boss_removed_discards = 0;
        if matches!(self.kind, BlindKind::Boss(BossBlinds::Manacle)) {
            self.hand_size_modifier += 1;
        }
        self.boss_ability_disabled = true;
        self.score_requirement = self.kind.score_requirement(self.stake, self.ante, true);
        self.forced_card_index = None;
        self.face_down_draws = false;
        self.face_down_face_cards = false;
        self.serpent_draws = false;
        self.crimson_heart_prepped = false;
        self.verdant_leaf_active = false;
    }

    pub(crate) fn card_is_debuffed(&self, card: &Card, modifiers: JokerModifiers) -> bool {
        if self.boss_ability_disabled {
            return false;
        }

        let boss = match self.kind {
            BlindKind::Boss(boss) => boss,
            _ => return false,
        };
        let matches_suit = |suit| {
            card.enhancement() != Enhancement::Stone
                && (card.enhancement() == Enhancement::Wild
                    || modifiers.suits_match(card.suit(), suit))
        };
        match boss {
            BossBlinds::Club => matches_suit(Suit::Clubs),
            BossBlinds::Goad => matches_suit(Suit::Spades),
            BossBlinds::Window => matches_suit(Suit::Diamonds),
            BossBlinds::Head => matches_suit(Suit::Hearts),
            BossBlinds::Plante => modifiers.is_face(card),
            BossBlinds::Pillar => card.was_played_in_ante(self.ante),
            BossBlinds::VerdantLeaf => self.verdant_leaf_active,
            _ => false,
        }
    }

    pub(crate) fn arm_active(&self) -> bool {
        matches!(self.kind, BlindKind::Boss(BossBlinds::Arm)) && !self.boss_ability_disabled
    }

    pub(crate) fn ox_active(&self) -> bool {
        matches!(self.kind, BlindKind::Boss(BossBlinds::Ox)) && !self.boss_ability_disabled
    }

    pub(crate) fn tooth_active(&self) -> bool {
        matches!(self.kind, BlindKind::Boss(BossBlinds::Tooth)) && !self.boss_ability_disabled
    }

    pub(crate) fn flint_active(&self) -> bool {
        matches!(self.kind, BlindKind::Boss(BossBlinds::Flint)) && !self.boss_ability_disabled
    }
}

impl BlindKind {
    pub(crate) fn score_requirement(self, stake: Stakes, ante: u8, disabled: bool) -> u64 {
        let requirements = score_requirements(stake, ante);
        match self {
            BlindKind::Small => requirements[0],
            BlindKind::Big => requirements[1],
            BlindKind::Boss(boss) => {
                let multiplier =
                    if disabled && matches!(boss, BossBlinds::Wall | BossBlinds::VioletVessel) {
                        2
                    } else {
                        boss.rule().target_multiplier
                    };
                requirements[2] / 2 * u64::from(multiplier)
            }
        }
    }
}
