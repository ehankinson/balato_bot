// Joker-specific types live here so the shared game enums remain independent of Joker implementation details.

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub(crate) enum JokerKind {
    Joker = 1,
    Greedy = 2,
    Lusty = 3,
    Wrathful = 4,
    Gluttonous = 5,
    Jolly = 6,
    Zany = 7,
    Mad = 8,
    Crazy = 9,
    Droll = 10,
    Sly = 11,
    Wily = 12,
    Clever = 13,
    Devious = 14,
    Crafty = 15,
    Half = 16,
    Stencil = 17,
    FourFingers = 18,
    Mime = 19,
    CreditCard = 20,
    CeremonialDagger = 21,
    Banner = 22,
    MysticSummit = 23,
    Marble = 24,
    Loyalty = 25,
    EightBall = 26,
    Misprint = 27,
    Dusk = 28,
    RaisedFist = 29,
    ChaosTheClown = 30,
    Fibonacci = 31,
    Steel = 32,
    ScaryFace = 33,
    Abstract = 34,
    DelayedGratification = 35,
    Hack = 36,
    Pareidolia = 37,
    GrosMichel = 38,
    EvenSteven = 39,
    OddTodd = 40,
    Scholar = 41,
    BusinessCard = 42,
    Supernova = 43,
    RideTheBus = 44,
    Space = 45,
    Egg = 46,
    Burglar = 47,
    Blackboard = 48,
    Runner = 49,
    IceCream = 50,
    DNA = 51,
    Splash = 52,
    Blue = 53,
    SixthSense = 54,
    Constellation = 55,
    Hiker = 56,
    Faceless = 57,
    Green = 58,
    Superposition = 59,
    ToDoList = 60,
    Cavendish = 61,
    CardSharp = 62,
    RedCard = 63,
    Madness = 64,
    SquareJoker = 65,
    Seance = 66,
    RiffRaff = 67,
    Vampire = 68,
    Shortcut = 69,
    Hologram = 70,
    Vagabond = 71,
    Baron = 72,
    Cloud9 = 73,
    Rocket = 74,
    Obelisk = 75,
    MidasMask = 76,
    Luchador = 77,
    Photograph = 78,
    GiftCard = 79,
    TurtleBean = 80,
    Erosion = 81,
    ReservedParking = 82,
    MailInRebate = 83,
    ToTheMoon = 84,
    Hallucination = 85,
    FortuneTeller = 86,
    Juggler = 87,
    Drunkard = 88,
    Stone = 89,
    Golden = 90,
    LuckyCat = 91,
    BaseballCard = 92,
    Bull = 93,
    DietCola = 94,
    TradingCard = 95,
    FlashCard = 96,
    Popcorn = 97,
    SpareTrousers = 98,
    Ancient = 99,
    Ramen = 100,
    WalkieTalkie = 101,
    Seltzer = 102,
    Castle = 103,
    SmileyFace = 104,
    Campfire = 105,
    GoldenTicket = 106,
    MrBones = 107,
    Acrobat = 108,
    SockAndBuskin = 109,
    Swashbuckler = 110,
    Troubadour = 111,
    Certificate = 112,
    Smeared = 113,
    Throwback = 114,
    HangingChad = 115,
    RoughGem = 116,
    Bloodstone = 117,
    Arrowhead = 118,
    OnyxAgate = 119,
    Glass = 120,
    Showman = 121,
    FlowerPot = 122,
    Blueprint = 123,
    Wee = 124,
    MerryAndy = 125,
    OopsAll6s = 126,
    Idol = 127,
    SeeingDouble = 128,
    Matador = 129,
    HitTheRoad = 130,
    Duo = 131,
    Trio = 132,
    Family = 133,
    Order = 134,
    Tribe = 135,
    Stuntman = 136,
    Invisible = 137,
    Brainstorm = 138,
    Satellite = 139,
    ShootTheMoon = 140,
    DriversLicense = 141,
    Cartomancer = 142,
    Astronomer = 143,
    BurntJoker = 144,
    Bootstraps = 145,
    Canio = 146,
    Triboulet = 147,
    Yorick = 148,
    Chicot = 149,
    Perkeo = 150,
}

impl JokerKind {
    /// Blueprint/Brainstorm compatibility for the 150-Joker ruleset.
    /// Source: https://balatrogame.fandom.com/wiki/Blueprint#Incompatible_Jokers
    pub(crate) fn copy_compatible(self) -> bool {
        !matches!(
            self,
            Self::Astronomer
                | Self::ChaosTheClown
                | Self::Chicot
                | Self::Cloud9
                | Self::CreditCard
                | Self::DelayedGratification
                | Self::Drunkard
                | Self::Egg
                | Self::FourFingers
                | Self::GiftCard
                | Self::Golden
                | Self::Invisible
                | Self::Juggler
                | Self::MerryAndy
                | Self::MidasMask
                | Self::MrBones
                | Self::OopsAll6s
                | Self::Pareidolia
                | Self::Rocket
                | Self::Satellite
                | Self::Shortcut
                | Self::Showman
                | Self::SixthSense
                | Self::Smeared
                | Self::Splash
                | Self::ToTheMoon
                | Self::TradingCard
                | Self::Troubadour
                | Self::TurtleBean
        )
    }

    pub(crate) const ALL: [JokerKind; 150] = [
        JokerKind::Joker,
        JokerKind::Greedy,
        JokerKind::Lusty,
        JokerKind::Wrathful,
        JokerKind::Gluttonous,
        JokerKind::Jolly,
        JokerKind::Zany,
        JokerKind::Mad,
        JokerKind::Crazy,
        JokerKind::Droll,
        JokerKind::Sly,
        JokerKind::Wily,
        JokerKind::Clever,
        JokerKind::Devious,
        JokerKind::Crafty,
        JokerKind::Half,
        JokerKind::Stencil,
        JokerKind::FourFingers,
        JokerKind::Mime,
        JokerKind::CreditCard,
        JokerKind::CeremonialDagger,
        JokerKind::Banner,
        JokerKind::MysticSummit,
        JokerKind::Marble,
        JokerKind::Loyalty,
        JokerKind::EightBall,
        JokerKind::Misprint,
        JokerKind::Dusk,
        JokerKind::RaisedFist,
        JokerKind::ChaosTheClown,
        JokerKind::Fibonacci,
        JokerKind::Steel,
        JokerKind::ScaryFace,
        JokerKind::Abstract,
        JokerKind::DelayedGratification,
        JokerKind::Hack,
        JokerKind::Pareidolia,
        JokerKind::GrosMichel,
        JokerKind::EvenSteven,
        JokerKind::OddTodd,
        JokerKind::Scholar,
        JokerKind::BusinessCard,
        JokerKind::Supernova,
        JokerKind::RideTheBus,
        JokerKind::Space,
        JokerKind::Egg,
        JokerKind::Burglar,
        JokerKind::Blackboard,
        JokerKind::Runner,
        JokerKind::IceCream,
        JokerKind::DNA,
        JokerKind::Splash,
        JokerKind::Blue,
        JokerKind::SixthSense,
        JokerKind::Constellation,
        JokerKind::Hiker,
        JokerKind::Faceless,
        JokerKind::Green,
        JokerKind::Superposition,
        JokerKind::ToDoList,
        JokerKind::Cavendish,
        JokerKind::CardSharp,
        JokerKind::RedCard,
        JokerKind::Madness,
        JokerKind::SquareJoker,
        JokerKind::Seance,
        JokerKind::RiffRaff,
        JokerKind::Vampire,
        JokerKind::Shortcut,
        JokerKind::Hologram,
        JokerKind::Vagabond,
        JokerKind::Baron,
        JokerKind::Cloud9,
        JokerKind::Rocket,
        JokerKind::Obelisk,
        JokerKind::MidasMask,
        JokerKind::Luchador,
        JokerKind::Photograph,
        JokerKind::GiftCard,
        JokerKind::TurtleBean,
        JokerKind::Erosion,
        JokerKind::ReservedParking,
        JokerKind::MailInRebate,
        JokerKind::ToTheMoon,
        JokerKind::Hallucination,
        JokerKind::FortuneTeller,
        JokerKind::Juggler,
        JokerKind::Drunkard,
        JokerKind::Stone,
        JokerKind::Golden,
        JokerKind::LuckyCat,
        JokerKind::BaseballCard,
        JokerKind::Bull,
        JokerKind::DietCola,
        JokerKind::TradingCard,
        JokerKind::FlashCard,
        JokerKind::Popcorn,
        JokerKind::SpareTrousers,
        JokerKind::Ancient,
        JokerKind::Ramen,
        JokerKind::WalkieTalkie,
        JokerKind::Seltzer,
        JokerKind::Castle,
        JokerKind::SmileyFace,
        JokerKind::Campfire,
        JokerKind::GoldenTicket,
        JokerKind::MrBones,
        JokerKind::Acrobat,
        JokerKind::SockAndBuskin,
        JokerKind::Swashbuckler,
        JokerKind::Troubadour,
        JokerKind::Certificate,
        JokerKind::Smeared,
        JokerKind::Throwback,
        JokerKind::HangingChad,
        JokerKind::RoughGem,
        JokerKind::Bloodstone,
        JokerKind::Arrowhead,
        JokerKind::OnyxAgate,
        JokerKind::Glass,
        JokerKind::Showman,
        JokerKind::FlowerPot,
        JokerKind::Blueprint,
        JokerKind::Wee,
        JokerKind::MerryAndy,
        JokerKind::OopsAll6s,
        JokerKind::Idol,
        JokerKind::SeeingDouble,
        JokerKind::Matador,
        JokerKind::HitTheRoad,
        JokerKind::Duo,
        JokerKind::Trio,
        JokerKind::Family,
        JokerKind::Order,
        JokerKind::Tribe,
        JokerKind::Stuntman,
        JokerKind::Invisible,
        JokerKind::Brainstorm,
        JokerKind::Satellite,
        JokerKind::ShootTheMoon,
        JokerKind::DriversLicense,
        JokerKind::Cartomancer,
        JokerKind::Astronomer,
        JokerKind::BurntJoker,
        JokerKind::Bootstraps,
        JokerKind::Canio,
        JokerKind::Triboulet,
        JokerKind::Yorick,
        JokerKind::Chicot,
        JokerKind::Perkeo,
    ];
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum JokerEdition {
    None = 0,
    Foil = 1,
    Holographic = 2,
    Polychrome = 3,
    Negative = 4,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub(crate) enum JokerTrigger {
    OnPlayedCard = 1,
    AfterHand = 2,
    OnHeldCard = 3,
    BeforePlayedCards = 4,
    None = 5,
    OnHandPlayed = 6,
    OnDiscard = 8,
    OnBossBlindAbility = 9,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub(crate) enum RetriggerTarget {
    FaceCards = 1,
    LowCards = 2,
    FinalHand = 3,
    FirstCard = 4,
    PlayedCards = 5,
    HeldCards = 6,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum CopyType {
    Right = 1,
    LeftMost = 2,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum JokerRarity {
    Common = 0,
    Uncommon = 1,
    Rare = 2,
    Legendary = 3,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub(crate) enum GenerateType {
    None = 0,
    StoneCard = 1,
    Tarot = 2,
    Spectral = 3,
    SealCard = 4,
    FirstPlayedCard = 5,
    Joker = 6,
    Gold = 7,
    PlanetLevel = 8,
    NegativeConsumable = 9,
}

#[derive(Clone)]
pub(crate) enum GameStateModifications {
    AllFaces = 0,
    PlayAllCards = 1,
    AllowStraightGaps = 2,
    DisableBossBlind = 3,
    PreventDeath = 4,
    DoubleSuit = 5,
    DuplicateJokers = 6,
    DoubleProbability = 7,
    FreePlanetCards = 8,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub(crate) enum JokerUpdate {
    JokerOrderChanged = 0,
    BlindSelected = 1,
    DuringBlind = 2,
    CardEnhancementAdded = 3,
    BlindCompleted = 4,
    HandCompleted = 5,
    DeckChanged = 6,
    PlanetCardUsed = 7,
    BoosterPackSkipped = 8,
    BoosterPackOpened = 9,
    TarotCardUsed = 10,
    JokerSold = 11,
    CardsDiscarded = 12,
    AfterCardSold = 13,
    AfterPlayerDeath = 14,
    CardDestroyed = 15,
    RoundStarted = 16,
    RoundCompleted = 17,
    DiscardActionCompleted = 18,
    LuckyCardSucceeded = 19,
    ShopRerolled = 20,
    BossBlindCompleted = 21,
    ShopClosed = 22,
}

impl JokerUpdate {
    pub(crate) const fn mask(self) -> u32 {
        1_u32 << (self as u32)
    }
}

/// Passive rules supplied by the currently owned, non-debuffed Jokers.
#[derive(Debug, Clone, Copy)]
pub(crate) struct JokerModifiers {
    pub(crate) all_faces: bool,
    pub(crate) splash: bool,
    pub(crate) straight_gaps: bool,
    pub(crate) smeared_suits: bool,
    pub(crate) showman: bool,
    pub(crate) free_planets: bool,
    pub(crate) disable_boss: bool,
    pub(crate) probability_multiplier: f32,
}

impl Default for JokerModifiers {
    fn default() -> Self {
        Self {
            all_faces: false,
            splash: false,
            straight_gaps: false,
            smeared_suits: false,
            showman: false,
            free_planets: false,
            disable_boss: false,
            probability_multiplier: 1.0,
        }
    }
}

impl JokerModifiers {
    pub(crate) fn is_face(self, card: &crate::core::card::Card) -> bool {
        self.all_faces
            || (card.enhancement() != crate::core::enums::Enhancement::Stone && card.is_face_card())
    }

    pub(crate) fn card_has_suit(
        self,
        card: &crate::core::card::Card,
        required: crate::core::enums::Suit,
    ) -> bool {
        use crate::core::enums::Enhancement;
        !card.is_debuffed()
            && card.enhancement() != Enhancement::Stone
            && (card.enhancement() == Enhancement::Wild || self.suits_match(card.suit(), required))
    }

    // Hand-wide suit checks retain a debuffed card's printed suit, but not its Wild effect.
    pub(crate) fn card_has_flush_suit(
        self,
        card: &crate::core::card::Card,
        required: crate::core::enums::Suit,
    ) -> bool {
        use crate::core::enums::Enhancement;
        card.enhancement() != Enhancement::Stone
            && ((card.enhancement() == Enhancement::Wild && !card.is_debuffed())
                || self.suits_match(card.suit(), required))
    }

    pub(crate) fn suits_match(
        self,
        actual: crate::core::enums::Suit,
        required: crate::core::enums::Suit,
    ) -> bool {
        actual == required || (self.smeared_suits && (actual as u8 / 2 == required as u8 / 2))
    }
}
