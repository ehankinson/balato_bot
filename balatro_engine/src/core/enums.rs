#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
#[repr(u8)]
pub(crate) enum PokerHand {
    HighCard = 1,
    Pair = 2,
    ThreeOfAKind = 3,
    FourOfAKind = 4,
    FiveOfAKind = 5,
    TwoPair = 6,
    Straight = 7,
    Flush = 8,
    FullHouse = 9,
    StraightFlush = 10,
    FlushHouse = 11,
    FlushFive = 12,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub(crate) enum Rank {
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
    Six = 6,
    Seven = 7,
    Eight = 8,
    Nine = 9,
    Ten = 10,
    Jack = 11,
    Queen = 12,
    King = 13,
    Ace = 14,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub(crate) enum Suit {
    Hearts = 0,
    Diamonds = 1,
    Clubs = 2,
    Spades = 3,
}

#[derive(Clone, Copy)]
pub(crate) enum Enhancement {
    None = 0,
    Stone = 1,
    Gold = 2,
    Bonus = 3,
    Mult = 4,
    Wild = 5,
    Lucky = 6,
    Glass = 7,
    Steel = 8,
}

#[derive(Clone, Copy)]
pub(crate) enum Seal {
    None = 0,
    Gold = 1,
    Purple = 2,
    Blue = 3,
    Red = 4,
}

#[derive(Clone, Copy)]
pub(crate) enum Edition {
    None = 0,
    Foil = 1,
    Holographic = 2,
    Polychrome = 3,
}

#[derive(PartialEq, Eq)]
pub(crate) enum Decks {
    Red = 1,
    Blue = 2,
    Yellow = 3,
    Green = 4,
    Black = 5,
    Magic = 6,
    Nebula = 7,
    Ghost = 8,
    Abandoned = 9,
    Checkered = 10,
    Zodiac = 11,
    Painted = 12,
    Anaglyph = 13,
    Plasma = 14,
    Erratic = 15,
}

pub(crate) enum Stakes {
    White = 1,
    Red = 2,
    Green = 3,
    Black = 4,
    Blue = 5,
    Purple = 6,
    Orange = 7,
    Gold = 8,
}

pub(crate) enum BossBlinds {
    Hook = 1,
    Ox = 2,
    House = 3,
    Wall = 4,
    Wheel = 5,
    Arm = 6,
    Club = 7,
    Fish = 8,
    Psychich = 9,
    Goad = 10,
    Water = 11,
    Window = 12,
    Manacle = 13,
    Eye = 14,
    Mouth = 15,
    Plante = 16,
    Serpent = 17,
    Pillar = 18,
    Needle = 19,
    Head = 20,
    Tooth = 21,
    Flint = 22,
    Mark = 23,
    AmberAcorn = 24,
    VerdantLeaf = 25,
    VioletVessel = 26,
    CrimsonHeart = 27,
    CeruleanBell = 28,
}

pub(crate) enum SkipTag {
    Uncommon = 1,
    Rare = 2,
    Negative = 3,
    Foil = 4,
    Holographic = 5,
    Polychrome = 6,
    Investment = 7,
    Voucher = 8,
    Boss = 9,
    Standard = 10,
    Charm = 11,
    Meteor = 12,
    Buffoon = 13,
    Handy = 14,
    Garbage = 15,
    Ethereal = 16,
    Coupon = 17,
    Double = 18,
    Juggle = 19,
    D6 = 20,
    TopUp = 21,
    Speed = 22,
    Orbital = 23,
    Economy = 24,
}

impl PokerHand {
    pub(crate) const DISCARD_HANDS: [PokerHand; 11] = [
        PokerHand::Pair,
        PokerHand::ThreeOfAKind,
        PokerHand::FourOfAKind,
        PokerHand::FiveOfAKind,
        PokerHand::TwoPair,
        PokerHand::Straight,
        PokerHand::Flush,
        PokerHand::FullHouse,
        PokerHand::StraightFlush,
        PokerHand::FlushHouse,
        PokerHand::FlushFive,
    ];
}

impl Rank {
    pub fn base_chips(&self) -> u8 {
        match self {
            Rank::King | Rank::Queen | Rank::Jack => 10,
            Rank::Ace => 11,
            _ => *self as u8,
        }
    }

    pub fn is_face_card(&self) -> bool {
        matches!(self, Rank::King | Rank::Queen | Rank::Jack)
    }

    pub fn is_low_card(&self) -> bool {
        matches!(self, Rank::Two | Rank::Three | Rank::Four | Rank::Five)
    }
}

pub const ALL_RANKS: [Rank; 13] = [
    Rank::Two,
    Rank::Three,
    Rank::Four,
    Rank::Five,
    Rank::Six,
    Rank::Seven,
    Rank::Eight,
    Rank::Nine,
    Rank::Ten,
    Rank::Jack,
    Rank::Queen,
    Rank::King,
    Rank::Ace,
];

pub const ALL_SUITS: [Suit; 4] = [Suit::Hearts, Suit::Diamonds, Suit::Clubs, Suit::Spades];
