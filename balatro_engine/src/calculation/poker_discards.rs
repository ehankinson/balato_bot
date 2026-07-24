use crate::core::enums::PokerHand;
use itertools::Itertools;
use use_combinatorics::combinations;
use std::{collections::HashMap, panic};

fn generate_draw_combos(
    remaining: &[usize],
    minimums: &[usize],
    cards_to_draw: usize,
) -> Vec<Vec<usize>> {
    
    let val: usize = minimums.iter().sum();
    
    if val > cards_to_draw {
        return Vec::new();
    }

    Vec::new()
}

fn calculate_odds(
    total_cards: usize,
    hand: &Vec<(u8, u8, i64)>,
    bucket: &Vec<u8>,
    values: &Vec<Vec<u8>>,
    amount: Vec<usize>,
    counts: &Vec<usize>,
) {
    let max_iter = bucket.len();
    let total_draws = combinations(total_cards, 5);
    let val_weights = vec![0usize; max_iter];

    let mut best_val = -1;
    let mut best_score = -1;
    let mut best_prob = 0.0;

    for hand in values.iter() {
        let good_draws = 0;
        let score = 0;
        let amount_needed = Vec::new();
        let deck_val_amounts = Vec::new();

        let mut skip_count = 0;
        for (val, req_amount) in data.iter().zip(amount.iter()) {
            let needed = max(0, &req_amount - &bucket[val]);
            if &needed == 0 {
                skip_count += 1;
                continue;
            }

            amount_needed.push(needed);
            let deck_val_count = counts[val];
            if deck_val_count == 0 | needed > deck_val_count {
                skip_count += 1;
                continue;
            }

            deck_val_amounts.push(deck_val_count);
        }

        if skip_count == hand.len() { continue; }
        
    }
}

pub(crate) fn generate_discard_table(
    total_cards: usize,
    suit_counts: Vec<usize>,
    suit_scores: Vec<i64>,
    rank_counts: Vec<usize>,
    rank_scores: Vec<i64>,
    suit_rank_counts: Vec<usize>,
    suit_rank_scores: Vec<i64>,
    current_hand: Vec<(u8, u8, i64)>,
) -> HashMap<PokerHand, (i64, f64, Vec<usize>)> {
    let mut suit_bucket: Vec<u8> = vec![0usize; 4];
    let mut rank_bucket: Vec<u8> = vec![0usize; 13];
    let mut suit_rank_bucket: Vec<u8> = vec![0usize; 64];

    for (rank, suit, score) in current_hand {
        let suit_rank_key = suit << 4 | rank;

        suit_rank_bucket[suit_rank_key as usize] += 1;
        suit_bucket[suit as usize] += 1;
        rank_bucket[rank as usize] += 1;
    }

    let mut table = HashMap::new();
    let rank_array: Vec<Vec<u8>> = (0u8..13).map(|rank| vec![rank]).collect();
    let suit_array = (0u8..4).map(|suit| vec![suit]).collect();
    let suit_rank_array = (0u8..64).map(|suit_rank| vec![suit_rank]).collect();
    let two_pair_combos: Vec<Vec<u8>> = (0u8..13).combinations(2).collect();
    let full_house_combos: Vec<Vec<u8>> = (0u8..13).permutations(2).collect();

    let mut straight_array: Vec<u8> = (0u8..13).rev().collect();
    straight_array.push(12); // for the wrap around A, 2, 3, 4, 5 straight

    let mut straight_combos = Vec::new();
    for cutoff in 5..=straight_array.len() {
        straight_combos.push(straight_array[cutoff - 5..cutoff].to_vec());
    }

    let mut straight_flush_combos = Vec::new();
    for straight in &straight_combos {
        for suit in 0u8..4 {
            let combination = straight.iter().map(|&rank| (suit << 4) | rank).collect();
            straight_flush_combos.push(combination);
        }
    }

    let mut flush_house_combos = Vec::new();
    for full_house in full_house_combos.iter() {
        for suit in 0u8..4 {
            let combination: Vec<u8> = full_house.iter().map(|&rank| (suit << 4) | rank).collect();
            flush_house_combos.push(combination);
        }
    }

    for hand in PokerHand::DISCARD_HANDS {
        let (bucket, values, amount, counts) = match hand {
            PokerHand::Pair
            | PokerHand::ThreeOfAKind
            | PokerHand::FourOfAKind
            | PokerHand::FiveOfAKind => {
                let bucket = &rank_bucket;
                let values = &rank_array;
                let amount = vec![hand as usize];
                let counts = &rank_counts;

                (bucket, values, amount, counts)
            }
            PokerHand::TwoPair => {
                let bucket = &rank_bucket;
                let values = &two_pair_combos;
                let amount = vec![2, 2];
                let counts = &rank_counts;

                (bucket, values, amount, counts)
            }
            PokerHand::Straight => {
                let bucket = &rank_bucket;
                let values = &straight_combos;
                let amount = vec![1usize; 5];
                let counts = &rank_counts;

                (bucket, values, amount, counts)
            }
            PokerHand::Flush => {
                let bucket = &suit_bucket;
                let values = &suit_array;
                let amount = vec![5];
                let counts = &suit_counts;

                (bucket, values, amount, counts)
            }
            PokerHand::FullHouse => {
                let bucket = &rank_bucket;
                let values = &full_house_combos;
                let amount = vec![3, 2];
                let counts = &rank_counts;

                (bucket, values, amount, counts)
            }
            PokerHand::StraightFlush => {
                let bucket = &suit_rank_bucket;
                let values = &straight_flush_combos;
                let amount = vec![1usize; 5];
                let counts = &suit_rank_counts;

                (bucket, values, amount, counts)
            }
            PokerHand::FlushHouse => {
                let bucket = &suit_rank_bucket;
                let values = &flush_house_combos;
                let amount = vec![3, 2];
                let counts = &suit_rank_counts;

                (bucket, values, amount, counts)
            }
            PokerHand::FlushFive => {
                let bucket = &suit_rank_bucket;
                let values = &suit_rank_array;
                let amount = vec![5];
                let counts = &suit_rank_counts;

                (bucket, values, amount, counts)
            }
            _ => {
                panic!("Not possible Poker HAND!!!!")
            }
        };

        let (val, prob, discard) = calculate_odds(total_cards, &current_hand, bucket, values, amount, counts);
        
    }
    table
}
