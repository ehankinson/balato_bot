use crate::core::enums::{
    Enhancement, PokerHand, Rank, Suit,
};
use crate::core::joker_types::{
    CopyType, GameStateModifications, GenerateType, JokerEdition, JokerKind, JokerRarity,
    JokerTrigger, RetriggerTarget,
};

#[path = "joker_events.rs"]
mod events;
pub use events::{Jokers, TriggerEvent};

pub struct Joker {
    kind: JokerKind,
    structure: JokerStructure,
    edition: JokerEdition,
    trigger: JokerTrigger,
    rarity: JokerRarity,
    price: u8,
    debuffed: bool,
    update_mask: u32,
}

pub(crate) enum JokerData {
    Scoring(JokerScoring),
    Econ(JokerEcon),
    GameState(JokerGameState),
    Retrigger(JokerRetrigger),
    Generate(JokerGenerate),
}

pub(crate) enum JokerStructure {
    Normal(JokerData),
    Copy(CopyType),
}

impl Joker {
    pub(crate) fn create_joker(joker_kind: JokerKind) -> Joker {
        let (price, rarity, trigger, structure) = match joker_kind {
            JokerKind::Joker => {
                let data = JokerScoring {
                    add_mult: 4,
                    ..JokerScoring::default()
                };

                (
                    2,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Greedy => {
                let data = JokerScoring {
                    add_mult: 3,
                    suit: Some(Suit::Diamonds),
                    ..JokerScoring::default()
                };

                (
                    5,
                    JokerRarity::Common,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Lusty => {
                let data = JokerScoring {
                    add_mult: 3,
                    suit: Some(Suit::Hearts),
                    ..JokerScoring::default()
                };

                (
                    5,
                    JokerRarity::Common,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Wrathful => {
                let data = JokerScoring {
                    add_mult: 3,
                    suit: Some(Suit::Spades),
                    ..JokerScoring::default()
                };

                (
                    5,
                    JokerRarity::Common,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Gluttonous => {
                let data = JokerScoring {
                    add_mult: 3,
                    suit: Some(Suit::Clubs),
                    ..JokerScoring::default()
                };

                (
                    5,
                    JokerRarity::Common,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Jolly => {
                let data = JokerScoring {
                    add_mult: 8,
                    poker_hand: Some(PokerHand::Pair),
                    ..JokerScoring::default()
                };

                (
                    3,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Zany => {
                let data = JokerScoring {
                    add_mult: 12,
                    poker_hand: Some(PokerHand::ThreeOfAKind),
                    ..JokerScoring::default()
                };

                (
                    4,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Mad => {
                let data = JokerScoring {
                    add_mult: 10,
                    poker_hand: Some(PokerHand::TwoPair),
                    ..JokerScoring::default()
                };

                (
                    4,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Crazy => {
                let data = JokerScoring {
                    add_mult: 12,
                    poker_hand: Some(PokerHand::Straight),
                    ..JokerScoring::default()
                };

                (
                    4,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Droll => {
                let data = JokerScoring {
                    add_mult: 10,
                    poker_hand: Some(PokerHand::Flush),
                    ..JokerScoring::default()
                };

                (
                    4,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Sly => {
                let data = JokerScoring {
                    chips: 50,
                    poker_hand: Some(PokerHand::Pair),
                    ..JokerScoring::default()
                };

                (
                    3,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Wily => {
                let data = JokerScoring {
                    chips: 100,
                    poker_hand: Some(PokerHand::ThreeOfAKind),
                    ..JokerScoring::default()
                };

                (
                    4,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Clever => {
                let data = JokerScoring {
                    chips: 80,
                    poker_hand: Some(PokerHand::TwoPair),
                    ..JokerScoring::default()
                };

                (
                    4,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Devious => {
                let data = JokerScoring {
                    chips: 100,
                    poker_hand: Some(PokerHand::Straight),
                    ..JokerScoring::default()
                };

                (
                    4,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Crafty => {
                let data = JokerScoring {
                    chips: 80,
                    poker_hand: Some(PokerHand::Flush),
                    ..JokerScoring::default()
                };

                (
                    4,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Half => {
                let data = JokerScoring {
                    add_mult: 20,
                    req_count: 3,
                    ..JokerScoring::default()
                };

                (
                    4,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Stencil => {
                let data = JokerScoring {
                    x_mult: 1.0,
                    ..JokerScoring::default()
                };

                (
                    8,
                    JokerRarity::Uncommon,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::FourFingers => {
                let data = JokerGameState {
                    straight_flush_size: 4,
                    ..JokerGameState::default()
                };

                (
                    7,
                    JokerRarity::Uncommon,
                    JokerTrigger::None,
                    JokerStructure::Normal(JokerData::GameState(data)),
                )
            }
            JokerKind::Mime => {
                let data = JokerRetrigger {
                    retrigger: 1,
                    target: RetriggerTarget::HeldCards,
                };

                (
                    5,
                    JokerRarity::Uncommon,
                    JokerTrigger::OnHeldCard,
                    JokerStructure::Normal(JokerData::Retrigger(data)),
                )
            }
            JokerKind::CreditCard => {
                let data = JokerGameState {
                    min_money: -20,
                    ..JokerGameState::default()
                };

                (
                    1,
                    JokerRarity::Common,
                    JokerTrigger::None,
                    JokerStructure::Normal(JokerData::GameState(data)),
                )
            }
            JokerKind::CeremonialDagger => {
                let data = JokerScoring {
                    add_mult: 0,
                    ..JokerScoring::default()
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Banner => {
                let data = JokerScoring {
                    chips: 0,
                    ..JokerScoring::default()
                };

                (
                    5,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::MysticSummit => {
                let data = JokerScoring {
                    add_mult: 0,
                    req_count: 0,
                    ..JokerScoring::default()
                };

                (
                    5,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Marble => {
                let data = JokerGenerate {
                    amount: 1,
                    generate_type: GenerateType::StoneCard,
                    ..JokerGenerate::default()
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::BeforePlayedCards,
                    JokerStructure::Normal(JokerData::Generate(data)),
                )
            }
            JokerKind::Loyalty => {
                let data = JokerScoring {
                    x_mult: 4.0,
                    ..JokerScoring::default()
                };

                (
                    5,
                    JokerRarity::Uncommon,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::EightBall => {
                let data = JokerGenerate {
                    amount: 1,
                    probability: 0.25,
                    generate_type: GenerateType::Tarot,
                    ..JokerGenerate::default()
                };

                (
                    5,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Generate(data)),
                )
            }
            JokerKind::Misprint => {
                let data = JokerScoring {
                    add_mult: 23,
                    ..JokerScoring::default()
                };

                (
                    4,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Dusk => {
                let data = JokerRetrigger {
                    retrigger: 1,
                    target: RetriggerTarget::FinalHand,
                };

                (
                    5,
                    JokerRarity::Uncommon,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Retrigger(data)),
                )
            }
            JokerKind::RaisedFist => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    5,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::ChaosTheClown => {
                let data = JokerGameState {
                    reroll_cost: 0,
                    ..JokerGameState::default()
                };

                (
                    4,
                    JokerRarity::Common,
                    JokerTrigger::None,
                    JokerStructure::Normal(JokerData::GameState(data)),
                )
            }
            JokerKind::Fibonacci => {
                let data = JokerScoring {
                    add_mult: 8,
                    ..JokerScoring::default()
                };

                (
                    8,
                    JokerRarity::Uncommon,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Steel => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    7,
                    JokerRarity::Uncommon,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::ScaryFace => {
                let data = JokerScoring {
                    chips: 30,
                    ..JokerScoring::default()
                };

                (
                    4,
                    JokerRarity::Common,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Abstract => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    4,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::DelayedGratification => {
                let data = JokerEcon {
                    money: 2,
                    ..JokerEcon::default()
                };

                (
                    4,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Econ(data)),
                )
            }
            JokerKind::Hack => {
                let data = JokerRetrigger {
                    retrigger: 1,
                    target: RetriggerTarget::LowCards,
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Retrigger(data)),
                )
            }
            JokerKind::Pareidolia => {
                let data = JokerGameState {
                    game_state_modification: Some(GameStateModifications::AllFaces),
                    ..JokerGameState::default()
                };

                (
                    4,
                    JokerRarity::Uncommon,
                    JokerTrigger::None,
                    JokerStructure::Normal(JokerData::GameState(data)),
                )
            }
            JokerKind::GrosMichel => {
                let data = JokerScoring {
                    add_mult: 15,
                    ..JokerScoring::default()
                };

                (
                    5,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::EvenSteven => {
                let data = JokerScoring {
                    add_mult: 4,
                    rank: Some(vec![
                        Rank::Two,
                        Rank::Four,
                        Rank::Six,
                        Rank::Eight,
                        Rank::Ten,
                    ]),
                    ..JokerScoring::default()
                };

                (
                    4,
                    JokerRarity::Common,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::OddTodd => {
                let data = JokerScoring {
                    chips: 31,
                    rank: Some(vec![
                        Rank::Ace,
                        Rank::Three,
                        Rank::Five,
                        Rank::Seven,
                        Rank::Nine,
                    ]),
                    ..JokerScoring::default()
                };

                (
                    4,
                    JokerRarity::Common,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Scholar => {
                let data = JokerScoring {
                    chips: 20,
                    add_mult: 4,
                    rank: Some(vec![Rank::Ace]),
                    ..JokerScoring::default()
                };

                (
                    4,
                    JokerRarity::Common,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::BusinessCard => {
                let data = JokerEcon {
                    money: 2,
                    probability: 0.5,
                    rank: Some(vec![Rank::King, Rank::Queen, Rank::Jack]),
                    ..JokerEcon::default()
                };

                (
                    4,
                    JokerRarity::Common,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Econ(data)),
                )
            }
            JokerKind::Supernova => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    5,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::RideTheBus => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    6,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Space => {
                let data = JokerGameState {
                    probability: 0.25,
                    ..JokerGameState::default()
                };

                (
                    5,
                    JokerRarity::Uncommon,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::GameState(data)),
                )
            }
            JokerKind::Egg => {
                let data = JokerEcon {
                    money: 3,
                    ..JokerEcon::default()
                };

                (
                    4,
                    JokerRarity::Common,
                    JokerTrigger::EndOfBlind,
                    JokerStructure::Normal(JokerData::Econ(data)),
                )
            }
            JokerKind::Burglar => {
                let data = JokerGameState {
                    hands: 3,
                    ..JokerGameState::default()
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::StartOfBlind,
                    JokerStructure::Normal(JokerData::GameState(data)),
                )
            }
            JokerKind::Blackboard => {
                let data = JokerScoring {
                    x_mult: 3.0,
                    ..JokerScoring::default()
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Runner => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    5,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::IceCream => {
                let data = JokerScoring {
                    chips: 100,
                    ..JokerScoring::default()
                };

                (
                    5,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::DNA => {
                let data = JokerGenerate {
                    amount: 1,
                    generate_type: GenerateType::FirstPlayedCard,
                    ..JokerGenerate::default()
                };

                (
                    8,
                    JokerRarity::Rare,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Generate(data)),
                )
            }
            JokerKind::Splash => {
                let data = JokerGameState {
                    game_state_modification: Some(GameStateModifications::PlayAllCards),
                    ..JokerGameState::default()
                };

                (
                    6,
                    JokerRarity::Common,
                    JokerTrigger::None,
                    JokerStructure::Normal(JokerData::GameState(data)),
                )
            }
            JokerKind::Blue => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    5,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::SixthSense => {
                let data = JokerGenerate {
                    amount: 1,
                    generate_type: GenerateType::Spectral,
                    ..JokerGenerate::default()
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Generate(data)),
                )
            }
            JokerKind::Constellation => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Hiker => {
                let data = JokerScoring {
                    chips: 5,
                    ..JokerScoring::default()
                };

                (
                    5,
                    JokerRarity::Uncommon,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Faceless => {
                let data = JokerEcon {
                    money: 5,
                    required_count: 3,
                    rank: Some(vec![Rank::King, Rank::Queen, Rank::Jack]),
                    ..JokerEcon::default()
                };

                (
                    4,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Econ(data)),
                )
            }
            JokerKind::Green => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    4,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Superposition => {
                let data = JokerGenerate {
                    amount: 1,
                    generate_type: GenerateType::Tarot,
                    rank: Some(Rank::Ace),
                    poker_hand: Some(PokerHand::Straight),
                    ..JokerGenerate::default()
                };

                (
                    4,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Generate(data)),
                )
            }
            JokerKind::ToDoList => {
                let data = JokerEcon {
                    money: 5,
                    ..JokerEcon::default()
                };

                (
                    4,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Econ(data)),
                )
            }
            JokerKind::Cavendish => {
                let data = JokerScoring {
                    x_mult: 3.0,
                    ..JokerScoring::default()
                };

                (
                    4,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::CardSharp => {
                let data = JokerScoring {
                    x_mult: 3.0,
                    ..JokerScoring::default()
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::RedCard => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    5,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Madness => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    7,
                    JokerRarity::Uncommon,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::SquareJoker => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    4,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Seance => {
                let data = JokerGenerate {
                    amount: 1,
                    generate_type: GenerateType::Spectral,
                    ..JokerGenerate::default()
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Generate(data)),
                )
            }
            JokerKind::RiffRaff => {
                let data = JokerGenerate {
                    amount: 2,
                    generate_type: GenerateType::Joker,
                    ..JokerGenerate::default()
                };

                (
                    6,
                    JokerRarity::Common,
                    JokerTrigger::BeforePlayedCards,
                    JokerStructure::Normal(JokerData::Generate(data)),
                )
            }
            JokerKind::Vampire => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    7,
                    JokerRarity::Uncommon,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Shortcut => {
                let data = JokerGameState {
                    game_state_modification: Some(GameStateModifications::AllowStraightGaps),
                    ..JokerGameState::default()
                };

                (
                    7,
                    JokerRarity::Uncommon,
                    JokerTrigger::None,
                    JokerStructure::Normal(JokerData::GameState(data)),
                )
            }
            JokerKind::Hologram => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    7,
                    JokerRarity::Uncommon,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Vagabond => {
                let data = JokerGenerate {
                    amount: 1,
                    generate_type: GenerateType::Tarot,
                    ..JokerGenerate::default()
                };

                (
                    8,
                    JokerRarity::Rare,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Generate(data)),
                )
            }
            JokerKind::Baron => {
                let data = JokerScoring {
                    x_mult: 1.5,
                    rank: Some(vec![Rank::King]),
                    ..JokerScoring::default()
                };

                (
                    8,
                    JokerRarity::Rare,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Cloud9 => {
                let data = JokerEcon {
                    ..JokerEcon::default()
                };

                (
                    7,
                    JokerRarity::Uncommon,
                    JokerTrigger::EndOfBlind,
                    JokerStructure::Normal(JokerData::Econ(data)),
                )
            }
            JokerKind::Rocket => {
                let data = JokerEcon {
                    money: 1,
                    ..JokerEcon::default()
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::EndOfBlind,
                    JokerStructure::Normal(JokerData::Econ(data)),
                )
            }
            JokerKind::Obelisk => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    8,
                    JokerRarity::Rare,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::MidasMask => {
                let data = JokerGenerate {
                    generate_type: GenerateType::Gold,
                    ..JokerGenerate::default()
                };

                (
                    7,
                    JokerRarity::Uncommon,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Generate(data)),
                )
            }
            JokerKind::Luchador => {
                let data = JokerGameState {
                    game_state_modification: Some(GameStateModifications::DisableBossBlind),
                    ..JokerGameState::default()
                };

                (
                    5,
                    JokerRarity::Uncommon,
                    JokerTrigger::None,
                    JokerStructure::Normal(JokerData::GameState(data)),
                )
            }
            JokerKind::Photograph => {
                let data = JokerScoring {
                    x_mult: 2.0,
                    rank: Some(vec![Rank::King, Rank::Queen, Rank::Jack]),
                    ..JokerScoring::default()
                };

                (
                    5,
                    JokerRarity::Common,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::GiftCard => {
                let data = JokerEcon {
                    money: 1,
                    ..JokerEcon::default()
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::EndOfBlind,
                    JokerStructure::Normal(JokerData::Econ(data)),
                )
            }
            JokerKind::TurtleBean => {
                let data = JokerGameState {
                    hand_size: 5,
                    ..JokerGameState::default()
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::None,
                    JokerStructure::Normal(JokerData::GameState(data)),
                )
            }
            JokerKind::Erosion => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::ReservedParking => {
                let data = JokerEcon {
                    money: 1,
                    probability: 0.5,
                    ..JokerEcon::default()
                };

                (
                    6,
                    JokerRarity::Common,
                    JokerTrigger::OnHeldCard,
                    JokerStructure::Normal(JokerData::Econ(data)),
                )
            }
            JokerKind::MailInRebate => {
                let data = JokerEcon {
                    money: 5,
                    ..JokerEcon::default()
                };

                (
                    4,
                    JokerRarity::Common,
                    JokerTrigger::OnDiscard,
                    JokerStructure::Normal(JokerData::Econ(data)),
                )
            }
            JokerKind::ToTheMoon => {
                let data = JokerEcon {
                    ..JokerEcon::default()
                };

                (
                    5,
                    JokerRarity::Uncommon,
                    JokerTrigger::EndOfBlind,
                    JokerStructure::Normal(JokerData::Econ(data)),
                )
            }
            JokerKind::Hallucination => {
                let data = JokerGenerate {
                    amount: 1,
                    generate_type: GenerateType::Tarot,
                    probability: 0.5,
                    ..JokerGenerate::default()
                };

                (
                    4,
                    JokerRarity::Common,
                    JokerTrigger::None,
                    JokerStructure::Normal(JokerData::Generate(data)),
                )
            }
            JokerKind::FortuneTeller => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    6,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Juggler => {
                let data = JokerGameState {
                    hand_size: 1,
                    ..JokerGameState::default()
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::None,
                    JokerStructure::Normal(JokerData::GameState(data)),
                )
            }
            JokerKind::Drunkard => {
                let data = JokerGameState {
                    discards: 1,
                    ..JokerGameState::default()
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::None,
                    JokerStructure::Normal(JokerData::GameState(data)),
                )
            }
            JokerKind::Stone => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Golden => {
                let data = JokerEcon {
                    money: 4,
                    ..JokerEcon::default()
                };

                (
                    6,
                    JokerRarity::Common,
                    JokerTrigger::EndOfBlind,
                    JokerStructure::Normal(JokerData::Econ(data)),
                )
            }
            JokerKind::LuckyCat => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::BaseballCard => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    8,
                    JokerRarity::Rare,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Bull => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::DietCola => {
                let data = JokerGameState {
                    ..JokerGameState::default()
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::None,
                    JokerStructure::Normal(JokerData::GameState(data)),
                )
            }
            JokerKind::TradingCard => {
                let data = JokerEcon {
                    money: 3,
                    required_count: 1,
                    ..JokerEcon::default()
                };

                (
                    6,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Econ(data)),
                )
            }
            JokerKind::FlashCard => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    5,
                    JokerRarity::Uncommon,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Popcorn => {
                let data = JokerScoring {
                    add_mult: 20,
                    ..JokerScoring::default()
                };

                (
                    5,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::SpareTrousers => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Ancient => {
                let data = JokerScoring {
                    x_mult: 1.5,
                    ..JokerScoring::default()
                };

                (
                    8,
                    JokerRarity::Rare,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Ramen => {
                let data = JokerScoring {
                    x_mult: 2.0,
                    ..JokerScoring::default()
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::OnDiscard,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::WalkieTalkie => {
                let data = JokerScoring {
                    chips: 10,
                    add_mult: 4,
                    rank: Some(vec![Rank::Ten, Rank::Four]),
                    ..JokerScoring::default()
                };

                (
                    4,
                    JokerRarity::Common,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Seltzer => {
                let data = JokerRetrigger {
                    retrigger: 1,
                    target: RetriggerTarget::PlayedCards,
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Retrigger(data)),
                )
            }
            JokerKind::Castle => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    4,
                    JokerRarity::Common,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::SmileyFace => {
                let data = JokerScoring {
                    add_mult: 5,
                    rank: Some(vec![Rank::King, Rank::Queen, Rank::Jack]),
                    ..JokerScoring::default()
                };

                (
                    4,
                    JokerRarity::Common,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Campfire => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    9,
                    JokerRarity::Rare,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::GoldenTicket => {
                let data = JokerEcon {
                    money: 4,
                    enhancment: Some(Enhancement::Gold),
                    ..JokerEcon::default()
                };

                (
                    5,
                    JokerRarity::Common,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Econ(data)),
                )
            }
            JokerKind::MrBones => {
                let data = JokerGameState {
                    game_state_modification: Some(GameStateModifications::PreventDeath),
                    ..JokerGameState::default()
                };

                (
                    5,
                    JokerRarity::Uncommon,
                    JokerTrigger::None,
                    JokerStructure::Normal(JokerData::GameState(data)),
                )
            }
            JokerKind::Acrobat => {
                let data = JokerScoring {
                    x_mult: 3.0,
                    ..JokerScoring::default()
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::SockAndBuskin => {
                let data = JokerRetrigger {
                    retrigger: 1,
                    target: RetriggerTarget::FaceCards,
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Retrigger(data)),
                )
            }
            JokerKind::Swashbuckler => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    4,
                    JokerRarity::Common,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Troubadour => {
                let data = JokerGameState {
                    hand_size: 2,
                    discards: -1,
                    ..JokerGameState::default()
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::None,
                    JokerStructure::Normal(JokerData::GameState(data)),
                )
            }
            JokerKind::Certificate => {
                let data = JokerGenerate {
                    amount: 1,
                    generate_type: GenerateType::SealCard,
                    ..JokerGenerate::default()
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::None,
                    JokerStructure::Normal(JokerData::Generate(data)),
                )
            }
            JokerKind::Smeared => {
                let data = JokerGameState {
                    game_state_modification: Some(GameStateModifications::DoubleSuit),
                    ..JokerGameState::default()
                };

                (
                    7,
                    JokerRarity::Uncommon,
                    JokerTrigger::None,
                    JokerStructure::Normal(JokerData::GameState(data)),
                )
            }
            JokerKind::Throwback => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::HangingChad => {
                let data = JokerRetrigger {
                    retrigger: 2,
                    target: RetriggerTarget::FirstCard,
                };

                (
                    4,
                    JokerRarity::Common,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Retrigger(data)),
                )
            }
            JokerKind::RoughGem => {
                let data = JokerEcon {
                    money: 1,
                    suit: Some(Suit::Diamonds),
                    ..JokerEcon::default()
                };

                (
                    7,
                    JokerRarity::Uncommon,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Econ(data)),
                )
            }
            JokerKind::Bloodstone => {
                let data = JokerScoring {
                    x_mult: 1.5,
                    probability: 0.5,
                    suit: Some(Suit::Hearts),
                    ..JokerScoring::default()
                };

                (
                    7,
                    JokerRarity::Uncommon,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Arrowhead => {
                let data = JokerScoring {
                    chips: 50,
                    suit: Some(Suit::Spades),
                    ..JokerScoring::default()
                };

                (
                    7,
                    JokerRarity::Uncommon,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::OnyxAgate => {
                let data = JokerScoring {
                    add_mult: 7,
                    suit: Some(Suit::Clubs),
                    ..JokerScoring::default()
                };

                (
                    7,
                    JokerRarity::Uncommon,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Glass => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Showman => {
                let data = JokerGameState {
                    game_state_modification: Some(GameStateModifications::DuplicateJokers),
                    ..JokerGameState::default()
                };

                (
                    5,
                    JokerRarity::Uncommon,
                    JokerTrigger::None,
                    JokerStructure::Normal(JokerData::GameState(data)),
                )
            }
            JokerKind::FlowerPot => {
                let data = JokerScoring {
                    x_mult: 3.0,
                    ..JokerScoring::default()
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Blueprint => (
                10,
                JokerRarity::Rare,
                JokerTrigger::None,
                JokerStructure::Copy(CopyType::Right),
            ),
            JokerKind::Wee => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    8,
                    JokerRarity::Rare,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::MerryAndy => {
                let data = JokerGameState {
                    discards: 3,
                    hand_size: -1,
                    ..JokerGameState::default()
                };

                (
                    7,
                    JokerRarity::Uncommon,
                    JokerTrigger::None,
                    JokerStructure::Normal(JokerData::GameState(data)),
                )
            }
            JokerKind::OopsAll6s => {
                let data = JokerGameState {
                    game_state_modification: Some(GameStateModifications::DoubleProbability),
                    ..JokerGameState::default()
                };

                (
                    4,
                    JokerRarity::Uncommon,
                    JokerTrigger::None,
                    JokerStructure::Normal(JokerData::GameState(data)),
                )
            }
            JokerKind::Idol => {
                let data = JokerScoring {
                    x_mult: 2.0,
                    ..JokerScoring::default()
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::SeeingDouble => {
                let data = JokerScoring {
                    x_mult: 2.0,
                    suit: Some(Suit::Clubs),
                    ..JokerScoring::default()
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Matador => {
                let data = JokerEcon {
                    money: 8,
                    ..JokerEcon::default()
                };

                (
                    7,
                    JokerRarity::Uncommon,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Econ(data)),
                )
            }
            JokerKind::HitTheRoad => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    8,
                    JokerRarity::Rare,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Duo => {
                let data = JokerScoring {
                    x_mult: 2.0,
                    poker_hand: Some(PokerHand::Pair),
                    ..JokerScoring::default()
                };

                (
                    8,
                    JokerRarity::Rare,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Trio => {
                let data = JokerScoring {
                    x_mult: 3.0,
                    poker_hand: Some(PokerHand::ThreeOfAKind),
                    ..JokerScoring::default()
                };

                (
                    8,
                    JokerRarity::Rare,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Family => {
                let data = JokerScoring {
                    x_mult: 4.0,
                    poker_hand: Some(PokerHand::FourOfAKind),
                    ..JokerScoring::default()
                };

                (
                    8,
                    JokerRarity::Rare,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Order => {
                let data = JokerScoring {
                    x_mult: 3.0,
                    poker_hand: Some(PokerHand::Straight),
                    ..JokerScoring::default()
                };

                (
                    8,
                    JokerRarity::Rare,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Tribe => {
                let data = JokerScoring {
                    x_mult: 2.0,
                    poker_hand: Some(PokerHand::Flush),
                    ..JokerScoring::default()
                };

                (
                    8,
                    JokerRarity::Rare,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Stuntman => {
                let data = JokerScoring {
                    chips: 250,
                    hand_size: -2,
                    poker_hand: Some(PokerHand::Flush),
                    ..JokerScoring::default()
                };

                (
                    7,
                    JokerRarity::Rare,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Invisible => {
                let data = JokerGenerate {
                    amount: 1,
                    generate_type: GenerateType::Joker,
                    ..JokerGenerate::default()
                };

                (
                    8,
                    JokerRarity::Rare,
                    JokerTrigger::None,
                    JokerStructure::Normal(JokerData::Generate(data)),
                )
            }
            JokerKind::Brainstorm => (
                10,
                JokerRarity::Rare,
                JokerTrigger::None,
                JokerStructure::Copy(CopyType::LeftMost),
            ),
            JokerKind::Satellite => {
                let data = JokerEcon {
                    ..JokerEcon::default()
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Econ(data)),
                )
            }
            JokerKind::ShootTheMoon => {
                let data = JokerScoring {
                    add_mult: 13,
                    rank: Some(vec![Rank::Queen]),
                    ..JokerScoring::default()
                };

                (
                    5,
                    JokerRarity::Common,
                    JokerTrigger::OnHeldCard,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::DriversLicense => {
                let data = JokerScoring {
                    x_mult: 3.0,
                    ..JokerScoring::default()
                };

                (
                    7,
                    JokerRarity::Rare,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Cartomancer => {
                let data = JokerGenerate {
                    amount: 1,
                    generate_type: GenerateType::Tarot,
                    ..JokerGenerate::default()
                };

                (
                    6,
                    JokerRarity::Uncommon,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Generate(data)),
                )
            }
            JokerKind::Astronomer => {
                let data = JokerGameState {
                    game_state_modification: Some(GameStateModifications::FreePlanetCards),
                    ..JokerGameState::default()
                };

                (
                    8,
                    JokerRarity::Uncommon,
                    JokerTrigger::None,
                    JokerStructure::Normal(JokerData::GameState(data)),
                )
            }
            JokerKind::BurntJoker => {
                let data = JokerGenerate {
                    amount: 1,
                    generate_type: GenerateType::PlanetLevel,
                    ..JokerGenerate::default()
                };

                (
                    8,
                    JokerRarity::Rare,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Generate(data)),
                )
            }
            JokerKind::Bootstraps => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    7,
                    JokerRarity::Uncommon,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Canio => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    20,
                    JokerRarity::Legendary,
                    JokerTrigger::AfterHand,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Triboulet => {
                let data = JokerScoring {
                    x_mult: 2.0,
                    rank: Some(vec![Rank::King, Rank::Queen]),
                    ..JokerScoring::default()
                };

                (
                    20,
                    JokerRarity::Legendary,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Yorick => {
                let data = JokerScoring {
                    ..JokerScoring::default()
                };

                (
                    20,
                    JokerRarity::Legendary,
                    JokerTrigger::OnPlayedCard,
                    JokerStructure::Normal(JokerData::Scoring(data)),
                )
            }
            JokerKind::Chicot => {
                let data = JokerGameState {
                    game_state_modification: Some(GameStateModifications::DisableBossBlind),
                    ..JokerGameState::default()
                };

                (
                    20,
                    JokerRarity::Legendary,
                    JokerTrigger::None,
                    JokerStructure::Normal(JokerData::GameState(data)),
                )
            }
            JokerKind::Perkeo => {
                let data = JokerGenerate {
                    amount: 1,
                    generate_type: GenerateType::NegativeConsumable,
                    ..JokerGenerate::default()
                };

                (
                    20,
                    JokerRarity::Legendary,
                    JokerTrigger::EndOfBlind,
                    JokerStructure::Normal(JokerData::Generate(data)),
                )
            }
        };

        let update_mask = Self::update_mask(joker_kind);

        Joker {
            kind: joker_kind,
            structure,
            edition: JokerEdition::None,
            trigger,
            rarity,
            price,
            debuffed: false,
            update_mask,
        }
    }
}

pub struct JokerScoring {
    chips: u16,
    add_mult: u8,
    req_count: u8,
    hand_size: i8,
    x_mult: f32,
    probability: f32,
    rank: Option<Vec<Rank>>,
    suit: Option<Suit>,
    poker_hand: Option<PokerHand>,
}

impl JokerScoring {
    pub fn default() -> JokerScoring {
        JokerScoring {
            chips: 0,
            add_mult: 0,
            req_count: 0,
            hand_size: 0,
            x_mult: 1.0,
            probability: 0.0,
            rank: None,
            suit: None,
            poker_hand: None,
        }
    }
}

pub struct JokerEcon {
    money: u8,
    required_count: u8,
    probability: f32,
    rank: Option<Vec<Rank>>,
    suit: Option<Suit>,
    poker_hand: Option<PokerHand>,
    enhancment: Option<Enhancement>,
}

impl JokerEcon {
    pub fn default() -> JokerEcon {
        JokerEcon {
            money: 0,
            required_count: 0,
            probability: 0.0,
            rank: None,
            suit: None,
            poker_hand: None,
            enhancment: None,
        }
    }
}

pub struct JokerGameState {
    discards: i8,
    hand_size: i8,
    hands: u8,
    reroll_cost: u8,
    min_money: i8,
    straight_flush_size: u8,
    probability: f32,
    game_state_modification: Option<GameStateModifications>,
}

impl JokerGameState {
    pub fn default() -> JokerGameState {
        JokerGameState {
            discards: 0,
            hand_size: 0,
            hands: 0,
            reroll_cost: 0,
            min_money: 0,
            straight_flush_size: 5,
            probability: 1.0,
            game_state_modification: None,
        }
    }
}

pub struct JokerRetrigger {
    retrigger: u8,
    target: RetriggerTarget,
}

pub struct JokerGenerate {
    probability: f32,
    amount: u8,
    generate_type: GenerateType,
    rank: Option<Rank>,
    poker_hand: Option<PokerHand>,
}

impl JokerGenerate {
    pub fn default() -> JokerGenerate {
        JokerGenerate {
            probability: 1.0,
            amount: 0,
            generate_type: GenerateType::None,
            rank: None,
            poker_hand: None,
        }
    }
}

#[cfg(test)]
#[path = "tests/joker.rs"]
mod tests;
