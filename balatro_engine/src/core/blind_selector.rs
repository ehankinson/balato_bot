use crate::core::blind::BlindKind;
use crate::core::enums::{BossBlinds, SkipTag};
use crate::core::game::GameState;
use rand::prelude::IndexedRandom;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) struct BlindChoice {
    pub(crate) kind: BlindKind,
    pub(crate) skip_tag: Option<SkipTag>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) struct BlindSelector {
    small: BlindChoice,
    big: BlindChoice,
    boss: BlindChoice,
}

impl BlindSelector {
    pub(crate) fn empty() -> BlindSelector {
        BlindSelector {
            small: BlindChoice {
                kind: BlindKind::Small,
                skip_tag: Some(SkipTag::Standard),
            },
            big: BlindChoice {
                kind: BlindKind::Big,
                skip_tag: Some(SkipTag::Standard),
            },
            boss: BlindChoice {
                kind: BlindKind::Boss(BossBlinds::Hook),
                skip_tag: None,
            },
        }
    }

    pub(crate) fn generate(state: &mut GameState) -> BlindSelector {
        let small_tag = *SkipTag::ALL
            .choose(&mut state.rng)
            .expect("the skip tag table must not be empty");
        let big_tag = *SkipTag::ALL
            .choose(&mut state.rng)
            .expect("the skip tag table must not be empty");
        let boss = state.random_boss_blind();

        BlindSelector {
            small: BlindChoice {
                kind: BlindKind::Small,
                skip_tag: Some(small_tag),
            },
            big: BlindChoice {
                kind: BlindKind::Big,
                skip_tag: Some(big_tag),
            },
            boss: BlindChoice {
                kind: BlindKind::Boss(boss),
                skip_tag: None,
            },
        }
    }

    pub(crate) fn small(&self) -> BlindChoice {
        self.small
    }

    pub(crate) fn big(&self) -> BlindChoice {
        self.big
    }

    pub(crate) fn boss(&self) -> BlindChoice {
        self.boss
    }

    #[cfg(test)]
    pub(crate) fn set_small_skip_tag(&mut self, tag: SkipTag) {
        self.small.skip_tag = Some(tag);
    }
}
