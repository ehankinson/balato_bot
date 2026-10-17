//! Tooltip parameters owned by the Joker feature; never resolve copy targets or
//! invoke scoring, updates, random rolls, or concealed Joker modifiers.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub(crate) enum JokerValueKey {
    Chips = 1,
    AddMult = 2,
    XMult = 3,
    Money = 4,
    RequiredCount = 5,
    HandSizeModifier = 6,
    Probability = 7,
    TargetSuit = 8,
    TargetHand = 9,
    TargetRanks = 10,
    HandsModifier = 11,
    DiscardsModifier = 12,
    RerollCostModifier = 13,
    MinMoney = 14,
    StraightFlushSize = 15,
    RetriggerCount = 16,
    GenerationCount = 17,
    RoundsCompleted = 18,
    ActiveHands = 19,
    PerishableRounds = 20,
}

impl JokerValueKey {
    pub(crate) fn is_raw(self) -> bool {
        matches!(
            self,
            Self::Probability
                | Self::TargetSuit
                | Self::TargetHand
                | Self::TargetRanks
                | Self::RoundsCompleted
                | Self::ActiveHands
                | Self::PerishableRounds
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct JokerVisibleValue {
    pub(crate) key: JokerValueKey,
    pub(crate) value: f64,
}

impl Joker {
    pub(crate) fn visible_values(
        &self,
        blind: &crate::core::blind::BlindState,
    ) -> Vec<JokerVisibleValue> {
        use JokerValueKey::*;
        if self.is_face_down() {
            return Vec::new();
        }
        let mut values = Vec::new();
        let mut add = |key, value| values.push(JokerVisibleValue { key, value });
        match &self.structure {
            JokerStructure::Copy(_) => {}
            JokerStructure::Normal(JokerData::Scoring(data)) => {
                if data.chips != 0
                    || matches!(
                        self.kind,
                        JokerKind::Runner
                            | JokerKind::SquareJoker
                            | JokerKind::Castle
                            | JokerKind::Wee
                    )
                {
                    add(Chips, f64::from(data.chips));
                }
                if data.add_mult != 0
                    || matches!(
                        self.kind,
                        JokerKind::CeremonialDagger
                            | JokerKind::RideTheBus
                            | JokerKind::Green
                            | JokerKind::RedCard
                            | JokerKind::SpareTrousers
                            | JokerKind::HitTheRoad
                            | JokerKind::FortuneTeller
                    )
                {
                    add(AddMult, f64::from(data.add_mult));
                }
                if data.x_mult != 1.0
                    || matches!(
                        self.kind,
                        JokerKind::Constellation
                            | JokerKind::Madness
                            | JokerKind::Vampire
                            | JokerKind::Hologram
                            | JokerKind::Obelisk
                            | JokerKind::LuckyCat
                            | JokerKind::Campfire
                            | JokerKind::Glass
                            | JokerKind::Canio
                            | JokerKind::Yorick
                    )
                {
                    add(XMult, f64::from(data.x_mult));
                }
                if data.req_count != 0 || self.kind == JokerKind::Yorick {
                    add(RequiredCount, f64::from(data.req_count));
                }
                if data.hand_size != 0 {
                    add(HandSizeModifier, f64::from(data.hand_size));
                }
                if data.probability != 0.0 {
                    add(Probability, f64::from(data.probability));
                }
                if let Some(suit) = data.suit.filter(|_| self.kind != JokerKind::Castle) {
                    add(TargetSuit, suit as u8 as f64);
                }
                if let Some(hand) = data.poker_hand {
                    add(TargetHand, hand as u8 as f64);
                }
                if let Some(ranks) = &data.rank {
                    add(TargetRanks, rank_mask(ranks));
                }
            }
            JokerStructure::Normal(JokerData::Econ(data)) => {
                if data.money != 0 {
                    add(Money, f64::from(data.money));
                }
                if data.required_count != 0 {
                    add(RequiredCount, f64::from(data.required_count));
                }
                if data.probability != 0.0 {
                    add(Probability, f64::from(data.probability));
                }
                if let Some(suit) = data.suit {
                    add(TargetSuit, suit as u8 as f64);
                }
                if let Some(hand) = data.poker_hand {
                    add(TargetHand, hand as u8 as f64);
                }
                if let Some(ranks) = &data.rank {
                    add(TargetRanks, rank_mask(ranks));
                }
            }
            JokerStructure::Normal(JokerData::GameState(data)) => {
                if data.hand_size != 0 {
                    add(HandSizeModifier, f64::from(data.hand_size));
                }
                if data.hands != 0 {
                    add(HandsModifier, f64::from(data.hands));
                }
                if data.discards != 0 {
                    add(DiscardsModifier, f64::from(data.discards));
                }
                if data.reroll_cost != 0 {
                    add(RerollCostModifier, f64::from(data.reroll_cost));
                }
                if data.min_money != 0 {
                    add(MinMoney, f64::from(data.min_money));
                }
                if data.straight_flush_size != 5 {
                    add(StraightFlushSize, f64::from(data.straight_flush_size));
                }
                // data.probability here is a modifier, not a [0,1] chance.
            }
            JokerStructure::Normal(JokerData::Retrigger(data)) => {
                add(RetriggerCount, f64::from(data.retrigger))
            }
            JokerStructure::Normal(JokerData::Generate(data)) => {
                add(GenerationCount, f64::from(data.amount));
                add(Probability, f64::from(data.probability));
                if let Some(rank) = data.rank {
                    add(TargetRanks, rank_mask(&[rank]));
                }
                if let Some(hand) = data.poker_hand {
                    add(TargetHand, hand as u8 as f64);
                }
            }
        }
        if self.kind == JokerKind::Castle {
            if let Some(suit) = blind.target_suit {
                add(TargetSuit, suit as u8 as f64);
            }
        }
        if self.kind == JokerKind::Invisible {
            add(RoundsCompleted, f64::from(self.rounds_completed));
        }
        if self.kind == JokerKind::Seltzer {
            add(ActiveHands, f64::from(self.active_hands));
        }
        if let Some(rounds) = self.perishable_rounds {
            add(PerishableRounds, f64::from(rounds));
        }
        values.sort_by_key(|value| value.key as i32);
        values
    }

    pub(crate) fn sell_value_after_discount(&self, discount: u8) -> u8 {
        let mut acquired = self.clone();
        acquired.apply_discount(discount);
        acquired.sell_value()
    }
}

fn rank_mask(ranks: &[Rank]) -> f64 {
    ranks
        .iter()
        .fold(0u16, |mask, rank| mask | (1 << (*rank as u8 - 2))) as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::game::Game;
    use crate::core::observation::ObservationError;

    #[test]
    fn concealed_nonfinite_joker_values_are_not_read_until_revealed() {
        let mut game = Game::new(601);
        let mut joker = Joker::create_joker(JokerKind::Hologram);
        joker.set_face_down(true);
        game.state.jokers.add(joker);
        let before = game.observe().unwrap();
        let joker = game.state.jokers.get_mut(0).unwrap();
        if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut joker.structure {
            data.x_mult = f32::INFINITY;
        }
        assert_eq!(game.observe().unwrap(), before);
        game.state.jokers.get_mut(0).unwrap().set_face_down(false);
        assert_eq!(game.observe(), Err(ObservationError::NonFinite));
    }

    #[test]
    fn invalid_visible_probabilities_fail_without_mutating_or_rolling() {
        let mut game = Game::new(602);
        game.state
            .jokers
            .add(Joker::create_joker(JokerKind::EightBall));
        let joker = game.state.jokers.get_mut(0).unwrap();
        if let JokerStructure::Normal(JokerData::Generate(data)) = &mut joker.structure {
            data.probability = 2.0;
        }
        let rng = game.state.rng.get_word_pos();
        assert_eq!(game.observe(), Err(ObservationError::InvalidProbability));
        assert_eq!(game.state.rng.get_word_pos(), rng);
    }
}
