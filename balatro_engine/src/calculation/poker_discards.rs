use crate::core::enums::PokerHand;
use itertools::Itertools;
use std::sync::LazyLock;
use std::{collections::HashMap, panic};
use use_combinatorics::combinations;

static HAND_POSSIBILITIES: LazyLock<Vec<Vec<Vec<u8>>>> = LazyLock::new(|| {
    let rank_combos: Vec<Vec<u8>> = (0..13).map(|rank| vec![rank]).collect();
    let suit_combos: Vec<Vec<u8>> = (0u8..4).map(|suit| vec![suit]).collect();
    let suit_rank_combos: Vec<Vec<u8>> = (0u8..64).map(|suit_rank| vec![suit_rank]).collect();
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
            let combination: Vec<u8> = straight.iter().map(|&rank| (suit << 4) | rank).collect();
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

    vec![
        rank_combos.clone(),   // Pair
        rank_combos.clone(),   // Three of a kind
        rank_combos.clone(),   // Four of a kind
        rank_combos,           // Five of a kind
        two_pair_combos,       // Two Pair
        straight_combos,       // Straight
        suit_combos,           // Flush
        full_house_combos,     // Full House
        straight_flush_combos, // Straight Flush
        flush_house_combos,    // Flush House
        suit_rank_combos,      // Flush Five
    ]
});

#[derive(Clone)]
struct Holder {
    count: u8,
    score: Vec<i64>,
}

fn generate_draw_combos(
    remaining: &[usize],
    minimums: &[usize],
    cards_to_draw: usize,
) -> Vec<Vec<usize>> {
    if minimums.iter().sum::<usize>() > cards_to_draw {
        return Vec::new();
    }

    let mut results = Vec::new();
    let mut stack: Vec<(usize, usize, Vec<usize>)> = vec![(0, 0, Vec::new())];

    while let Some((index, used_cards, drawn)) = stack.pop() {
        if index == remaining.len() {
            let mut combo = drawn;
            combo.push(cards_to_draw - used_cards);
            results.push(combo);
            continue;
        }

        let min_draw = minimums[index];
        let max_draw = remaining[index].min(cards_to_draw - used_cards);

        for amount in (min_draw..=max_draw).rev() {
            let mut next_drawn = drawn.clone();
            next_drawn.push(amount);
            stack.push((index + 1, used_cards + amount, next_drawn));
        }
    }

    results
}

fn calculate_odds(
    total_cards: usize,
    hand: &Vec<(u8, u8, i64)>,
    bucket: &Vec<Holder>,
    values: &Vec<Vec<u8>>,
    amount: Vec<usize>,
    counts: &Vec<usize>,
    scores: &Vec<i64>,
) -> (i32, f64, Vec<(u8, u8, i64)>) {
    let max_iter = bucket.len();
    let total_draws = combinations(total_cards as u64, 5).unwrap_or(0);
    let mut val_weights = vec![0.0f64; max_iter];

    let mut best_val = -1;
    let mut best_score = -1.0;
    let mut best_prob = 0.0;

    let id_shft = match max_iter {
        4 => 2,
        13 => 4,
        _ => 6,
    };

    let mut amount_needed = Vec::with_capacity(6);
    let mut deck_val_amounts = Vec::with_capacity(6);

    for hand in values.iter() {
        let mut score = 0.0;

        let mut skip_count = 0;
        for (val, req_amount) in hand.iter().zip(amount.iter()) {
            let needed = req_amount.saturating_sub(bucket[*val as usize].count as usize);
            if needed == 0 {
                skip_count += 1;
                continue;
            }

            amount_needed.push(needed);
            let deck_val_count = counts[*val as usize];
            if (deck_val_count == 0) | (needed > deck_val_count) {
                skip_count += 1;
                continue;
            }

            deck_val_amounts.push(deck_val_count);
            let expected_deck_score = (scores[*val as usize] as usize / deck_val_count) as f64;
            let card_holder = &bucket[*val as usize];
            let card_score: f64 = card_holder.score[..card_holder.count as usize]
                .iter()
                .sum::<i64>() as f64;

            score += card_score + expected_deck_score;
        }

        if skip_count == hand.len() {
            continue;
        }

        let draw_combos = generate_draw_combos(&deck_val_amounts, &amount_needed, 5);
        deck_val_amounts.push(total_cards - deck_val_amounts.iter().sum::<usize>());

        let good_draws: u128 = draw_combos
            .iter()
            .map(|draw| {
                draw.iter()
                    .zip(deck_val_amounts.iter())
                    .map(|(&amount, &available)| {
                        combinations(available as u64, amount as u64).unwrap_or(0)
                    })
                    .product::<u128>()
            })
            .sum();

        // resetting the arrays instead of needing to re-allocate them
        amount_needed.clear();
        deck_val_amounts.clear();

        let probability = good_draws as f64 / total_draws as f64;
        let weighted_score = probability * score;
        let mut val_id = 1;
        for val in hand.iter() {
            val_weights[*val as usize] += weighted_score;
            val_id = (*val as i32) << id_shft;
        }

        if probability > best_prob {
            best_prob = probability;
            best_score = score;
            best_val = val_id;
        } else if probability == best_prob && score > best_score {
            best_score = score;
            best_val = val_id;
        }
    }

    let mut ordered_hand = hand.clone();

    ordered_hand.sort_by(|a, b| {
        let a_index = if max_iter == 4 {
            a.1 as usize
        } else if max_iter == 13 {
            a.0 as usize
        } else {
            ((a.1 << 4) | a.0) as usize
        };

        let b_index = if max_iter == 4 {
            b.1 as usize
        } else if max_iter == 13 {
            b.0 as usize
        } else {
            ((b.1 << 4) | b.0) as usize
        };

        val_weights[a_index].total_cmp(&val_weights[b_index])
    });

    let discard = ordered_hand[..ordered_hand.len().min(5)].to_vec();

    (best_val, best_prob, discard)
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
) -> HashMap<PokerHand, (i32, f64, Vec<(u8, u8, i64)>)> {
    let mut suit_bucket: Vec<Holder> = vec![
        Holder {
            count: 0,
            score: Vec::new()
        };
        4
    ];
    let mut rank_bucket: Vec<Holder> = vec![
        Holder {
            count: 0,
            score: Vec::new()
        };
        13
    ];
    let mut suit_rank_bucket: Vec<Holder> = vec![
        Holder {
            count: 0,
            score: Vec::new()
        };
        64
    ];

    for (rank, suit, score) in current_hand.iter() {
        let suit_rank_key = suit << 4 | rank;

        suit_rank_bucket[suit_rank_key as usize].count += 1;
        suit_rank_bucket[suit_rank_key as usize].score.push(*score);

        suit_bucket[*suit as usize].count += 1;
        suit_bucket[*suit as usize].score.push(*score);

        rank_bucket[*rank as usize].count += 1;
        rank_bucket[*rank as usize].score.push(*score);
    }

    for suit in &mut suit_bucket {
        suit.score.sort_unstable_by(|a, b| b.cmp(a));
    }

    for rank in &mut rank_bucket {
        rank.score.sort_unstable_by(|a, b| b.cmp(a));
    }

    for suit_rank in &mut suit_rank_bucket {
        suit_rank.score.sort_unstable_by(|a, b| b.cmp(a));
    }

    let mut table = HashMap::new();

    for hand in PokerHand::DISCARD_HANDS {
        let index = hand as usize - 2;
        let (bucket, values, amount, counts, scores) = match hand {
            PokerHand::Pair
            | PokerHand::ThreeOfAKind
            | PokerHand::FourOfAKind
            | PokerHand::FiveOfAKind => {
                let bucket = &rank_bucket;
                let values = &HAND_POSSIBILITIES[index];
                let amount = vec![hand as usize];
                let counts = &rank_counts;
                let scores = &rank_scores;

                (bucket, values, amount, counts, scores)
            }
            PokerHand::TwoPair => {
                let bucket = &rank_bucket;
                let values = &HAND_POSSIBILITIES[index];
                let amount = vec![2, 2];
                let counts = &rank_counts;
                let scores = &rank_scores;

                (bucket, values, amount, counts, scores)
            }
            PokerHand::Straight => {
                let bucket = &rank_bucket;
                let values = &HAND_POSSIBILITIES[index];
                let amount = vec![1usize; 5];
                let counts = &rank_counts;
                let scores = &rank_scores;

                (bucket, values, amount, counts, scores)
            }
            PokerHand::Flush => {
                let bucket = &suit_bucket;
                let values = &HAND_POSSIBILITIES[index];
                let amount = vec![5];
                let counts = &suit_counts;
                let scores = &suit_scores;

                (bucket, values, amount, counts, scores)
            }
            PokerHand::FullHouse => {
                let bucket = &rank_bucket;
                let values = &HAND_POSSIBILITIES[index];
                let amount = vec![3, 2];
                let counts = &rank_counts;
                let scores = &rank_scores;

                (bucket, values, amount, counts, scores)
            }
            PokerHand::StraightFlush => {
                let bucket = &suit_rank_bucket;
                let values = &HAND_POSSIBILITIES[index];
                let amount = vec![1usize; 5];
                let counts = &suit_rank_counts;
                let scores = &suit_rank_scores;

                (bucket, values, amount, counts, scores)
            }
            PokerHand::FlushHouse => {
                let bucket = &suit_rank_bucket;
                let values = &HAND_POSSIBILITIES[index];
                let amount = vec![3, 2];
                let counts = &suit_rank_counts;
                let scores = &suit_rank_scores;

                (bucket, values, amount, counts, scores)
            }
            PokerHand::FlushFive => {
                let bucket = &suit_rank_bucket;
                let values = &HAND_POSSIBILITIES[index];
                let amount = vec![5];
                let counts = &suit_rank_counts;
                let scores = &suit_rank_scores;

                (bucket, values, amount, counts, scores)
            }
            _ => {
                panic!("Not possible Poker HAND!!!!")
            }
        };

        let (val, prob, discard) = calculate_odds(
            total_cards,
            &current_hand,
            bucket,
            values,
            amount,
            counts,
            scores,
        );

        table.insert(hand, (val, prob, discard));
    }
    table
}
