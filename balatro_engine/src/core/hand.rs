use crate::core::card::Card;

pub struct Hand {
    cards: Vec<Card>,
}

impl Hand {
    pub(crate) fn new() -> Hand {
        Hand { cards: Vec::new() }
    }

    pub fn hand_size(&self) -> u8 {
        self.cards.len() as u8
    }

    pub(crate) fn cards(&self) -> &[Card] {
        &self.cards
    }

    pub(crate) fn cards_mut(&mut self) -> &mut [Card] {
        &mut self.cards
    }

    pub(crate) fn card(&self, index: usize) -> Option<&Card> {
        self.cards.get(index)
    }

    pub(crate) fn card_mut(&mut self, index: usize) -> Option<&mut Card> {
        self.cards.get_mut(index)
    }

    pub(crate) fn remove_card_at(&mut self, index: usize) -> Option<Card> {
        (index < self.cards.len()).then(|| self.cards.remove(index))
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
