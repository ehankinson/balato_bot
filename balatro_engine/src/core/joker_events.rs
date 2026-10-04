use rand::prelude::IndexedRandom;

use crate::core::card::Card;
use crate::core::enums::{PokerHand, ALL_RANKS, ALL_SUITS};
use crate::core::game::GameState;
use crate::core::joker::{Joker, JokerData, JokerStructure};
use crate::core::joker_types::{JokerKind, JokerTrigger, JokerUpdate};

pub enum TriggerEvent<'a> {
    PlayedCard(&'a Card),
    HeldCard(&'a Card),
    Discard(&'a [Card]),
    BeforePlayedCards,
    AfterHand,
    StartOfBlind,
    EndOfBlind,
}

pub struct Jokers {
    pub(super) jokers: Vec<Joker>,
}

impl Jokers {
    pub fn new() -> Jokers {
        Jokers {
            jokers: Vec::with_capacity(5),
        }
    }

    pub fn add(&mut self, joker: Joker) {
        self.jokers.push(joker);
    }

    pub fn initialize(&mut self, game_state: &mut GameState) {
        for joker in &mut self.jokers {
            joker.update(JokerUpdate::BlindCompleted, game_state);
        }
    }

    pub fn as_slice(&self) -> &[Joker] {
        &self.jokers
    }

    pub fn trigger(&mut self, event: TriggerEvent<'_>, game_state: &mut GameState) {
        for joker in &mut self.jokers {
            joker.trigger(&event, game_state);
        }
    }

    pub fn update(&mut self, event: JokerUpdate, game_state: &mut GameState) {
        for joker in &mut self.jokers {
            if joker.update_mask & event.mask() == 0 {
                continue;
            }

            joker.update(event, game_state);
        }
    }
}

impl Joker {
    pub(super) fn update_mask(joker_kind: JokerKind) -> u32 {
        use JokerUpdate::*;

        let mut mask = 0;
        match joker_kind {
            JokerKind::Glass => mask |= CardDestroyed.mask(),
            JokerKind::Marble
            | JokerKind::RiffRaff
            | JokerKind::Certificate
            | JokerKind::Madness
            | JokerKind::CeremonialDagger => mask |= BlindSelected.mask(),
            JokerKind::HitTheRoad => mask |= CardsDiscarded.mask() | RoundStarted.mask(),
            JokerKind::RideTheBus | JokerKind::Runner | JokerKind::SquareJoker => {
                mask |= HandCompleted.mask()
            }
            JokerKind::Invisible => mask |= RoundCompleted.mask(),
            JokerKind::Canio => mask |= CardDestroyed.mask(),
            JokerKind::Yorick => mask |= CardsDiscarded.mask(),
            JokerKind::IceCream
            | JokerKind::TurtleBean
            | JokerKind::Popcorn => mask |= RoundCompleted.mask(),
            JokerKind::Constellation => mask |= PlanetCardUsed.mask(),
            JokerKind::Green => mask |= HandCompleted.mask() | DiscardActionCompleted.mask(),
            JokerKind::RedCard => mask |= BoosterPackSkipped.mask(),
            JokerKind::Hologram => mask |= DeckChanged.mask(),
            JokerKind::Obelisk | JokerKind::Seltzer | JokerKind::SpareTrousers => {
                mask |= HandCompleted.mask()
            }
            JokerKind::LuckyCat => mask |= LuckyCardSucceeded.mask(),
            JokerKind::FlashCard => mask |= ShopRerolled.mask(),
            JokerKind::Ramen | JokerKind::Castle => mask |= CardsDiscarded.mask(),
            JokerKind::Campfire => mask |= AfterCardSold.mask() | BossBlindCompleted.mask(),
            JokerKind::Idol
            | JokerKind::Ancient
            | JokerKind::MailInRebate
            | JokerKind::ToDoList => mask |= BlindCompleted.mask(),
            JokerKind::Rocket => mask |= BlindCompleted.mask(),
            JokerKind::Blueprint | JokerKind::Brainstorm => {
                mask |= JokerOrderChanged.mask()
            }
            _ => {}
        }

        mask
    }

    fn trigger(&mut self, event: &TriggerEvent<'_>, game_state: &mut GameState) {
        match event {
            TriggerEvent::Discard(cards) if self.trigger == JokerTrigger::OnDiscard => {
                self.on_discard(cards, game_state);
            }
            TriggerEvent::PlayedCard(card) if self.trigger == JokerTrigger::OnPlayedCard => {
                self.on_played_card(card, game_state);
            }
            TriggerEvent::HeldCard(card) if self.trigger == JokerTrigger::OnHeldCard => {
                self.on_held_card(card, game_state);
            }
            TriggerEvent::BeforePlayedCards
                if self.trigger == JokerTrigger::BeforePlayedCards =>
            {
                self.on_before_played_cards(game_state);
            }
            TriggerEvent::AfterHand if self.trigger == JokerTrigger::AfterHand => {
                self.on_after_hand(game_state);
            }
            TriggerEvent::StartOfBlind if self.trigger == JokerTrigger::StartOfBlind => {
                self.on_start_of_blind(game_state);
            }
            TriggerEvent::EndOfBlind if self.trigger == JokerTrigger::EndOfBlind => {
                self.on_end_of_blind(game_state);
            }
            _ => {}
        }
    }

    fn on_discard(&mut self, cards: &[Card], game_state: &mut GameState) {
        match self.kind {
            JokerKind::MailInRebate => {
                let JokerStructure::Normal(JokerData::Econ(data)) = &self.structure else {
                    return;
                };
                let Some(target) = data.rank.as_ref().and_then(|ranks| ranks.first()) else {
                    return;
                };
                let matching_cards = cards
                    .iter()
                    .filter(|card| card.rank() == *target)
                    .count();
                game_state.money += data.money as i16 * matching_cards as i16;
            }
            JokerKind::Ramen => {
                let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure else {
                    return;
                };
                data.x_mult = (data.x_mult - 0.01 * cards.len() as f32).max(1.0);
            }
            _ => {}
        }
    }

    fn on_played_card(&mut self, _card: &Card, _game_state: &mut GameState) {}

    fn on_held_card(&mut self, _card: &Card, _game_state: &mut GameState) {}

    fn on_before_played_cards(&mut self, _game_state: &mut GameState) {}

    fn on_after_hand(&mut self, _game_state: &mut GameState) {}

    fn on_start_of_blind(&mut self, _game_state: &mut GameState) {}

    fn on_end_of_blind(&mut self, _game_state: &mut GameState) {}

    fn update(&mut self, event: JokerUpdate, game_state: &mut GameState) {
        match (event, self.kind) {
            (JokerUpdate::BlindCompleted, JokerKind::Idol) => {
                let rank = *ALL_RANKS.choose(&mut game_state.rng).unwrap();
                let suit = *ALL_SUITS.choose(&mut game_state.rng).unwrap();
                if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                    data.rank = Some(vec![rank]);
                    data.suit = Some(suit);
                }
            }
            (JokerUpdate::BlindCompleted, JokerKind::Ancient | JokerKind::Castle) => {
                let suit = *ALL_SUITS.choose(&mut game_state.rng).unwrap();
                if let JokerStructure::Normal(JokerData::Scoring(data)) = &mut self.structure {
                    data.suit = Some(suit);
                }
            }
            (JokerUpdate::BlindCompleted, JokerKind::MailInRebate) => {
                let rank = *ALL_RANKS.choose(&mut game_state.rng).unwrap();
                if let JokerStructure::Normal(JokerData::Econ(data)) = &mut self.structure {
                    data.rank = Some(vec![rank]);
                }
            }
            (JokerUpdate::BlindCompleted, JokerKind::ToDoList) => {
                let poker_hand = *PokerHand::DISCARD_HANDS
                    .choose(&mut game_state.rng)
                    .unwrap();
                if let JokerStructure::Normal(JokerData::Econ(data)) = &mut self.structure {
                    data.poker_hand = Some(poker_hand);
                }
            }
            _ => {}
        }
    }
}
