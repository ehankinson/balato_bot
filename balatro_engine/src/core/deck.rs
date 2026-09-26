use crate::core::card::Card;
use crate::core::enums::{ALL_RANKS, ALL_SUITS, Decks, Edition, Enhancement, Rank, Seal};
use crate::core::hand::Hand;
use rand::prelude::IndexedRandom;
use rand::seq::SliceRandom;

const DECK_SIZE: u8 = 52;

struct Deck {
    deck_type: Decks,
    available_cards: u8,
    hand_size: u8,
    cards: Vec<Card>,
    discarded_cards: Vec<Card>,
}

impl Deck {
    pub fn new(deck_type: Decks) -> Deck {
        let cards = Deck::build_cards(&deck_type);
        let hand_size = match deck_type {
            Decks::Painted => 10,
            _ => 8,
        };

        Deck {
            deck_type,
            available_cards: cards.len() as u8,
            hand_size,
            cards,
            discarded_cards: Vec::with_capacity(64),
        }
    }

    pub fn add_card_to_deck(&mut self, card: Card) {
        self.cards.push(card);
    }

    pub fn deal_cards(&mut self, hand: &mut Hand) {
        let mut rng = rand::rng();
        self.cards.shuffle(&mut rng);
        let add_card_amount = self.hand_size - hand.hand_size();
        for _ in 0..add_card_amount {
            hand.add_card(self.cards.pop().unwrap());
        }
    }

    pub fn add_to_discard(mut self, mut cards: Vec<Card>) {
        let length = cards.len();
        for _ in 0..length {
            self.discarded_cards.push(cards.pop().unwrap());
        }
    }

    pub fn reset_deck(&mut self) {
        let length = self.discarded_cards.len();
        for _ in 0..length {
            self.cards.push(self.discarded_cards.pop().unwrap());
        }
    }

    fn build_cards(deck_type: &Decks) -> Vec<Card> {
        let mut vec = Vec::with_capacity(64);
        if deck_type == &Decks::Erratic {
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
                if deck_type == &Decks::Abandoned && Rank::is_face_card(&rank) {
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
