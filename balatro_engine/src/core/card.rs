use crate::core::enums::{Edition, Enhancement, Rank, Seal, Suit};

pub struct Card {
    rank: Rank,
    suit: Suit,
    enhancement: Enhancement,
    seal: Seal,
    edition: Edition,
    debuffed: bool,
    score: u32,
    id: u16,
}

impl Card {
    pub fn new(
        rank: Rank,
        suit: Suit,
        enhancement: Enhancement,
        edition: Edition,
        seal: Seal,
    ) -> Card {
        let id = Card::build_id(&rank, &suit, &enhancement, &edition, &seal);

        Card {
            rank: rank,
            suit: suit,
            enhancement: enhancement,
            edition: edition,
            seal: seal,
            debuffed: false,
            score: 0,
            id: id,
        }
    }

    fn build_id(
        rank: &Rank,
        suit: &Suit,
        enhancement: &Enhancement,
        edition: &Edition,
        seal: &Seal,
    ) -> u16 {
        ((*rank as u16) << 9
            | ((*suit as u16) << 7)
            | ((*enhancement as u16) << 4)
            | ((*seal) as u16) << 2)
            | (*edition as u16)
    }
}
