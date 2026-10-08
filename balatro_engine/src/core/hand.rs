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

    pub fn select_cards(&mut self, mut positions: Vec<u8>) -> Vec<Card> {
        positions.sort_unstable();

        if positions.windows(2).any(|pair| pair[0] == pair[1])
            || positions
                .iter()
                .any(|&position| position as usize >= self.cards.len())
        {
            return Vec::new();
        }

        let mut selected = Vec::with_capacity(positions.len());
        for &position in positions.iter().rev() {
            selected.push(self.cards.remove(position as usize));
        }
        selected.reverse();
        selected
    }
}
