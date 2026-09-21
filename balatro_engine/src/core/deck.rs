use crate::core::card::Card;
use crate::core::enums::{ALL_RANKS, ALL_SUITS, Decks, Edition, Enhancement, Rank, Seal};
use rand::prelude::IndexedRandom;

const DECK_SIZE: u8 = 52;

struct Deck {
    deck_type: Decks,
    available_cards: u8,
    cards: [Card; 128],
}

impl Deck {
    pub fn new(deck_type: Decks) {
        let cards = Deck::build_cards(deck_type);
    }

    fn build_cards(deck_type: Decks) -> Vec<Card> {
        let mut vec = Vec::with_capacity(128);
        if deck_type == Decks::Erratic {
            let mut rng = rand::rng();
            for _ in 0..DECK_SIZE {
                vec.push(Card::new(
                    *ALL_RANKS.choose(&mut rng).unwrap(),
                    *ALL_SUITS.choose(&mut rng).unwrap(),
                    Enhancement::None,
                    Edition::None,
                    Seal::None,
                ))
            }
        } else {
            for rank in ALL_RANKS {
                if deck_type == Decks::Abandoned && Rank::is_face_card(&rank) {
                    continue;
                }

                for suit in ALL_SUITS {
                    vec.push(Card::new(
                        rank,
                        suit,
                        Enhancement::None,
                        Edition::None,
                        Seal::None,
                    ))
                }
            }
        }

        vec
    }
}
