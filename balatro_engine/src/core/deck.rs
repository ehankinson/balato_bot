use crate::core::card::Card;
use crate::core::enums::{ALL_RANKS, ALL_SUITS, Decks, Edition, Enhancement, Rank, Seal};
use crate::core::hand::Hand;
use rand::prelude::IndexedRandom;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;

const DECK_SIZE: u8 = 52;
const BASE_HAND_SIZE: i8 = 8;

pub(crate) struct Deck {
    deck_type: Decks,
    hand_size_modifier: i8,
    starting_size: u16,
    cards: Vec<Card>,
    discarded_cards: Vec<Card>,
}

impl Deck {
    pub(crate) fn new(deck_type: Decks, rng: &mut StdRng) -> Deck {
        let cards = Deck::build_cards(&deck_type, rng);
        let hand_size_modifier = match deck_type {
            Decks::Black => -1,
            Decks::Painted => 2,
            _ => 0,
        };

        Deck {
            deck_type,
            hand_size_modifier,
            starting_size: cards.len() as u16,
            cards,
            discarded_cards: Vec::with_capacity(64),
        }
    }

    pub(crate) fn add_card_to_deck(&mut self, card: Card) {
        self.cards.push(card);
    }

    pub(crate) fn shuffle(&mut self, rng: &mut StdRng) {
        self.cards.shuffle(rng);
    }

    pub(crate) fn draw_card(&mut self) -> Option<Card> {
        self.cards.pop()
    }

    pub(crate) fn draw_cards(&mut self, amount: usize) -> Vec<Card> {
        (0..amount).filter_map(|_| self.draw_card()).collect()
    }

    pub(crate) fn deal_cards(&mut self, hand: &mut Hand, hand_size: u8, rng: &mut StdRng) {
        self.shuffle(rng);
        let add_card_amount = hand_size.saturating_sub(hand.hand_size());
        for _ in 0..add_card_amount {
            if let Some(card) = self.draw_card() {
                hand.add_card(card);
            }
        }
    }

    pub(crate) fn discard_card(&mut self, card: Card) {
        self.discarded_cards.push(card);
    }

    pub(crate) fn discard_cards(&mut self, cards: Vec<Card>) {
        self.discarded_cards.extend(cards);
    }

    pub(crate) fn reset_deck(&mut self) {
        self.cards.append(&mut self.discarded_cards);
    }

    pub(crate) fn remove_card_at(&mut self, index: usize) -> Option<Card> {
        (index < self.cards.len()).then(|| self.cards.remove(index))
    }

    pub(crate) fn cards(&self) -> &[Card] {
        &self.cards
    }

    pub(crate) fn discarded_cards(&self) -> &[Card] {
        &self.discarded_cards
    }

    pub(crate) fn card(&self, index: usize) -> Option<&Card> {
        self.cards.get(index)
    }

    pub(crate) fn deck_type(&self) -> Decks {
        self.deck_type
    }

    pub(crate) fn card_mut(&mut self, index: usize) -> Option<&mut Card> {
        self.cards.get_mut(index)
    }

    pub(crate) fn size(&self) -> u16 {
        self.cards.len() as u16
    }

    pub(crate) fn starting_size(&self) -> u16 {
        self.starting_size
    }

    pub(crate) fn hand_size(&self) -> i8 {
        BASE_HAND_SIZE + self.hand_size_modifier
    }

    pub(crate) fn adjust_hand_size(&mut self, amount: i8) {
        if amount >= 0 {
            self.hand_size_modifier = self.hand_size_modifier.saturating_add(amount);
        } else {
            self.hand_size_modifier = self
                .hand_size_modifier
                .saturating_sub(amount.unsigned_abs() as i8);
        }
    }

    pub(crate) fn consumable_slot_modifier(&self) -> i8 {
        match self.deck_type {
            Decks::Nebula => -1,
            _ => 0,
        }
    }

    pub(crate) fn enhanced_count(&self) -> u16 {
        self.cards
            .iter()
            .filter(|card| card.enhancement() != Enhancement::None)
            .count() as u16
    }

    pub(crate) fn rank_count(&self, rank: Rank) -> u16 {
        self.cards.iter().filter(|card| card.rank() == rank).count() as u16
    }

    pub(crate) fn remove_cards_of_rank(&mut self, rank: Rank, amount: usize) -> Vec<Card> {
        let mut remaining = amount;
        let mut removed = Vec::new();
        self.cards.retain(|card| {
            if remaining > 0 && card.rank() == rank {
                remaining -= 1;
                removed.push(card.clone());
                false
            } else {
                true
            }
        });
        removed
    }

    #[cfg(test)]
    pub(crate) fn remove_cards(&mut self, amount: usize) {
        let new_length = self.cards.len().saturating_sub(amount);
        self.cards.truncate(new_length);
    }

    fn build_cards(deck_type: &Decks, rng: &mut StdRng) -> Vec<Card> {
        let mut vec = Vec::with_capacity(64);
        if deck_type == &Decks::Erratic {
            for _ in 0..DECK_SIZE {
                vec.push(Card::new(
                    *ALL_RANKS.choose(rng).unwrap(),
                    *ALL_SUITS.choose(rng).unwrap(),
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
