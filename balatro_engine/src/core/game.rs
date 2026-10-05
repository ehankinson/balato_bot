use crate::core::deck::Deck;
use crate::core::enums::{Consumable, Decks, PokerHand, Rank, Suit, Vouchers};
use crate::core::hand::Hand;
use crate::core::joker::{JokerEffect, Jokers, TriggerEvent, UpdateEvent};
use rand::rngs::StdRng;
use rand::SeedableRng;

/// Run counters shared by the shop and blind phases.
///
/// Hands and discards remain here until the dedicated blind state is added.
/// They are per-blind values today; the other fields accumulate across the run.
pub(crate) struct GameCounters {
    pub(crate) discards_used: u8,
    pub(crate) discards_remaining: u8,
    pub(crate) hands_remaining: u8,
    pub(crate) hands_played: u32,
    pub(crate) round: u32,
    pub(crate) ante: u8,
    pub(crate) life: i16,
    pub(crate) cards_sold: u16,
    pub(crate) discarded_cards_since_gain: u32,
    pub(crate) tarot_cards_used: u32,
    pub(crate) planet_cards_used: u32,
    pub(crate) booster_packs_skipped: u32,
    pub(crate) shop_rerolls: u32,
    pub(crate) blinds_skipped: u32,
    pub(crate) unique_planet_cards_used: u32,
    pub(crate) uncommon_jokers: u8,
    pub(crate) empty_joker_slots: u8,
    pub(crate) joker_sell_value_bonus: u8,
    pub(crate) consumable_sell_value_bonus: u8,
    pub(crate) hand_size_modifier: i8,
}

impl GameCounters {
    fn new() -> GameCounters {
        GameCounters {
            discards_used: 0,
            discards_remaining: 4,
            hands_remaining: 4,
            hands_played: 0,
            round: 1,
            ante: 1,
            life: 1,
            cards_sold: 0,
            discarded_cards_since_gain: 0,
            tarot_cards_used: 0,
            planet_cards_used: 0,
            booster_packs_skipped: 0,
            shop_rerolls: 0,
            blinds_skipped: 0,
            unique_planet_cards_used: 0,
            uncommon_jokers: 0,
            empty_joker_slots: 5,
            joker_sell_value_bonus: 0,
            consumable_sell_value_bonus: 0,
            hand_size_modifier: 0,
        }
    }
}

pub struct GameState {
    pub(crate) deck: Deck,
    pub(crate) jokers: Jokers,
    pub(crate) money: i16,
    pub(crate) vouchers: Vec<Vouchers>,
    pub(crate) consumables: Vec<Consumable>,
    pub(crate) counters: GameCounters,
    pub(crate) boss_blind_disabled: bool,
    pub(crate) prevent_death: bool,
    pub(crate) most_played_hand: Option<PokerHand>,
    pub(crate) current_hand: Option<PokerHand>,
    pub(crate) target_suit: Option<Suit>,
    pub(crate) blind_is_boss: bool,
    pub(crate) rng: StdRng,
}

impl GameState {
    pub fn new(seed: u64) -> GameState {
        GameState::with_deck(seed, Decks::Red)
    }

    pub(crate) fn with_deck(seed: u64, deck_type: Decks) -> GameState {
        let mut rng = StdRng::seed_from_u64(seed);

        GameState {
            deck: Deck::new(deck_type, &mut rng),
            jokers: Jokers::new(),
            money: 0,
            vouchers: Vec::new(),
            consumables: Vec::new(),
            counters: GameCounters::new(),
            boss_blind_disabled: false,
            prevent_death: false,
            most_played_hand: None,
            current_hand: None,
            target_suit: None,
            blind_is_boss: false,
            rng,
        }
    }

    pub(crate) fn hand_size(&self) -> i8 {
        self.deck.hand_size() + self.counters.hand_size_modifier
    }

    pub(crate) fn deck_size(&self) -> u16 {
        self.deck.size()
    }

    pub(crate) fn starting_deck_size(&self) -> u16 {
        self.deck.starting_size()
    }

    pub(crate) fn enhanced_cards(&self) -> u16 {
        self.deck.enhanced_count()
    }

    pub(crate) fn nines_in_deck(&self) -> u16 {
        self.deck.rank_count(Rank::Nine)
    }

    pub(crate) fn deal_cards(&mut self, hand: &mut Hand) {
        let hand_size = self.hand_size().max(0) as u8;
        self.deck.deal_cards(hand, hand_size, &mut self.rng);
    }

    pub(crate) fn trigger_jokers(&mut self, event: TriggerEvent<'_>) -> JokerEffect {
        let mut jokers = std::mem::replace(&mut self.jokers, Jokers::new());
        let effect = jokers.trigger(event, self);
        self.jokers = jokers;
        effect
    }

    pub(crate) fn update_jokers(&mut self, event: UpdateEvent<'_>) -> JokerEffect {
        let mut jokers = std::mem::replace(&mut self.jokers, Jokers::new());
        let effect = jokers.update(event, self);
        self.jokers = jokers;
        effect
    }
}

/// The runtime object that will coordinate phase transitions.
pub struct Game {
    pub(crate) state: GameState,
}

impl Game {
    pub fn new(seed: u64) -> Game {
        Game {
            state: GameState::new(seed),
        }
    }
}

#[cfg(test)]
#[path = "tests/game_state.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/card_deck.rs"]
mod card_deck_tests;
