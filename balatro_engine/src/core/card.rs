use crate::core::enums::{Edition, Enhancement, Rank, Seal, Suit};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ScoringValues {
    pub(crate) chips: i32,
    pub(crate) add_mult: i32,
    pub(crate) x_mult: f32,
}

impl Default for ScoringValues {
    fn default() -> ScoringValues {
        ScoringValues {
            chips: 0,
            add_mult: 0,
            x_mult: 1.0,
        }
    }
}

#[derive(Clone)]
pub struct Card {
    rank: Rank,
    suit: Suit,
    enhancement: Enhancement,
    seal: Seal,
    edition: Edition,
    debuffed: bool,
    scoring_values: ScoringValues,
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
            scoring_values: ScoringValues::default(),
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

    pub(crate) fn rank(&self) -> Rank {
        self.rank
    }

    pub(crate) fn suit(&self) -> Suit {
        self.suit
    }

    pub(crate) fn enhancement(&self) -> Enhancement {
        self.enhancement
    }

    pub(crate) fn seal(&self) -> Seal {
        self.seal
    }

    pub(crate) fn edition(&self) -> Edition {
        self.edition
    }

    pub(crate) fn scoring_values(&self) -> ScoringValues {
        let mut values = self.scoring_values;
        values.chips += self.rank.base_chips() as i32;

        match self.enhancement {
            Enhancement::Bonus => values.chips += 30,
            Enhancement::Mult => values.add_mult += 4,
            Enhancement::Glass => values.x_mult *= 2.0,
            Enhancement::Stone => values.chips += 50,
            Enhancement::None
            | Enhancement::Gold
            | Enhancement::Lucky
            | Enhancement::Steel
            | Enhancement::Wild => {}
        }

        match self.edition {
            Edition::Foil => values.chips += 50,
            Edition::Holographic => values.add_mult += 10,
            Edition::Polychrome => values.x_mult *= 1.5,
            Edition::None => {}
        }

        values
    }

    pub(crate) fn scoring_values_mut(&mut self) -> &mut ScoringValues {
        &mut self.scoring_values
    }

    pub(crate) fn is_face_card(&self) -> bool {
        self.rank.is_face_card()
    }

    pub(crate) fn is_low_card(&self) -> bool {
        self.rank.is_low_card()
    }

    pub(crate) fn id(&self) -> u16 {
        self.id
    }

    pub(crate) fn set_rank(&mut self, rank: Rank) {
        self.rank = rank;
        self.refresh_id();
    }

    pub(crate) fn set_suit(&mut self, suit: Suit) {
        self.suit = suit;
        self.refresh_id();
    }

    pub(crate) fn set_enhancement(&mut self, enhancement: Enhancement) {
        self.enhancement = enhancement;
        self.refresh_id();
    }

    pub(crate) fn set_seal(&mut self, seal: Seal) {
        self.seal = seal;
        self.refresh_id();
    }

    pub(crate) fn set_edition(&mut self, edition: Edition) {
        self.edition = edition;
        self.refresh_id();
    }

    pub(crate) fn copy_properties_from(&mut self, other: &Card) {
        self.rank = other.rank;
        self.suit = other.suit;
        self.enhancement = other.enhancement;
        self.seal = other.seal;
        self.edition = other.edition;
        self.refresh_id();
    }

    fn refresh_id(&mut self) {
        self.id = Card::build_id(
            &self.rank,
            &self.suit,
            &self.enhancement,
            &self.edition,
            &self.seal,
        );
    }
}
