use crate::consts::ante_scores::score_requirements;
use crate::core::blind::{BlindKind, BlindState};
use crate::core::card::Card;
use crate::core::enums::{BossBlinds, PokerHand, Stakes, Suit};
use rand::prelude::RngExt;
use rand::rngs::StdRng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BossBlindRule {
    pub(crate) minimum_ante: u8,
    pub(crate) target_multiplier: u8,
    pub(crate) reward: u8,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum BlindPlayError {
    NoHandsRemaining,
    TooFewCards,
    HandTypeNotAllowed,
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
            BossBlinds::Water => self.discards_remaining = 0,
            BossBlinds::Manacle => self.hand_size_modifier = -1,
            BossBlinds::Needle => self.hands_remaining = 1,
            BossBlinds::House => self.face_down_draws = true,
            BossBlinds::Mark => self.face_down_face_cards = true,
            BossBlinds::VerdantLeaf => self.verdant_leaf_active = true,
            BossBlinds::CeruleanBell => self.forced_card_index = Some(0),
            _ => {}
        }
    }

    pub(crate) fn validate_boss_play(
        &self,
        selected_indices: &[usize],
        hand_type: PokerHand,
    ) -> Result<(), BlindPlayError> {
        let BlindKind::Boss(boss) = self.kind else {
            return Ok(());
        };
        if self.boss_ability_disabled {
            return Ok(());
        }

        match boss {
            BossBlinds::Psychich if selected_indices.len() != 5 => Err(BlindPlayError::TooFewCards),
            BossBlinds::Eye if self.played_hand_types.contains(&hand_type) => {
                Err(BlindPlayError::HandTypeNotAllowed)
            }
            BossBlinds::Mouth
                if self
                    .played_hand_types
                    .first()
                    .is_some_and(|first| *first != hand_type) =>
            {
                Err(BlindPlayError::HandTypeNotAllowed)
            }
            BossBlinds::Needle if self.hands_played > 0 => Err(BlindPlayError::NoHandsRemaining),
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

    pub(crate) fn record_boss_play(&mut self) {
        self.face_down_draws = matches!(self.kind, BlindKind::Boss(BossBlinds::Fish));
        self.serpent_draws = matches!(self.kind, BlindKind::Boss(BossBlinds::Serpent));
    }

    pub(crate) fn record_boss_discard(&mut self) {
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

    pub(crate) fn prepare_drawn_card(&self, card: &mut Card, rng: &mut StdRng) {
        card.set_debuffed(self.card_is_debuffed(card));
        let face_down = match self.kind {
            BlindKind::Boss(BossBlinds::House) => !self.first_action_taken,
            BlindKind::Boss(BossBlinds::Wheel) => rng.random_range(0..7u8) == 0,
            BlindKind::Boss(BossBlinds::Fish) => self.face_down_draws,
            BlindKind::Boss(BossBlinds::Mark) => card.is_face_card(),
            _ => false,
        };
        card.set_face_down(face_down);
    }

    pub(crate) fn choose_forced_card(&mut self, hand_size: usize, rng: &mut StdRng) {
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
        self.boss_ability_disabled = true;
        self.score_requirement = self.kind.score_requirement(self.stake, self.ante, true);
        self.forced_card_index = None;
        self.face_down_draws = false;
        self.face_down_face_cards = false;
        self.serpent_draws = false;
        self.verdant_leaf_active = false;
    }

    pub(crate) fn card_is_debuffed(&self, card: &Card) -> bool {
        if self.boss_ability_disabled {
            return card.is_debuffed();
        }

        let boss = match self.kind {
            BlindKind::Boss(boss) => boss,
            _ => return card.is_debuffed(),
        };
        let boss_debuffs_card = match boss {
            BossBlinds::Club => card.suit() == Suit::Clubs,
            BossBlinds::Goad => card.suit() == Suit::Spades,
            BossBlinds::Window => card.suit() == Suit::Diamonds,
            BossBlinds::Head => card.suit() == Suit::Hearts,
            BossBlinds::Plante => card.is_face_card(),
            BossBlinds::VerdantLeaf => self.verdant_leaf_active,
            _ => false,
        };
        card.is_debuffed() || boss_debuffs_card
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
