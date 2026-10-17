//! Independent game slots. Resets are explicit; terminal games are never replaced.
use crate::core::enums::{Decks, Stakes};
use crate::core::game::Game;

pub(crate) struct GameBatch {
    pub(crate) games: Vec<Game>,
    rules: Vec<(Decks, Stakes)>,
}

impl GameBatch {
    pub(crate) fn new(seeds: &[u64], rules: Vec<(Decks, Stakes)>) -> Self {
        assert_eq!(seeds.len(), rules.len());
        let games = seeds
            .iter()
            .zip(&rules)
            .map(|(seed, (deck, stake))| Game::with_rules(*seed, *deck, *stake))
            .collect();
        Self { games, rules }
    }

    pub(crate) fn reset(&mut self, index: usize, seed: u64) {
        let (deck, stake) = self.rules[index];
        self.games[index] = Game::with_rules(seed, deck, stake);
    }
}
