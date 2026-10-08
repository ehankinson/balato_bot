use crate::core::enums::{Enhancement, PokerHand, Rank};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct PokerHandResult {
    pub(crate) hand: PokerHand,
    pub(crate) scoring_card_indices: Vec<usize>,
}

/// Naively determines the strongest poker hand and which input cards score it.
///
/// This first version ignores seals and editions. Wild cards count toward
/// every suit for flush detection while retaining their stored rank.
pub(crate) fn determine_poker_hand(card_ids: &[u16]) -> Option<PokerHandResult> {
    if card_ids.is_empty() {
        return None;
    }

    let mut rank_indices: [Vec<usize>; 15] = std::array::from_fn(|_| Vec::new());
    let mut suit_indices: [Vec<usize>; 4] = std::array::from_fn(|_| Vec::new());
    let mut rank_counts = [0_u8; 15];
    let mut suit_counts = [0_u8; 4];
    let mut rank_mask = 0_u16;
    let mut suit_rank_masks = [0_u16; 4];

    for (index, &card_id) in card_ids.iter().enumerate() {
        let rank = Rank::from_card_id(card_id) as u8;
        let suit = suit_from_id(card_id);
        let rank_bit = 1_u16 << rank;
        let is_wild = enhancement_from_id(card_id) == Enhancement::Wild as u8;

        rank_counts[rank as usize] += 1;
        rank_indices[rank as usize].push(index);
        rank_mask |= rank_bit;

        if is_wild {
            for suit_index in 0..suit_indices.len() {
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

    let flush_suit = suit_counts
        .iter()
        .enumerate()
        .filter(|(_, count)| **count >= 5)
        .max_by_key(|(suit, count)| (**count, std::cmp::Reverse(*suit)))
        .map(|(suit, _)| suit);
    let has_five = rank_counts.iter().any(|&count| count >= 5);
    let has_four = rank_counts.iter().any(|&count| count >= 4);
    let has_three = rank_counts.iter().any(|&count| count >= 3);
    let pair_count = rank_counts.iter().filter(|&&count| count >= 2).count();
    let straight = straight_ranks(rank_mask);
    let straight_flush = flush_suit
        .and_then(|suit| straight_ranks(suit_rank_masks[suit]).map(|ranks| (suit, ranks)));

    let (hand, scoring_card_indices) = if let Some(suit) = flush_suit {
        if has_five {
            (
                PokerHand::FlushFive,
                rank_indices
                    .iter()
                    .find(|indices| indices.len() >= 5)
                    .map(|indices| {
                        indices
                            .iter()
                            .copied()
                            .filter(|index| suit_indices[suit].contains(index))
                            .collect()
                    })
                    .unwrap_or_default(),
            )
        } else if has_three && pair_count >= 2 {
            (
                PokerHand::FlushHouse,
                full_house_indices(&rank_indices, &rank_counts),
            )
        } else if let Some((_, ranks)) = straight_flush {
            (
                PokerHand::StraightFlush,
                sequence_indices(&suit_indices[suit], &rank_indices, &ranks),
            )
        } else {
            (PokerHand::Flush, suit_indices[suit].clone())
        }
    } else if has_five {
        (
            PokerHand::FiveOfAKind,
            rank_indices
                .iter()
                .find(|indices| indices.len() >= 5)
                .cloned()
                .unwrap_or_default(),
        )
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
        let index = rank_indices
            .iter()
            .rposition(|indices| !indices.is_empty())
            .unwrap();
        (PokerHand::HighCard, vec![rank_indices[index][0]])
    };

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
    ranks: &[u8; 5],
) -> Vec<usize> {
    ranks
        .iter()
        .filter_map(|rank| {
            rank_indices[*rank as usize]
                .iter()
                .copied()
                .find(|index| available_indices.contains(index))
        })
        .collect()
}

fn straight_ranks(rank_mask: u16) -> Option<[u8; 5]> {
    for start in (2..=10).rev() {
        if (start..start + 5).all(|rank| rank_mask & (1_u16 << rank) != 0) {
            return Some([
                start as u8,
                (start + 1) as u8,
                (start + 2) as u8,
                (start + 3) as u8,
                (start + 4) as u8,
            ]);
        }
    }
    if [Rank::Ace, Rank::Two, Rank::Three, Rank::Four, Rank::Five]
        .iter()
        .all(|rank| rank_mask & (1_u16 << *rank as u8) != 0)
    {
        Some([14, 2, 3, 4, 5])
    } else {
        None
    }
}

fn suit_from_id(card_id: u16) -> u8 {
    ((card_id >> 7) & 0x03) as u8
}

fn enhancement_from_id(card_id: u16) -> u8 {
    ((card_id >> 4) & 0x0f) as u8
}
