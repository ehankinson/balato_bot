//! Frozen categorical mappings for observation schema v1.
use crate::core::enums::*;
use crate::core::joker_types::{JokerEdition, JokerKind, JokerRarity};
use crate::core::pack::{BoosterPackKind, BoosterPackSize};

pub(super) fn poker_hand(value: PokerHand) -> i32 {
    match value {
        PokerHand::HighCard => 1,
        PokerHand::Pair => 2,
        PokerHand::ThreeOfAKind => 3,
        PokerHand::FourOfAKind => 4,
        PokerHand::FiveOfAKind => 5,
        PokerHand::TwoPair => 6,
        PokerHand::Straight => 7,
        PokerHand::Flush => 8,
        PokerHand::FullHouse => 9,
        PokerHand::StraightFlush => 10,
        PokerHand::FlushHouse => 11,
        PokerHand::FlushFive => 12,
    }
}

pub(super) fn rank(value: Rank) -> i32 {
    match value {
        Rank::Two => 2,
        Rank::Three => 3,
        Rank::Four => 4,
        Rank::Five => 5,
        Rank::Six => 6,
        Rank::Seven => 7,
        Rank::Eight => 8,
        Rank::Nine => 9,
        Rank::Ten => 10,
        Rank::Jack => 11,
        Rank::Queen => 12,
        Rank::King => 13,
        Rank::Ace => 14,
    }
}

pub(super) fn suit(value: Suit) -> i32 {
    match value {
        Suit::Hearts => 0,
        Suit::Diamonds => 1,
        Suit::Clubs => 2,
        Suit::Spades => 3,
    }
}

pub(super) fn enhancement(value: Enhancement) -> i32 {
    match value {
        Enhancement::None => 0,
        Enhancement::Stone => 1,
        Enhancement::Gold => 2,
        Enhancement::Bonus => 3,
        Enhancement::Mult => 4,
        Enhancement::Wild => 5,
        Enhancement::Lucky => 6,
        Enhancement::Glass => 7,
        Enhancement::Steel => 8,
    }
}

pub(super) fn seal(value: Seal) -> i32 {
    match value {
        Seal::None => 0,
        Seal::Gold => 1,
        Seal::Purple => 2,
        Seal::Blue => 3,
        Seal::Red => 4,
    }
}

pub(super) fn edition(value: Edition) -> i32 {
    match value {
        Edition::None => 0,
        Edition::Foil => 1,
        Edition::Holographic => 2,
        Edition::Polychrome => 3,
    }
}

pub(super) fn decks(value: Decks) -> i32 {
    match value {
        Decks::Red => 1,
        Decks::Blue => 2,
        Decks::Yellow => 3,
        Decks::Green => 4,
        Decks::Black => 5,
        Decks::Magic => 6,
        Decks::Nebula => 7,
        Decks::Ghost => 8,
        Decks::Abandoned => 9,
        Decks::Checkered => 10,
        Decks::Zodiac => 11,
        Decks::Painted => 12,
        Decks::Anaglyph => 13,
        Decks::Plasma => 14,
        Decks::Erratic => 15,
    }
}

pub(super) fn stakes(value: Stakes) -> i32 {
    match value {
        Stakes::White => 1,
        Stakes::Red => 2,
        Stakes::Green => 3,
        Stakes::Black => 4,
        Stakes::Blue => 5,
        Stakes::Purple => 6,
        Stakes::Orange => 7,
        Stakes::Gold => 8,
    }
}

pub(super) fn boss_blinds(value: BossBlinds) -> i32 {
    match value {
        BossBlinds::Hook => 1,
        BossBlinds::Ox => 2,
        BossBlinds::House => 3,
        BossBlinds::Wall => 4,
        BossBlinds::Wheel => 5,
        BossBlinds::Arm => 6,
        BossBlinds::Club => 7,
        BossBlinds::Fish => 8,
        BossBlinds::Psychich => 9,
        BossBlinds::Goad => 10,
        BossBlinds::Water => 11,
        BossBlinds::Window => 12,
        BossBlinds::Manacle => 13,
        BossBlinds::Eye => 14,
        BossBlinds::Mouth => 15,
        BossBlinds::Plante => 16,
        BossBlinds::Serpent => 17,
        BossBlinds::Pillar => 18,
        BossBlinds::Needle => 19,
        BossBlinds::Head => 20,
        BossBlinds::Tooth => 21,
        BossBlinds::Flint => 22,
        BossBlinds::Mark => 23,
        BossBlinds::AmberAcorn => 24,
        BossBlinds::VerdantLeaf => 25,
        BossBlinds::VioletVessel => 26,
        BossBlinds::CrimsonHeart => 27,
        BossBlinds::CeruleanBell => 28,
    }
}

pub(super) fn skip_tag(value: SkipTag) -> i32 {
    match value {
        SkipTag::Uncommon => 1,
        SkipTag::Rare => 2,
        SkipTag::Negative => 3,
        SkipTag::Foil => 4,
        SkipTag::Holographic => 5,
        SkipTag::Polychrome => 6,
        SkipTag::Investment => 7,
        SkipTag::Voucher => 8,
        SkipTag::Boss => 9,
        SkipTag::Standard => 10,
        SkipTag::Charm => 11,
        SkipTag::Meteor => 12,
        SkipTag::Buffoon => 13,
        SkipTag::Handy => 14,
        SkipTag::Garbage => 15,
        SkipTag::Ethereal => 16,
        SkipTag::Coupon => 17,
        SkipTag::Double => 18,
        SkipTag::Juggle => 19,
        SkipTag::D6 => 20,
        SkipTag::TopUp => 21,
        SkipTag::Speed => 22,
        SkipTag::Orbital => 23,
        SkipTag::Economy => 24,
    }
}

pub(super) fn vouchers(value: Vouchers) -> i32 {
    match value {
        Vouchers::Overstock => 1,
        Vouchers::OverstockPlus => 2,
        Vouchers::ClearanceSale => 3,
        Vouchers::Liquidation => 4,
        Vouchers::Hone => 5,
        Vouchers::GlowUp => 6,
        Vouchers::RerollSurplus => 7,
        Vouchers::RerollGlut => 8,
        Vouchers::CrystalBall => 9,
        Vouchers::OmenGlobe => 10,
        Vouchers::Telescope => 11,
        Vouchers::Observatory => 12,
        Vouchers::Grabber => 13,
        Vouchers::NachoTong => 14,
        Vouchers::Wasteful => 15,
        Vouchers::Recyclomancy => 16,
        Vouchers::TarotMerchant => 17,
        Vouchers::TarotTycoon => 18,
        Vouchers::PlanetMerchant => 19,
        Vouchers::PlanetTycoon => 20,
        Vouchers::SeedMoney => 21,
        Vouchers::MoneyTree => 22,
        Vouchers::Blank => 23,
        Vouchers::Antimatter => 24,
        Vouchers::MagicTrick => 25,
        Vouchers::Illusion => 26,
        Vouchers::Hieroglyph => 27,
        Vouchers::Petroglpyh => 28,
        Vouchers::DirectorsCut => 29,
        Vouchers::Retcon => 30,
        Vouchers::PaintBrush => 31,
        Vouchers::Palette => 32,
    }
}

pub(super) fn tarot(value: Tarot) -> i32 {
    match value {
        Tarot::Fool => 1,
        Tarot::Magician => 2,
        Tarot::HighPriestess => 3,
        Tarot::Emperor => 4,
        Tarot::Empress => 5,
        Tarot::Hierophant => 6,
        Tarot::Lovers => 7,
        Tarot::Chariot => 8,
        Tarot::Justice => 9,
        Tarot::Hermit => 10,
        Tarot::WheelOfFortune => 11,
        Tarot::Strength => 12,
        Tarot::HangedMan => 13,
        Tarot::Death => 14,
        Tarot::Temperance => 15,
        Tarot::Devil => 16,
        Tarot::Tower => 17,
        Tarot::Star => 18,
        Tarot::Moon => 19,
        Tarot::Sun => 20,
        Tarot::Judgement => 21,
        Tarot::World => 22,
    }
}

pub(super) fn spectral(value: Spectral) -> i32 {
    match value {
        Spectral::Familiar => 1,
        Spectral::Grim => 2,
        Spectral::Incantation => 3,
        Spectral::Talisman => 4,
        Spectral::Aura => 5,
        Spectral::Wraith => 6,
        Spectral::Sigil => 7,
        Spectral::Ouija => 8,
        Spectral::Ectoplasm => 9,
        Spectral::Immolate => 10,
        Spectral::Ankh => 11,
        Spectral::DejaVu => 12,
        Spectral::Hex => 13,
        Spectral::Trance => 14,
        Spectral::Medium => 15,
        Spectral::Cryptid => 16,
        Spectral::Soul => 17,
        Spectral::BlackHole => 18,
    }
}

pub(super) fn planet(value: Planet) -> i32 {
    match value {
        Planet::Pluto => 1,
        Planet::Mercury => 2,
        Planet::Uranus => 3,
        Planet::Venus => 4,
        Planet::Saturn => 5,
        Planet::Jupiter => 6,
        Planet::Earth => 7,
        Planet::Mars => 8,
        Planet::Neptune => 9,
        Planet::PlanetX => 10,
        Planet::Ceres => 11,
        Planet::Eris => 12,
    }
}

pub(super) fn joker_kind(value: JokerKind) -> i32 {
    match value {
        JokerKind::Joker => 1,
        JokerKind::Greedy => 2,
        JokerKind::Lusty => 3,
        JokerKind::Wrathful => 4,
        JokerKind::Gluttonous => 5,
        JokerKind::Jolly => 6,
        JokerKind::Zany => 7,
        JokerKind::Mad => 8,
        JokerKind::Crazy => 9,
        JokerKind::Droll => 10,
        JokerKind::Sly => 11,
        JokerKind::Wily => 12,
        JokerKind::Clever => 13,
        JokerKind::Devious => 14,
        JokerKind::Crafty => 15,
        JokerKind::Half => 16,
        JokerKind::Stencil => 17,
        JokerKind::FourFingers => 18,
        JokerKind::Mime => 19,
        JokerKind::CreditCard => 20,
        JokerKind::CeremonialDagger => 21,
        JokerKind::Banner => 22,
        JokerKind::MysticSummit => 23,
        JokerKind::Marble => 24,
        JokerKind::Loyalty => 25,
        JokerKind::EightBall => 26,
        JokerKind::Misprint => 27,
        JokerKind::Dusk => 28,
        JokerKind::RaisedFist => 29,
        JokerKind::ChaosTheClown => 30,
        JokerKind::Fibonacci => 31,
        JokerKind::Steel => 32,
        JokerKind::ScaryFace => 33,
        JokerKind::Abstract => 34,
        JokerKind::DelayedGratification => 35,
        JokerKind::Hack => 36,
        JokerKind::Pareidolia => 37,
        JokerKind::GrosMichel => 38,
        JokerKind::EvenSteven => 39,
        JokerKind::OddTodd => 40,
        JokerKind::Scholar => 41,
        JokerKind::BusinessCard => 42,
        JokerKind::Supernova => 43,
        JokerKind::RideTheBus => 44,
        JokerKind::Space => 45,
        JokerKind::Egg => 46,
        JokerKind::Burglar => 47,
        JokerKind::Blackboard => 48,
        JokerKind::Runner => 49,
        JokerKind::IceCream => 50,
        JokerKind::DNA => 51,
        JokerKind::Splash => 52,
        JokerKind::Blue => 53,
        JokerKind::SixthSense => 54,
        JokerKind::Constellation => 55,
        JokerKind::Hiker => 56,
        JokerKind::Faceless => 57,
        JokerKind::Green => 58,
        JokerKind::Superposition => 59,
        JokerKind::ToDoList => 60,
        JokerKind::Cavendish => 61,
        JokerKind::CardSharp => 62,
        JokerKind::RedCard => 63,
        JokerKind::Madness => 64,
        JokerKind::SquareJoker => 65,
        JokerKind::Seance => 66,
        JokerKind::RiffRaff => 67,
        JokerKind::Vampire => 68,
        JokerKind::Shortcut => 69,
        JokerKind::Hologram => 70,
        JokerKind::Vagabond => 71,
        JokerKind::Baron => 72,
        JokerKind::Cloud9 => 73,
        JokerKind::Rocket => 74,
        JokerKind::Obelisk => 75,
        JokerKind::MidasMask => 76,
        JokerKind::Luchador => 77,
        JokerKind::Photograph => 78,
        JokerKind::GiftCard => 79,
        JokerKind::TurtleBean => 80,
        JokerKind::Erosion => 81,
        JokerKind::ReservedParking => 82,
        JokerKind::MailInRebate => 83,
        JokerKind::ToTheMoon => 84,
        JokerKind::Hallucination => 85,
        JokerKind::FortuneTeller => 86,
        JokerKind::Juggler => 87,
        JokerKind::Drunkard => 88,
        JokerKind::Stone => 89,
        JokerKind::Golden => 90,
        JokerKind::LuckyCat => 91,
        JokerKind::BaseballCard => 92,
        JokerKind::Bull => 93,
        JokerKind::DietCola => 94,
        JokerKind::TradingCard => 95,
        JokerKind::FlashCard => 96,
        JokerKind::Popcorn => 97,
        JokerKind::SpareTrousers => 98,
        JokerKind::Ancient => 99,
        JokerKind::Ramen => 100,
        JokerKind::WalkieTalkie => 101,
        JokerKind::Seltzer => 102,
        JokerKind::Castle => 103,
        JokerKind::SmileyFace => 104,
        JokerKind::Campfire => 105,
        JokerKind::GoldenTicket => 106,
        JokerKind::MrBones => 107,
        JokerKind::Acrobat => 108,
        JokerKind::SockAndBuskin => 109,
        JokerKind::Swashbuckler => 110,
        JokerKind::Troubadour => 111,
        JokerKind::Certificate => 112,
        JokerKind::Smeared => 113,
        JokerKind::Throwback => 114,
        JokerKind::HangingChad => 115,
        JokerKind::RoughGem => 116,
        JokerKind::Bloodstone => 117,
        JokerKind::Arrowhead => 118,
        JokerKind::OnyxAgate => 119,
        JokerKind::Glass => 120,
        JokerKind::Showman => 121,
        JokerKind::FlowerPot => 122,
        JokerKind::Blueprint => 123,
        JokerKind::Wee => 124,
        JokerKind::MerryAndy => 125,
        JokerKind::OopsAll6s => 126,
        JokerKind::Idol => 127,
        JokerKind::SeeingDouble => 128,
        JokerKind::Matador => 129,
        JokerKind::HitTheRoad => 130,
        JokerKind::Duo => 131,
        JokerKind::Trio => 132,
        JokerKind::Family => 133,
        JokerKind::Order => 134,
        JokerKind::Tribe => 135,
        JokerKind::Stuntman => 136,
        JokerKind::Invisible => 137,
        JokerKind::Brainstorm => 138,
        JokerKind::Satellite => 139,
        JokerKind::ShootTheMoon => 140,
        JokerKind::DriversLicense => 141,
        JokerKind::Cartomancer => 142,
        JokerKind::Astronomer => 143,
        JokerKind::BurntJoker => 144,
        JokerKind::Bootstraps => 145,
        JokerKind::Canio => 146,
        JokerKind::Triboulet => 147,
        JokerKind::Yorick => 148,
        JokerKind::Chicot => 149,
        JokerKind::Perkeo => 150,
    }
}

pub(super) fn joker_edition(value: JokerEdition) -> i32 {
    match value {
        JokerEdition::None => 0,
        JokerEdition::Foil => 1,
        JokerEdition::Holographic => 2,
        JokerEdition::Polychrome => 3,
        JokerEdition::Negative => 4,
    }
}

pub(super) fn joker_rarity(value: JokerRarity) -> i32 {
    match value {
        JokerRarity::Common => 0,
        JokerRarity::Uncommon => 1,
        JokerRarity::Rare => 2,
        JokerRarity::Legendary => 3,
    }
}

pub(super) fn pack_kind(value: BoosterPackKind) -> i32 {
    match value {
        BoosterPackKind::Standard => 0,
        BoosterPackKind::Arcana => 1,
        BoosterPackKind::Celestial => 2,
        BoosterPackKind::Buffoon => 3,
        BoosterPackKind::Spectral => 4,
    }
}

pub(super) fn pack_size(value: BoosterPackSize) -> i32 {
    match value {
        BoosterPackSize::Normal => 0,
        BoosterPackSize::Jumbo => 1,
        BoosterPackSize::Mega => 2,
    }
}

pub(super) fn consumable(value: Consumable) -> [i32; 2] {
    match value {
        Consumable::Tarot(value) => [1, tarot(value)],
        Consumable::Planet(value) => [2, planet(value)],
        Consumable::Spectral(value) => [3, spectral(value)],
    }
}
