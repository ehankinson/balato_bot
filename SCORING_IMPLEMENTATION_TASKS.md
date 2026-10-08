# Scoring Implementation Tasks

The scoring pipeline will be implemented in this order:

1. Determine the poker hand and partition played and held cards into scoring and non-scoring indices.
2. Trigger Jokers registered for `before_played` effects.
3. Iterate through scoring cards from left to right.
4. Apply each card's chips, enhancement, seal, and edition effects.
5. Trigger the relevant Joker effects for each scoring card.
6. Apply held-card effects after the played-card pass.
7. Trigger retriggers in their preserved left-to-right order.
8. Trigger `after_hand` Joker effects.
9. Calculate and record the final score using the exact order in which chips, additive mult, and multiplicative mult were applied.

The first poker-hand implementation is intentionally naive and lives in
`balatro_engine/src/calculation/poker.rs`. It will be expanded as scoring rules
and special card behavior are added.
