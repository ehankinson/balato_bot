use crate::core::enums::{Enhancement, PokerHand, Rank};

/// Naively determines the strongest poker hand represented by the card IDs.
///
/// This first version ignores seals and editions. Wild cards count toward every
/// suit for flush detection while retaining their stored rank.
pub(crate) fn determine_poker_hand(card_ids: &[u16]) -> Option<PokerHand> {
    if card_ids.is_empty() {
        return None;
    }

    let mut rank_counts = [0_u8; 15];
    let mut suit_counts = [0_u8; 4];
    let mut suit_rank_masks = [0_u16; 4];
    let mut rank_mask = 0_u16;
    for &card_id in card_ids {
        let rank = rank_from_id(card_id);
        let suit = suit_from_id(card_id);
        let rank_bit = 1_u16 << rank;
        rank_counts[rank as usize] += 1;
        rank_mask |= rank_bit;

        if enhancement_from_id(card_id) == Enhancement::Wild as u8 {
            for suit_index in 0..suit_counts.len() {
                suit_counts[suit_index] += 1;
                suit_rank_masks[suit_index] |= rank_bit;
            }
        } else {
            suit_counts[suit as usize] += 1;
            suit_rank_masks[suit as usize] |= rank_bit;
        }
    }

    let has_flush = suit_counts.into_iter().any(|count| count >= 5);
    let has_any_straight = has_straight(rank_mask);
    let has_straight_flush = suit_counts
        .into_iter()
        .zip(suit_rank_masks)
        .any(|(count, rank_mask)| count >= 5 && has_straight(rank_mask));
    let has_five = rank_counts.into_iter().any(|count| count >= 5);
    let has_four = rank_counts.into_iter().any(|count| count >= 4);
    let has_three = rank_counts.into_iter().any(|count| count >= 3);
    let pair_count = rank_counts.into_iter().filter(|&count| count >= 2).count();

    if has_five && has_flush {
        Some(PokerHand::FlushFive)
    } else if has_flush && has_three && pair_count >= 2 {
        Some(PokerHand::FlushHouse)
    } else if has_five {
        Some(PokerHand::FiveOfAKind)
    } else if has_straight_flush {
        Some(PokerHand::StraightFlush)
    } else if has_four {
        Some(PokerHand::FourOfAKind)
    } else if has_three && pair_count >= 2 {
        Some(PokerHand::FullHouse)
    } else if has_flush {
        Some(PokerHand::Flush)
    } else if has_any_straight {
        Some(PokerHand::Straight)
    } else if has_three {
        Some(PokerHand::ThreeOfAKind)
    } else if pair_count >= 2 {
        Some(PokerHand::TwoPair)
    } else if pair_count == 1 {
        Some(PokerHand::Pair)
    } else {
        Some(PokerHand::HighCard)
    }
}

fn has_straight(rank_mask: u16) -> bool {
    (2..=10).any(|start| (start..start + 5).all(|rank| rank_mask & (1_u16 << rank) != 0))
        || [Rank::Ace, Rank::Two, Rank::Three, Rank::Four, Rank::Five]
            .iter()
            .all(|rank| rank_mask & (1_u16 << *rank as u8) != 0)
}

fn rank_from_id(card_id: u16) -> u8 {
    ((card_id >> 9) & 0x0f) as u8
}

fn suit_from_id(card_id: u16) -> u8 {
    ((card_id >> 7) & 0x03) as u8
}

fn enhancement_from_id(card_id: u16) -> u8 {
    ((card_id >> 4) & 0x0f) as u8
}
