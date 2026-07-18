# Balatro First-Blind Bot: Current Plan

## Goal

Train a bot for the first blind (initially base deck, no jokers) that wins
reliably while using as few hands and discards as possible.

The bot is intentionally planner-guided. It will receive currently available
summaries such as the best scoring hand, discard probabilities, economy
options, and Tarot, Planet, and Spectral-generation options. The policy learns
how to decide from those summaries.

## Policy Inputs

Keep card and game-state features plus advisor features that can be calculated
when an action is made.

Allowed advisor features include:

- Current `best_hand` score, hand type, and scored/unscored card roles.
- The discard table and its estimated probabilities.
- Future economy, Tarot, Planet, and Spectral candidate information.

They must not depend on future deck order or future random outcomes.

`encoder.py` tasks:

- Keep a stable hand-card order before flattening features.
- `PokerHand` values are `1..12`; one-hot values for 12 classes must be shifted
  to `0..11` with `int(hand) - 1`.
- Remove the no-op `tuple(state_features)` line.
- Ensure every observation has a fixed shape.

## Environment

Create `python/simulation/blind_env.py`:

```python
observation, action_masks = env.reset(seed)
next_observation, reward, terminated, info = env.step(action)
```

Initial configuration: target 300, four hands, three discards, eight-card hand,
base deck, no jokers.

### Reset

1. Create and shuffle a fresh deck.
2. Reset game state and draw eight cards.
3. Calculate advisor data (`best_hand`, discard table).
4. Return the encoded state and action masks.

### Play Transition

1. Decode selected cards and score the play.
2. Call `game_state.play_hand()` and add earned score.
3. Remove all selected played cards; add them to the discard pile.
4. Unless the blind is won, draw back to hand size.
5. End as a win at target score, or loss when no hands remain.

### Discard Transition

1. Decode selected cards and call `game_state.discard()`.
2. Remove them, add them to the discard pile, and draw replacements.

Write tests for reset, play, discard, win, loss, and card conservation before
training.

## Action Policy

Keep the factorized policy:

- `mode_head`: play versus discard.
- `count_head`: number of selected cards.
- `card_head`: score/logit for each card slot.

Inference:

```python
mode = argmax(mode_logits)
count = argmax(legal_count_logits) + 1
cards = top_k(card_logits, k=count)
```

Training requires stochastic card selection. Sample `count` cards without
replacement: sample one valid card, mask it, and repeat. The PPO action log
probability is:

```python
log_prob = mode_log_prob + count_log_prob + sum(card_selection_log_probs)
```

Use masks instead of invalid-action penalties. Block discarding with no
discards, playing with no hands, padded cards, and plays over five cards.

The current five-output count head deliberately limits discards to five cards.
For the full game, expand it to eight outputs; mask 6-8 for play and allow them
for discard.

Add a PPO value head:

```python
self.value_head = nn.Linear(hidden_size, 1)
```

## Reward

Use `calculate_game_score(game_state)` only at the terminal step:

```python
reward = calculate_game_score(game_state) if won or lost else 0.0
```

This rewards winning and, among wins, retaining hands and discards. Do not emit
the cumulative game score after every action because it would repeatedly reward
the same progress. Keep the win bonus large enough that preserving resources
does not cost too much win probability.

## Training

Use PPO because the policy has factorized stochastic actions.

1. Collect complete blind rollouts: observations, actions, combined log
   probabilities, values, rewards, terminal flags, and masks.
2. Compute returns and generalized advantage estimates.
3. Optimize the PPO clipped policy objective and value loss.
4. Use entropy regularization early for exploration.
5. Evaluate with deterministic argmax/top-k actions.

Use a discount near 1.0 because episodes are short and finite.

## Baselines and Evaluation

Build before trusting learning results:

- Random legal policy.
- Greedy immediate-score policy using current `best_hand`.
- Discard-table heuristic when immediate score is weak.

Evaluate with held-out fixed deck seeds. Report win rate, mean hands remaining
on wins, mean discards remaining on wins, terminal reward, and each baseline.
Use 10,000 episodes for regular evaluation and 50,000+ episodes for a final
99% win-rate claim with a confidence interval.

## Expansion Order

1. Variable blind targets.
2. Jokers and their planner features.
3. Card enhancements, seals, and editions.
4. Shop actions.
5. Economy, Tarot, Planet, and Spectral planning features.
6. Multi-ante training.
