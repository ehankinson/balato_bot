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
