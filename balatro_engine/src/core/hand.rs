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

    pub fn select_cards(mut self, mut positions: Vec<u8>) -> Vec<Card> {
        let mut cards = Vec::new();
        positions.sort_unstable_by(|a, b| b.cmp(a));
        
        for position in positions {
            cards.push(self.cards.remove(position as usize));
        }

        cards
    }
}