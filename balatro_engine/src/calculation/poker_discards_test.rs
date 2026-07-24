use super::poker_discards::generate_discard_table;
use crate::core::enums::PokerHand;
use std::hint::black_box;
use std::time::{Duration, Instant};

const TOTAL_CARDS_AFTER_DEAL: usize = 44;
const PYTHON_EXAMPLE_HAND: [(u8, u8, i64); 8] = [
    // (rank, suit, score): A♦, Q♥, 10♦, 8♥, 7♥, 6♣, 6♦, 5♣
    (12, 1, 1),
    (10, 0, 1),
    (8, 1, 1),
    (6, 0, 1),
    (5, 0, 1),
    (4, 2, 1),
    (4, 1, 1),
    (3, 2, 1),
];

fn python_example_deck_summary() -> (Vec<usize>, Vec<usize>, Vec<usize>) {
    // Start with a standard 52-card deck, then remove the eight cards in the
    // Python example (`deck.filter(hand)`).
    let mut suit_counts = vec![13; 4];
    let mut rank_counts = vec![4; 13];
    let mut suit_rank_counts = vec![1; 64];

    for &(rank, suit, _) in &PYTHON_EXAMPLE_HAND {
        suit_counts[suit as usize] -= 1;
        rank_counts[rank as usize] -= 1;
        suit_rank_counts[((suit << 4) | rank) as usize] -= 1;
    }

    (suit_counts, rank_counts, suit_rank_counts)
}

#[test]
fn python_example_produces_the_expected_pair_probability() {
    let (suit_counts, rank_counts, suit_rank_counts) = python_example_deck_summary();

    let table = generate_discard_table(
        TOTAL_CARDS_AFTER_DEAL,
        suit_counts,
        vec![0; 4],
        rank_counts,
        vec![0; 13],
        suit_rank_counts,
        vec![0; 64],
        PYTHON_EXAMPLE_HAND.to_vec(),
    );

    assert_eq!(table.len(), PokerHand::DISCARD_HANDS.len());

    let (_, pair_probability, _) = table
        .get(&PokerHand::Pair)
        .expect("the table must include a Pair entry");

    // Reference value from python/calculation/poker_discards.py using the
    // same deck and eight-card hand.
    assert!((pair_probability - 0.309_951_676_230_746).abs() < 1e-12);
}

#[test]
#[ignore = "run manually to measure discard-table throughput"]
fn benchmark_discard_tables_for_one_second() {
    let (suit_counts, rank_counts, suit_rank_counts) = python_example_deck_summary();
    let suit_scores = vec![0; 4];
    let rank_scores = vec![0; 13];
    let suit_rank_scores = vec![0; 64];
    let hand = PYTHON_EXAMPLE_HAND.to_vec();
    let duration = Duration::from_secs(1);
    let started_at = Instant::now();
    let mut tables_generated = 0_u64;

    while started_at.elapsed() < duration {
        black_box(generate_discard_table(
            TOTAL_CARDS_AFTER_DEAL,
            suit_counts.clone(),
            suit_scores.clone(),
            rank_counts.clone(),
            rank_scores.clone(),
            suit_rank_counts.clone(),
            suit_rank_scores.clone(),
            hand.clone(),
        ));
        tables_generated += 1;
    }

    let elapsed = started_at.elapsed();
    println!(
        "generated {tables_generated} discard tables in {:.3} s ({:.0} tables/sec, {:.3} ms/table)",
        elapsed.as_secs_f64(),
        tables_generated as f64 / elapsed.as_secs_f64(),
        elapsed.as_secs_f64() * 1_000.0 / tables_generated as f64,
    );
    assert!(tables_generated > 0);
}
