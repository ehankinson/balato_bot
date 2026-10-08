use crate::calculation::poker::determine_poker_hand;
use crate::core::blind::{ScoringCard, ScoringData, card_ids};
use crate::core::card::Card;
use crate::core::enums::{Edition, Enhancement, PokerHand, Rank, Seal, Suit};

fn card(rank: Rank, suit: Suit) -> Card {
    Card::new(rank, suit, Enhancement::None, Edition::None, Seal::None)
}

#[test]
fn empty_cards_have_no_poker_hand() {
    assert_eq!(determine_poker_hand(&[]), None);
}

#[test]
fn naive_poker_hand_detection_covers_common_hands() {
    let pair = [card(Rank::Two, Suit::Spades), card(Rank::Two, Suit::Hearts)];
    assert_eq!(
        determine_poker_hand(&card_ids(&pair)).map(|result| result.hand),
        Some(PokerHand::Pair)
    );
    let straight_flush = [
        card(Rank::Two, Suit::Spades),
        card(Rank::Three, Suit::Spades),
        card(Rank::Four, Suit::Spades),
        card(Rank::Five, Suit::Spades),
        card(Rank::Six, Suit::Spades),
    ];
    assert_eq!(
        determine_poker_hand(&card_ids(&straight_flush)).map(|result| result.hand),
        Some(PokerHand::StraightFlush)
    );
}

#[test]
fn naive_detection_handles_ace_low_straights() {
    let cards = [
        card(Rank::Ace, Suit::Spades),
        card(Rank::Two, Suit::Hearts),
        card(Rank::Three, Suit::Clubs),
        card(Rank::Four, Suit::Diamonds),
        card(Rank::Five, Suit::Spades),
    ];
    assert_eq!(
        determine_poker_hand(&card_ids(&cards)).map(|result| result.hand),
        Some(PokerHand::Straight)
    );
}

#[test]
fn result_identifies_only_the_cards_that_score_the_hand() {
    let cards = [
        card(Rank::Two, Suit::Spades),
        card(Rank::Five, Suit::Hearts),
        card(Rank::Ace, Suit::Clubs),
        card(Rank::Nine, Suit::Diamonds),
        card(Rank::Seven, Suit::Hearts),
    ];

    let result = determine_poker_hand(&card_ids(&cards)).unwrap();

    assert_eq!(result.hand, PokerHand::HighCard);
    assert_eq!(result.scoring_card_indices, vec![2]);
}

#[test]
fn scoring_data_preserves_card_ids_and_source_order() {
    let ids = [11_u16, 22_u16];
    let data = ScoringData::from_card_ids(&ids, &[]);

    assert_eq!(
        data.non_scoring_played_cards,
        vec![
            ScoringCard { index: 0, id: 11 },
            ScoringCard { index: 1, id: 22 },
        ]
    );
}

#[test]
fn hand_priority_prefers_the_requested_higher_hand() {
    let flush_five = vec![card(Rank::Ace, Suit::Spades); 5];
    assert_eq!(
        determine_poker_hand(&card_ids(&flush_five)).map(|result| result.hand),
        Some(PokerHand::FlushFive)
    );

    let flush_house = [
        card(Rank::Ace, Suit::Spades),
        card(Rank::Ace, Suit::Spades),
        card(Rank::Ace, Suit::Spades),
        card(Rank::King, Suit::Spades),
        card(Rank::King, Suit::Spades),
    ];
    assert_eq!(
        determine_poker_hand(&card_ids(&flush_house)).map(|result| result.hand),
        Some(PokerHand::FlushHouse)
    );

    let five_of_a_kind = [
        card(Rank::Ace, Suit::Spades),
        card(Rank::Ace, Suit::Hearts),
        card(Rank::Ace, Suit::Diamonds),
        card(Rank::Ace, Suit::Clubs),
        card(Rank::Ace, Suit::Spades),
    ];
    assert_eq!(
        determine_poker_hand(&card_ids(&five_of_a_kind)).map(|result| result.hand),
        Some(PokerHand::FiveOfAKind)
    );
}

#[test]
fn wild_cards_count_as_every_suit_for_flushes() {
    let cards = [
        card(Rank::Two, Suit::Spades),
        card(Rank::Three, Suit::Spades),
        card(Rank::Four, Suit::Spades),
        card(Rank::Five, Suit::Spades),
        Card::new(
            Rank::Six,
            Suit::Hearts,
            Enhancement::Wild,
            Edition::None,
            Seal::None,
        ),
    ];

    assert_eq!(
        determine_poker_hand(&card_ids(&cards)).map(|result| result.hand),
        Some(PokerHand::StraightFlush)
    );
}
