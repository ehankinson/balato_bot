use crate::core::enums::Decks;
use rand::rngs::StdRng;
use rand::SeedableRng;

pub struct GameState {
    pub(crate) money: i16,
    pub(crate) discards_used: u8,
    pub(crate) blind_is_boss: bool,
    pub(crate) rng: StdRng,
}

impl GameState {
    pub fn new(seed: u64) -> GameState {
        GameState {
            money: 0,
            discards_used: 0,
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
