# Joker Test Implementation Plan

This is the behavioral test checklist for all 150 Joker kinds. Each Joker gets at least one base case and one boundary/negative case. Tests should exercise the public event path (`Jokers::trigger`, `Jokers::update`, or `Jokers::sell`) rather than calling private implementation helpers directly.

## Conventions

- **Base** means the normal condition described by the Joker is satisfied once.
- **Edge** means a nearby condition that must not trigger, a threshold boundary, an empty collection, a probability boundary, or a lifecycle interaction.
- `implemented` means the current engine has enough state and event plumbing to test the behavior now.
- `planned` means the behavior is known but still needs a test module.
- `blocked` means the engine needs a missing domain model or behavior implementation before a meaningful test can be written.
- Tests are grouped by behavior in `src/core/tests/`; they are not required to mirror the 150-case production match statement.

## Current implementation batches

- `joker_triggers.rs`: basic scoring conditions and dynamic scoring.
- `joker_retriggers.rs`: Mime, Sock and Buskin, and Hanging Chad.
- `joker_economy.rs`: Mail-In Rebate, Golden Ticket, and Matador.
- `joker_updates.rs`: lifecycle and state updates already implemented in the engine.
- `joker_scoring.rs`: simple scoring-condition coverage added in the first batch.

The remaining rows are the backlog. A `blocked` row should move to `planned` only after the required game-state API exists.

## Joker-by-joker matrix

| # | Joker | Base case | Edge case(s) | Test path / status |
|---:|---|---|---|---|
| 1 | Joker | Score any played card for +4 Mult. | Debuffed or no played card gives no effect. | scoring / implemented |
| 2 | Greedy Joker | Score a Diamond for +3 Mult. | Non-Diamond does not score. | scoring / implemented |
| 3 | Lusty Joker | Score a Heart for +3 Mult. | Non-Heart does not score. | scoring / implemented |
| 4 | Wrathful Joker | Score a Spade for +3 Mult. | Non-Spade does not score. | scoring / implemented |
| 5 | Gluttonous Joker | Score a Club for +3 Mult. | Non-Club does not score. | scoring / implemented |
| 6 | Jolly Joker | Pair scores +8 Mult. | High Card or Three of a Kind does not score. | scoring / implemented |
| 7 | Zany Joker | Three of a Kind scores +12 Mult. | Pair does not score. | scoring / implemented |
| 8 | Mad Joker | Two Pair scores +10 Mult in the current implementation. | Pair does not score; wiki-value reconciliation remains. | scoring / implemented |
| 9 | Crazy Joker | Straight scores +12 Mult. | Flush does not score. | scoring / implemented |
| 10 | Droll Joker | Flush scores +10 Mult. | Straight does not score. | scoring / implemented |
| 11 | Sly Joker | Pair scores +50 Chips. | High Card does not score. | scoring / implemented |
| 12 | Wily Joker | Three of a Kind scores +100 Chips. | Pair does not score. | scoring / implemented |
| 13 | Clever Joker | Two Pair scores +80 Chips in the current implementation. | Pair does not score; wiki-value reconciliation remains. | scoring / implemented |
| 14 | Devious Joker | Straight scores +100 Chips. | Flush does not score. | scoring / implemented |
| 15 | Crafty Joker | Flush scores +80 Chips. | Straight does not score. | scoring / implemented |
| 16 | Half Joker | Three or fewer played cards scores +20 Mult. | Four cards does not score; zero cards stays safe. | scoring / implemented |
| 17 | Stencil | Empty Joker slots increase XMult. | Full capacity has the minimum bonus; negative edition slot rules need coverage. | scoring / implemented |
| 18 | Four Fingers | Four-card Straight/Flush is accepted. | Three-card hand is rejected. | game rules / blocked |
| 19 | Mime | Held-card abilities retrigger. | Played cards are not treated as held cards. | retrigger / implemented |
| 20 | Credit Card | Debt/negative money is allowed. | Spending remains bounded by the configured credit limit. | game rules / blocked |
| 21 | Ceremonial Dagger | At blind selection, destroy the right Joker and gain its sell value as Mult. | Empty/rightmost-only lineup and self-targeting must be safe. | update / implemented |
| 22 | Banner | Each remaining discard adds Chips. | Zero discards and exhausted discards. | scoring / implemented |
| 23 | Mystic Summit | No discards remaining gives +15 Mult. | One remaining discard gives no bonus. | scoring / implemented |
| 24 | Marble Joker | At blind selection, add a Stone card. | Full deck/hand capacity and repeated selection. | update / blocked |
| 25 | Loyalty Card | Every sixth hand gets XMult. | Fifth and seventh hands do not trigger early/again. | scoring / blocked |
| 26 | 8 Ball | Played 8 has a chance to create a Planet card. | Non-8 and failed probability path. | trigger / blocked |
| 27 | Misprint | Score with a random Mult in its range. | Range endpoints and deterministic seeded RNG. | scoring / implemented |
| 28 | Dusk | Final hand of the round retriggers played cards. | Non-final hand does not retrigger. | retrigger / blocked |
| 29 | Raised Fist | Lowest held card scores twice its rank as Mult. | Ties and an empty hand. | scoring / blocked |
| 30 | Chaos the Clown | First shop reroll is free. | Later rerolls still cost money. | update / blocked |
| 31 | Fibonacci | 2–5, 8, Ace played cards score +8 Mult/+40 Chips. | 6, 7, and face cards do not trigger. | scoring / implemented |
| 32 | Steel Joker | Steel cards held in hand give XMult. | Non-Steel card and no held cards. | scoring / implemented |
| 33 | Scary Face | Face card gives +30 Chips. | Number card does not score. | scoring / implemented |
| 34 | Abstract Joker | Each Joker gives +3 Mult. | Empty lineup and lineup growth. | scoring / implemented |
| 35 | Delayed Gratification | Finish a round without discarding to gain money. | Any discard prevents payment. | update / blocked |
| 36 | Hack | 2–5 played cards retrigger. | 6–Ace does not retrigger. | retrigger / blocked |
| 37 | Pareidolia | All cards count as face cards. | Face-dependent Joker works on a number card. | game rules / blocked |
| 38 | Gros Michel | Score +15 Mult while alive. | One-in-six end-round failure removes it; seeded success survives. | update / implemented |
| 39 | Even Steven | Even-rank card gives +4 Mult. | Odd-rank card does not score. | scoring / implemented |
| 40 | Odd Todd | Odd-rank card gives +31 Chips. | Even-rank card does not score. | scoring / implemented |
| 41 | Scholar | Ace gives +20 Chips/+4 Mult. | Non-Ace does not score. | scoring / implemented |
| 42 | Business Card | Face card has a chance to earn money. | Number card and failed probability path. | trigger / blocked |
| 43 | Supernova | Each scored poker hand type increases Mult. | Repeating a hand and a different hand use separate counts. | update / blocked |
| 44 | Ride the Bus | Consecutive non-face hands increase Mult. | A face card resets the streak. | update / planned |
| 45 | Space Joker | Played hand has a chance to level up. | Failed probability and empty hand. | trigger / blocked |
| 46 | Egg | End round increases sell value. | Multiple rounds accumulate; no scoring side effect. | update / implemented |
| 47 | Burglar | Blind selection gives hands and removes discards. | Cannot go below allowed discard floor. | update / implemented |
| 48 | Blackboard | All cards held in hand are Spades/Clubs for XMult. | One Heart/Diamond disables the bonus. | scoring / implemented |
| 49 | Runner | Straight hand increases Chips. | Non-Straight hand does not increase it. | update / implemented |
| 50 | Ice Cream | Each played hand loses Chips; it expires at zero. | Exact zero and one hand after expiration. | update / implemented |
| 51 | DNA | Single-card hand duplicates the card. | Two-card hand does not duplicate. | trigger / blocked |
| 52 | Splash | Every played card counts in scoring. | A non-scoring card still contributes. | scoring / blocked |
| 53 | Blue Joker | Each card remaining in deck gives Chips. | Empty deck and one-card deck. | scoring / implemented |
| 54 | Sixth Sense | Single 6 hand destroys it and creates a Spectral card. | Non-6 and multi-card hand do not trigger. | trigger / blocked |
| 55 | Constellation | Planet card use increases XMult. | Tarot/Spectral use does not increase it. | update / implemented |
| 56 | Hiker | Each played card permanently gains Chips. | Same card cannot be double-counted accidentally. | trigger / blocked |
| 57 | Faceless Joker | Sell 3+ face cards for money. | Selling two face cards does not pay. | update / blocked |
| 58 | Green Joker | Played hands add Mult; discards remove Mult. | Exact zero hands/discards and no underflow. | update / implemented |
| 59 | Superposition | Straight containing an Ace creates a Tarot card. | Straight without Ace and non-Straight do not trigger. | trigger / blocked |
| 60 | To Do List | Matching poker hand earns money. | Nonmatching hand and hand rotation. | update / implemented |
| 61 | Cavendish | Score +3XMult while alive. | One-in-1000 end-round failure removes it. | update / implemented |
| 62 | Card Sharp | Repeated poker hand scores XMult. | First occurrence and different hand do not trigger. | scoring / implemented |
| 63 | Red Card | Booster skip increases Mult. | Opening a pack does not increase it. | update / planned |
| 64 | Madness | Blind selection gives XMult and may destroy a random Joker. | Empty lineup and self-destruction. | update / implemented |
| 65 | Square Joker | Four-card hand increases Chips. | Three- and five-card hands do not increase it. | update / planned |
| 66 | Séance | Straight Flush creates a Spectral card. | Other poker hands do not trigger. | trigger / blocked |
| 67 | Riff-Raff | Blind selection creates two Common Jokers. | Full lineup/insufficient slots. | update / blocked |
| 68 | Vampire | Played enhanced cards increase XMult and lose enhancements. | Unenhanced card and multiple cards. | trigger / blocked |
| 69 | Shortcut | Straights may contain one rank gap. | Two gaps and ordinary invalid hand. | game rules / blocked |
| 70 | Hologram | Added cards increase XMult. | Non-addition deck events do not increase it. | update / implemented |
| 71 | Vagabond | Low-money hand creates a Tarot card. | Money at/above threshold and probability failure. | trigger / blocked |
| 72 | Baron | Held Kings give XMult. | Non-King and no held Kings. | scoring / implemented |
| 73 | Cloud 9 | End round pays for 9s remaining in deck. | Zero 9s pays zero. | update / implemented |
| 74 | Rocket | End round pays money; boss completion increases payout. | Normal vs boss completion and debuffed state. | update / implemented |
| 75 | Obelisk | Consecutive hands without the most-played hand increase XMult. | Playing the dominant hand resets the streak. | update / planned |
| 76 | Midas Mask | Played face cards become Gold. | Number card remains unchanged. | trigger / blocked |
| 77 | Luchador | Selling it disables the current boss blind. | Non-boss context has no extra effect. | update / implemented |
| 78 | Photograph | First played face card gives XMult. | Second face card and non-face card. | scoring / planned |
| 79 | Gift Card | End round increases held card sell values. | Multiple rounds accumulate; unrelated state unchanged. | update / implemented |
| 80 | Turtle Bean | Round hand size decreases over time. | Minimum hand size is not crossed. | update / planned |
| 81 | Erosion | Missing deck cards add Mult. | Full deck gives no bonus; one missing card gives exactly one increment. | scoring / implemented |
| 82 | Reserved Parking | Held face cards have a chance to earn money. | Non-face card and failed probability. | trigger / blocked |
| 83 | Mail-In Rebate | Matching discarded rank earns money. | No matching cards and empty discard. | trigger / implemented |
| 84 | To the Moon | End round earns interest-based money. | Zero money and interest cap boundary. | update / implemented |
| 85 | Hallucination | Opening a booster can create a Tarot card. | Non-booster event and failed probability. | trigger / implemented |
| 86 | Fortune Teller | Tarot use increases Mult. | Planet/Spectral use does not increase it. | update / implemented |
| 87 | Juggler | Hand size increases by one. | Hand size remains valid after sell/removal. | static / blocked |
| 88 | Drunkard | Discard limit increases by one. | Round reset preserves the configured bonus. | static / blocked |
| 89 | Stone Joker | Each Stone card in deck gives Chips. | Zero Stone cards and one Stone card. | scoring / blocked |
| 90 | Golden Joker | End round gives money. | Debuffed/inactive Joker gives no payout. | update / implemented |
| 91 | Lucky Cat | Lucky card success increases XMult. | Failed lucky roll does not increase it. | update / implemented |
| 92 | Baseball Card | Uncommon Jokers give XMult. | Common/Rare/Legendary classifications. | scoring / blocked |
| 93 | Bull | Money gives Chips. | Zero money and negative-money boundary. | scoring / implemented |
| 94 | Diet Cola | Selling it creates a Double Tag. | No tag generation when debuffed. | update / implemented |
| 95 | Trading Card | Final discard destroys a card and earns money. | Non-final discard and empty discard. | trigger / blocked |
| 96 | Flash Card | Shop reroll increases Mult. | First reroll and no reroll. | update / implemented |
| 97 | Popcorn | End round loses Mult. | Exact zero and expiration behavior. | update / planned |
| 98 | Spare Trousers | Two Pair increases Mult. | Other hands do not increase it. | update / planned |
| 99 | Ancient Joker | Matching suit scores XMult; suit rotates at round lifecycle. | Nonmatching suit and rotation boundary. | trigger/update / implemented |
| 100 | Ramen | Discarded cards reduce XMult. | XMult clamps at 1.0. | trigger / implemented |
| 101 | Walkie Talkie | Played 10/4 gives Chips and Mult. | Other ranks do not score. | scoring / planned |
| 102 | Seltzer | Retriggers cards for a limited number of hands. | Exactly final active hand and after expiration. | update/retrigger / implemented |
| 103 | Castle | Discarded target-suit cards give Chips; target suit rotates. | Non-target discard and empty discard. | trigger/update / implemented |
| 104 | Smiley Face | Face cards give Mult. | Number card does not score. | scoring / implemented |
| 105 | Campfire | Selling cards increases XMult; boss completion resets it. | No sale and reset after boss. | update / implemented |
| 106 | Golden Ticket | Played Gold cards earn money. | Non-Gold card and zero matching cards. | trigger / implemented |
| 107 | Mr. Bones | Prevents death when score reaches 25% requirement, then removes itself. | Below threshold survives no-death effect; exact threshold. | update / implemented |
| 108 | Acrobat | Final hand retriggers played cards. | Non-final hand does not retrigger. | retrigger / blocked |
| 109 | Sock and Buskin | Retriggers played face cards. | Non-face card does not retrigger. | retrigger / implemented |
| 110 | Swashbuckler | Joker sell values add to Mult. | Empty lineup and selling a Joker. | scoring / blocked |
| 111 | Troubadour | Hand size increases and hands per round decrease. | Round reset and minimum hands. | static / blocked |
| 112 | Certificate | Round start creates a card with a random seal. | Invalid/empty deck and deterministic RNG. | update / implemented |
| 113 | Smeared Joker | Hearts/Diamonds and Clubs/Spades are treated as the same suit. | Cross-color suit mismatch remains invalid. | game rules / blocked |
| 114 | Throwback | Skipped blinds increase XMult. | Non-skip events do not increase it. | update / planned |
| 115 | Hanging Chad | Retriggers first played card twice. | Second card and empty played cards. | retrigger / implemented |
| 116 | Rough Gem | Diamond cards earn money. | Non-Diamond card earns nothing. | trigger / planned |
| 117 | Bloodstone | Hearts have a chance to give XMult. | Non-Heart and failed probability. | scoring / implemented |
| 118 | Arrowhead | Spades give Chips. | Non-Spade does not score. | scoring / planned |
| 119 | Onyx Agate | Clubs give Mult. | Non-Club does not score. | scoring / planned |
| 120 | Glass Joker | Glass-card destruction gives XMult. | Non-Glass destruction and debuffed state. | update / implemented |
| 121 | Showman | Allows duplicate Jokers/consumables/vouchers. | Duplicate prevention remains for unrelated objects. | game rules / blocked |
| 122 | Flower Pot | Face + suit diversity scores XMult. | Missing one required suit/face fails. | scoring / implemented |
| 123 | Blueprint | Copies the right Joker ability. | No right neighbor and lineup reorder. | copy/update / blocked |
| 124 | Wee Joker | Played 2s increase Chips. | Non-2 rank and multiple 2s. | trigger / blocked |
| 125 | Merry Andy | Discard limit increases and hand size decreases. | Round reset preserves both modifiers. | static / blocked |
| 126 | Oops! All 6s | Doubles probability values. | Probability one, zero, and multiple probability Jokers. | game rules / blocked |
| 127 | The Idol | Chosen rank+suit scores XMult; target changes after blind. | Non-target card and target reroll boundary. | update/scoring / implemented |
| 128 | Seeing Double | Club plus another suit in the same hand gives XMult. | All Clubs or no Club fails. | scoring / implemented |
| 129 | Matador | Boss-blind ability trigger earns money. | Ordinary trigger does not pay. | trigger / implemented |
| 130 | Hit the Road | Discarding Jacks increases XMult. | Discard without Jacks and round-start state. | update / planned |
| 131 | The Duo | Pair scores XMult. | Non-pair does not score. | scoring / implemented |
| 132 | The Trio | Three of a Kind scores XMult. | Pair does not score. | scoring / implemented |
| 133 | The Family | Four of a Kind scores XMult. | Full House does not score. | scoring / implemented |
| 134 | The Order | Straight scores XMult. | Straight Flush is evaluated according to the intended ordering rule. | scoring / implemented |
| 135 | The Tribe | Flush scores XMult. | Straight does not score. | scoring / implemented |
| 136 | Stuntman | High hand-size requirement gives Chips and reduces hand size. | Below threshold and repeated round changes. | static/scoring / blocked |
| 137 | Invisible Joker | After two rounds, selling it creates a Joker. | One round and insufficient slots. | update / implemented |
| 138 | Brainstorm | Copies the leftmost Joker ability. | Empty lineup and self-copy/order change. | copy/update / blocked |
| 139 | Satellite | End round pays for unique Planet cards used. | Duplicate Planet cards count once; none pays zero. | update / implemented |
| 140 | Shoot the Moon | Held Queens give Mult. | Non-Queen and empty held hand. | scoring / implemented |
| 141 | Driver's License | At least 16 enhanced cards gives XMult. | Fifteen enhanced cards fails; exactly sixteen passes. | scoring / implemented |
| 142 | Cartomancer | Booster opening creates a Tarot card. | Non-booster event and failed probability. | trigger / blocked |
| 143 | Astronomer | Planet cards in shop are free. | Non-Planet cards still cost money. | static / blocked |
| 144 | Burnt Joker | Discarding one card destroys it and improves hand level. | Multi-card discard and empty discard. | trigger / blocked |
| 145 | Bootstraps | Chips scale with money and Mult scales with Chips. | Zero values and threshold changes. | scoring / implemented |
| 146 | Canio | Destroyed face cards increase XMult. | Destroyed number card does not increase it. | update / implemented |
| 147 | Triboulet | Kings and Queens give XMult. | Other ranks do not score. | scoring / implemented |
| 148 | Yorick | Discarding enough cards increases XMult. | One below threshold and exact threshold. | update / planned |
| 149 | Chicot | Disables boss-blind abilities. | Ordinary blinds are unaffected. | static / blocked |
| 150 | Perkeo | Shop close creates a negative copy of a consumable. | Empty consumable area and no-copy case. | update / implemented |

## Execution order

1. Finish direct scoring and retrigger tests in `joker_scoring.rs` and `joker_retriggers.rs`.
2. Finish event-driven update tests in `joker_updates.rs` and `joker_economy.rs`.
3. Add generation tests once generated cards/consumables have stable inspection APIs.
4. Add copy, static modifier, probability, and game-rule tests after those behaviors are represented in `GameState`.
5. Run the full suite after each batch; keep every test deterministic by supplying a seeded `GameState`.
