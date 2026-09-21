use crate::core::card::Card;

pub struct Hand {
    cards: Vec<Card>
}

impl Hand {
    pub fn hand_size(&self) -> u8 {
        self.cards.len() as u8
    }

    pub fn add_card(&mut self, card: Card) {
        self.cards.push(card);
    }
}