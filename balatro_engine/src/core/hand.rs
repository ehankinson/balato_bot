use crate::core::card::Card;

pub struct Hand {
    cards: Vec<Card>,
}

impl Hand {
    pub(crate) fn new() -> Hand {
        Hand { cards: Vec::new() }
    }

    pub fn hand_size(&self) -> usize {
        self.cards.len()
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

    pub fn select_cards(&mut self, mut positions: Vec<usize>) -> Vec<Card> {
        positions.sort_unstable();

        if !crate::core::selection::valid_indices(&positions, self.cards.len()) {
            return Vec::new();
        }

        let mut selected = Vec::with_capacity(positions.len());
        for &position in positions.iter().rev() {
            selected.push(self.cards.remove(position));
        }
        selected.reverse();
        selected
    }

    pub(crate) fn reorder(&mut self, order: &[usize]) -> bool {
        if !crate::core::selection::valid_permutation(order, self.cards.len()) {
            return false;
        }
        let mut old = std::mem::take(&mut self.cards)
            .into_iter()
            .map(Some)
            .collect::<Vec<_>>();
        self.cards = order
            .iter()
            .map(|&index| old[index].take().unwrap())
            .collect();
        true
    }

    pub(crate) fn take_cards(&mut self) -> Vec<Card> {
        std::mem::take(&mut self.cards)
    }
}
