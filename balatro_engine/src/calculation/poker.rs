use crate::core::enums::{Enhancement, PokerHand, Rank};
use crate::core::joker_types::JokerModifiers;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct PokerHandResult {
    pub(crate) hand: PokerHand,
    pub(crate) scoring_card_indices: Vec<usize>,
}

/// Naively determines the strongest poker hand and which input cards score it.
///
/// This first version ignores seals and editions. Wild cards count toward
/// every suit for flush detection while retaining their stored rank.
pub(crate) fn determine_poker_hand(
    card_ids: &[u16],
    modifiers: JokerModifiers,
) -> Option<PokerHandResult> {
    if card_ids.is_empty() {
        return None;
    }

    let mut rank_indices: [Vec<usize>; 15] = std::array::from_fn(|_| Vec::new());
    let mut suit_indices: [Vec<usize>; 4] = std::array::from_fn(|_| Vec::new());
    let mut rank_counts = [0_u8; 15];
    let mut suit_counts = [0_u8; 4];
    let mut rank_mask = 0_u16;
    let mut suit_rank_masks = [0_u16; 4];

    let mut stone_indices = Vec::new();
    for (index, &card_id) in card_ids.iter().enumerate() {
        if enhancement_from_id(card_id) == Enhancement::Stone as u8 {
            stone_indices.push(index);
            continue;
        }
        let rank = Rank::from_card_id(card_id) as u8;
        let suit = suit_from_id(card_id);
        let rank_bit = 1_u16 << rank;
        let is_wild = enhancement_from_id(card_id) == Enhancement::Wild as u8;

        rank_counts[rank as usize] += 1;
        rank_indices[rank as usize].push(index);
        rank_mask |= rank_bit;

        if is_wild || modifiers.smeared_suits {
            for suit_index in 0..suit_indices.len() {
                if !is_wild && suit_index / 2 != suit as usize / 2 {
                    continue;
                }
                suit_counts[suit_index] += 1;
                suit_indices[suit_index].push(index);
                suit_rank_masks[suit_index] |= rank_bit;
            }
        } else {
            suit_counts[suit as usize] += 1;
            suit_indices[suit as usize].push(index);
            suit_rank_masks[suit as usize] |= rank_bit;
        }
    }

    let required = if modifiers.four_fingers { 4 } else { 5 };
    let flush_suit = suit_counts
        .iter()
        .enumerate()
        .filter(|(_, count)| **count >= required)
        .max_by_key(|(suit, count)| (**count, std::cmp::Reverse(*suit)))
        .map(|(suit, _)| suit);
    let has_five = rank_counts.iter().any(|&count| count >= 5);
    let has_four = rank_counts.iter().any(|&count| count >= 4);
    let has_three = rank_counts.iter().any(|&count| count >= 3);
    let pair_count = rank_counts.iter().filter(|&&count| count >= 2).count();
    let straight = straight_ranks(rank_mask, modifiers.straight_gaps, required);
    let straight_flush = if modifiers.four_fingers {
        flush_suit.and_then(|suit| straight.clone().map(|ranks| (suit, ranks)))
    } else {
        flush_suit.and_then(|suit| {
            straight_ranks(suit_rank_masks[suit], modifiers.straight_gaps, required)
                .map(|ranks| (suit, ranks))
        })
    };

    let (hand, mut scoring_card_indices) = if flush_suit.is_some() && has_five {
        (
            PokerHand::FlushFive,
            rank_indices
                .iter()
                .find(|indices| indices.len() >= 5)
                .cloned()
                .unwrap_or_default(),
        )
    } else if flush_suit.is_some() && has_three && pair_count >= 2 {
        (
            PokerHand::FlushHouse,
            full_house_indices(&rank_indices, &rank_counts),
        )
    } else if has_five {
        (
            PokerHand::FiveOfAKind,
            rank_indices
                .iter()
                .find(|indices| indices.len() >= 5)
                .cloned()
                .unwrap_or_default(),
        )
    } else if let Some((suit, ranks)) = straight_flush {
        let mut indices = suit_indices[suit].clone();
        indices.extend(sequence_indices(
            &(0..card_ids.len()).collect::<Vec<_>>(),
            &rank_indices,
            &ranks,
        ));
        indices.sort_unstable();
        indices.dedup();
        (PokerHand::StraightFlush, indices)
    } else if has_four {
        (
            PokerHand::FourOfAKind,
            rank_indices
                .iter()
                .find(|indices| indices.len() >= 4)
                .cloned()
                .unwrap_or_default(),
        )
    } else if has_three && pair_count >= 2 {
        (
            PokerHand::FullHouse,
            full_house_indices(&rank_indices, &rank_counts),
        )
    } else if let Some(suit) = flush_suit {
        (PokerHand::Flush, suit_indices[suit].clone())
    } else if let Some(ranks) = straight {
        (
            PokerHand::Straight,
            sequence_indices(
                &(0..card_ids.len()).collect::<Vec<_>>(),
                &rank_indices,
                &ranks,
            ),
        )
    } else if has_three {
        (
            PokerHand::ThreeOfAKind,
            rank_indices
                .iter()
                .find(|indices| indices.len() >= 3)
                .cloned()
                .unwrap_or_default(),
        )
    } else if pair_count >= 2 {
        (
            PokerHand::TwoPair,
            rank_indices
                .iter()
                .filter(|indices| indices.len() >= 2)
                .flat_map(|indices| indices.iter().copied())
                .collect(),
        )
    } else if pair_count == 1 {
        (
            PokerHand::Pair,
            rank_indices
                .iter()
                .find(|indices| indices.len() >= 2)
                .cloned()
                .unwrap_or_default(),
        )
    } else {
        let indices = rank_indices
            .iter()
            .rfind(|indices| !indices.is_empty())
            .map(|indices| vec![indices[0]])
            .unwrap_or_default();
        (PokerHand::HighCard, indices)
    };

    scoring_card_indices.extend(stone_indices);
    Some(PokerHandResult {
        hand,
        scoring_card_indices,
    })
}

fn full_house_indices(rank_indices: &[Vec<usize>; 15], rank_counts: &[u8; 15]) -> Vec<usize> {
    rank_indices
        .iter()
        .enumerate()
        .filter(|(rank, _)| rank_counts[*rank] >= 2)
        .flat_map(|(_, indices)| indices.iter().copied())
        .collect()
}

fn sequence_indices(
    available_indices: &[usize],
    rank_indices: &[Vec<usize>; 15],
    ranks: &[u8],
) -> Vec<usize> {
    ranks
        .iter()
        .flat_map(|rank| {
            rank_indices[*rank as usize]
                .iter()
                .copied()
                .filter(|index| available_indices.contains(index))
        })
        .collect()
}

pub(crate) fn straight_ranks(rank_mask: u16, allow_gaps: bool, required: u8) -> Option<Vec<u8>> {
    // Ace can end a high straight or start a low one, never wrap through King.
    let mut ranks = Vec::new();
    for rank in (2..=14).rev() {
        if rank_mask & (1 << rank) != 0 {
            ranks.push(rank as u8);
        }
    }
    if rank_mask & (1 << 14) != 0 {
        ranks.push(1);
    }
    let maximum_step = if allow_gaps { 2 } else { 1 };
    // Four Fingers lowers the minimum; it does not exclude a fifth card.
    for length in (required..=5).rev() {
        for window in ranks.windows(length as usize) {
            if window
                .windows(2)
                .all(|pair| pair[0] - pair[1] <= maximum_step)
            {
                return Some(
                    window
                        .iter()
                        .rev()
                        .map(|&rank| if rank == 1 { 14 } else { rank })
                        .collect(),
                );
            }
        }
    }
    None
}

fn suit_from_id(card_id: u16) -> u8 {
    ((card_id >> 9) & 0x03) as u8
}

fn enhancement_from_id(card_id: u16) -> u8 {
    ((card_id >> 5) & 0x0f) as u8
}
