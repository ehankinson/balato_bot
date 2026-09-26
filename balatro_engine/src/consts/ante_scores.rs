use crate::core::enums::Stakes;

pub const ANTE_SCORES: [[[u64; 3]; 8]; 3] = [
    // Easy: White / Red
    [
        [300, 450, 600],
        [800, 1_200, 1_600],
        [2_000, 3_000, 4_000],
        [5_000, 7_500, 10_000],
        [11_000, 16_500, 22_000],
        [20_000, 30_000, 40_000],
        [35_000, 52_500, 70_000],
        [50_000, 75_000, 100_000],
    ],
    // Medium: Green / Black / Blue
    [
        [300, 450, 600],
        [900, 1_350, 1_800],
        [2_600, 3_900, 5_200],
        [8_000, 12_000, 16_000],
        [20_000, 30_000, 40_000],
        [36_000, 54_000, 72_000],
        [60_000, 90_000, 120_000],
        [100_000, 150_000, 200_000],
    ],
    // Hard: Purple / Orange / Gold
    [
        [300, 450, 600],
        [1_000, 1_500, 2_000],
        [3_200, 4_800, 6_400],
        [9_000, 13_500, 18_000],
        [25_000, 37_500, 50_000],
        [60_000, 90_000, 120_000],
        [110_000, 165_000, 220_000],
        [200_000, 300_000, 400_000],
    ],
];

pub fn endless_ante_stakes(stake: &Stakes, ante: u8) -> [f64; 3] {
    assert!(ante > 8, "endless ante_stakes requires ante > 8");

    let difficulty = match stake {
        Stakes::White | Stakes::Red => 50_000.0,
        Stakes::Green | Stakes::Black | Stakes::Blue => 100_000.0,
        _ => 200_000.0,
    };

    let d = (ante - 8) as f64;

    let true_score = difficulty * (1.6 + (0.75 * d).powf(1.0 + 0.2 * d)).powf(d);
    let digits = true_score.log10().floor();
    let z = 10.0f64.powf(digits - 1.0f64);

    let base_score = (true_score / z).floor() * z;

    [base_score, base_score * 1.5, base_score * 2.0]
}
