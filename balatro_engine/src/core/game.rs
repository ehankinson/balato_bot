use crate::core::enums::{Decks, PokerHand, Suit};
use rand::SeedableRng;
use rand::rngs::StdRng;

pub struct GameState {
    pub(crate) money: i16,
    pub(crate) discards_used: u8,
    pub(crate) discards_remaining: u8,
    pub(crate) hands_remaining: u8,
    pub(crate) hands_played: u32,
    pub(crate) round: u32,
    pub(crate) ante: u8,
    pub(crate) hand_size: i8,
    pub(crate) deck_size: u16,
    pub(crate) starting_deck_size: u16,
    pub(crate) enhanced_cards: u16,
    pub(crate) life: i16,
    pub(crate) cards_sold: u16,
    pub(crate) discarded_cards_since_gain: u32,
    pub(crate) tarot_cards_used: u32,
    pub(crate) planet_cards_used: u32,
    pub(crate) booster_packs_skipped: u32,
    pub(crate) shop_rerolls: u32,
    pub(crate) blinds_skipped: u32,
    pub(crate) nines_in_deck: u16,
    pub(crate) unique_planet_cards_used: u32,
    pub(crate) uncommon_jokers: u8,
    pub(crate) empty_joker_slots: u8,
    pub(crate) joker_sell_value_bonus: u8,
    pub(crate) consumable_sell_value_bonus: u8,
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
        GameState {
            money: 0,
            discards_used: 0,
            discards_remaining: 4,
            hands_remaining: 4,
            hands_played: 0,
            round: 1,
            ante: 1,
            hand_size: 8,
            deck_size: 52,
            starting_deck_size: 52,
            enhanced_cards: 0,
            life: 1,
            cards_sold: 0,
            discarded_cards_since_gain: 0,
            tarot_cards_used: 0,
            planet_cards_used: 0,
            booster_packs_skipped: 0,
            shop_rerolls: 0,
            blinds_skipped: 0,
            nines_in_deck: 0,
            unique_planet_cards_used: 0,
            uncommon_jokers: 0,
            empty_joker_slots: 5,
            joker_sell_value_bonus: 0,
            consumable_sell_value_bonus: 0,
            boss_blind_disabled: false,
            prevent_death: false,
            most_played_hand: None,
            current_hand: None,
            target_suit: None,
            blind_is_boss: false,
            rng: StdRng::seed_from_u64(seed),
        }
    }
}

pub struct Game {
    econ: i16,
    deck: Decks,
    // jokers: Jokers
    // consumables: Vec<Consumables>
}
