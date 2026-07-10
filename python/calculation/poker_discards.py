import math
import time

from core.enums import Rank, Suit
from core.models import CARD_STRINGS, Card, Deck


def print_timing(label: str, elapsed_ns: int) -> None:
    elapsed_ms = elapsed_ns / 1_000_000
    elapsed_s = elapsed_ns / 1_000_000_000
    print(
        f"{label:<20} {elapsed_s:>10.6f}s  {elapsed_ms:>10.3f}ms  {elapsed_ns:>12,d}ns"
    )


def odds_for_single_value(
    deck: Deck, hand: list[Card], bucket: list[int], amount: int
) -> tuple[int, float, list[Card]]:
    total_cards = deck.total_cards
    total_draws = math.comb(total_cards, 5)
    max_iter = len(bucket)
    val_weights = {i: 0.0 for i in range(max_iter)}

    best_score = 0.0
    best_option = -1
    best_probability = -1.0

    for val in range(max_iter):
        amount_needed = amount - bucket[val]
        if amount_needed == 0:
            continue

        shift_amount, equal_val = (9, 0b11) if max_iter == 4 else (11, 0b11111)
        score = sum(
            card.score
            for card in hand
            if ((card.card_id >> shift_amount & equal_val) == val)
        )

        deck_val = deck.suits[val] if max_iter == 4 else deck.ranks[val]
        deck_val_count = len(deck_val)

        max_fetch_amount = 5 if deck_val_count > 5 else deck_val_count

        every_other_card = total_cards - deck_val_count
        good_draws = 0
        for fetched_rank in range(amount_needed, max_fetch_amount + 1):
            good_draws += math.comb(deck_val_count, fetched_rank) * math.comb(
                every_other_card, 5 - fetched_rank
            )

        total_probability = good_draws / total_draws

        score_of_cards = sum(card.score for card in deck_val) / deck_val_count
        expected_card_score = score_of_cards * amount_needed
        total_card_score = score + expected_card_score
        val_weights[val] = total_probability * total_card_score

        if total_probability > best_probability:
            best_probability = total_probability
            best_option = val
            best_score = total_card_score

        elif total_probability == best_probability and total_card_score > best_score:
            best_score = total_card_score
            best_option = val

    is_suit = max_iter == 4
    ordered_hand = sorted(
        hand, key=lambda x: val_weights[x.suit if is_suit else x.rank]
    )

    cards_to_discard = ordered_hand[:5]
    return best_option, best_probability, cards_to_discard


def odds_for_double_value(
    deck: Deck, hand: list[Card], bucket: list[int], amount: int
) -> tuple[int, float, list[Card]]:
    total_cards = deck.total_cards
    total_draws = math.comb(total_cards, 5)
    max_iter = len(bucket)
    rank_weights = {i: 0.0 for i in range(max_iter)}

    is_two_pair = amount == 2
    left_amount = amount
    right_amount = 2

    best_score = 0.0
    best_option = -1
    best_probability = -1.0

    for left_rank in range(max_iter):
        left_deck = deck.ranks[left_rank]
        left_deck_count = len(left_deck)

        left_amount_needed = left_amount - bucket[left_rank]
        left_score = sum(card.score for card in hand if card.rank == left_rank)
        right_vals = (
            [i for i in range(left_rank + 1, max_iter)]
            if is_two_pair
            else [i for i in range(max_iter) if i != left_rank]
        )

        for right_rank in right_vals:
            right_deck = deck.ranks[right_rank]
            right_deck_count = len(right_deck)

            right_amount_needed = right_amount - bucket[right_rank]
            if left_amount_needed + right_amount_needed == 0:
                continue

            right_score = sum(card.score for card in hand if card.rank == right_rank)

            left_deck_amount, right_deck_amount = left_deck_count, right_deck_count
            rest_of_deck = total_cards - (left_deck_amount + right_deck_amount)

            right_max_iter = 5 if right_deck_count > 5 else right_deck_count
            left_max_iter = 5 if left_deck_count > 5 else left_deck_count

            good_draws = 0
            if left_amount_needed == 0:
                rest_of_deck = total_cards - right_deck_amount
                good_draws = sum(
                    math.comb(right_deck_amount, right_draw)
                    * math.comb(rest_of_deck, 5 - right_draw)
                    for right_draw in range(right_amount_needed, right_max_iter)
                )
            elif right_amount_needed == 0:
                rest_of_deck = total_cards - left_deck_amount
                good_draws = sum(
                    math.comb(right_deck_amount, left_draw)
                    * math.comb(rest_of_deck, 5 - left_draw)
                    for left_draw in range(left_amount_needed, left_max_iter)
                )
            else:
                rest_of_deck = total_cards - (left_deck_amount + right_deck_amount)
                good_draws = sum(
                    math.comb(left_deck_amount, left_draw)
                    * math.comb(right_deck_amount, right_draw)
                    * math.comb(rest_of_deck, 5 - (left_draw + right_draw))
                    for right_draw in range(right_amount_needed, right_max_iter)
                    for left_draw in range(left_amount_needed, left_max_iter)
                    if right_draw + left_draw <= 5
                )
            probability = good_draws / total_draws

            key = left_rank << 4 | right_rank
            left_expected_score = (
                sum(card.score for card in left_deck)
                / left_deck_count
                * left_amount_needed
            )
            right_expected_score = (
                sum(card.score for card in right_deck)
                / right_deck_count
                * right_amount_needed
            )

            total_score = (
                left_score + right_score + left_expected_score + right_expected_score
            )

            weighted_score = total_score * probability
            rank_weights[left_rank] += weighted_score
            rank_weights[right_rank] += weighted_score

            if best_probability < probability:
                best_probability = probability
                best_option = key
                best_score = total_score

            elif best_probability == probability and total_score > best_score:
                best_score = total_score
                best_option = key

    ordered_hand = sorted(hand, key=lambda x: rank_weights[x.rank])
    cards_to_discard = ordered_hand[:5]

    return best_option, best_probability, cards_to_discard


def odds_for_straight(
    deck: Deck, hand: list[Card], rank_bucket: list[int], straight_length: int
) -> tuple[int, float, list[Card]]:
    rank_order = [
        Rank.ACE,
        Rank.KING,
        Rank.QUEEN,
        Rank.JACK,
        Rank.TEN,
        Rank.NINE,
        Rank.EIGHT,
        Rank.SEVEN,
        Rank.SIX,
        Rank.FIVE,
        Rank.FOUR,
        Rank.THREE,
        Rank.TWO,
        Rank.ACE,  # ace-low support
    ]
    total_cards = deck.total_cards
    total_draws = math.comb(total_cards, 5)
    best_probability = 0.0
    best_option = -1
    best_score = -1
    rank_weights = {i: 0.0 for i in range(len(rank_bucket))}

    for cut_off in range(straight_length, len(rank_order)):
        straight = rank_order[cut_off - straight_length : cut_off]
        straight_cards_needed = [1 - rank_bucket[rank] for rank in straight]

        score = 0
        for rank in straight:
            max_score = 0
            for card in hand:
                if card.rank == rank and card.score > max_score:
                    max_score = card.score

            score += max_score

        good_draw = 1
        fetch_count = 0
        other_cards = total_cards
        for i, amount_needed in enumerate(straight_cards_needed):
            if amount_needed < 1:
                continue

            fetch_count += 1
            deck_rank = deck.ranks[straight[i]]
            deck_rank_amount = len(deck_rank)
            other_cards -= deck_rank_amount
            good_draw *= math.comb(deck_rank_amount, amount_needed)

            score += (
                sum(card.score for card in deck_rank) / deck_rank_amount * amount_needed
            )

        good_draw *= math.comb(other_cards, 5 - fetch_count)
        probability = good_draw / total_draws

        for rank in straight:
            rank_weights[rank] += score * probability

        if probability > best_probability:
            best_probability = probability
            best_score = score
            best_option = 0
            for rank in straight:
                best_option = best_option << 4 | rank

        elif probability == best_probability and score > best_score:
            best_score = score
            best_option = 0
            for rank in straight:
                best_option = best_option << 4 | rank

    straight_weight = []
    rank_count = [0] * len(Rank)
    for card in hand:
        card_decay = math.pow(2, rank_count[card.rank])
        straight_weight.append((card, rank_weights[card.rank] / card_decay))
        rank_count[card.rank] += 1

    ordered_hand = sorted(straight_weight, key=lambda x: x[1])
    cards_to_discard = [card for card, _ in ordered_hand[:5]]

    return best_option, best_probability, cards_to_discard


def odds_for_straigh_flush(
    deck: Deck, hand: list[Card], straight_length: int
) -> tuple[int, float, list[Card]]:
    rank_order = [
        Rank.ACE,
        Rank.KING,
        Rank.QUEEN,
        Rank.JACK,
        Rank.TEN,
        Rank.NINE,
        Rank.EIGHT,
        Rank.SEVEN,
        Rank.SIX,
        Rank.FIVE,
        Rank.FOUR,
        Rank.THREE,
        Rank.TWO,
        Rank.ACE,  # ace-low support
    ]
    total_cards = deck.total_cards
    total_draws = math.comb(total_cards, 5)
    best_probability = -1.0
    best_option = -1
    best_score = 0.0
    suit_rank_weights = {suit << 4 | rank: 0.0 for suit in Suit for rank in Rank}

    for suit in Suit:
        for cut_off in range(straight_length, len(rank_order)):
            straight = rank_order[cut_off - straight_length : cut_off]
            straight_cards_needed = [
                1 - sum(1 for card in hand if card.rank == rank and card.suit == suit)
                for rank in straight
            ]

            score = 0
            for rank in straight:
                max_score = 0
                for card in hand:
                    if (
                        card.rank == rank
                        and card.suit == suit
                        and card.score > max_score
                    ):
                        max_score = card.score

                score += max_score

            good_draw = 1
            fetch_count = 0
            other_cards = total_cards
            for i, amount_needed in enumerate(straight_cards_needed):
                if amount_needed < 1:
                    continue

                fetch_count += 1
                suit_rank_key = suit << 4 | straight[i]
                deck_suit_rank = deck.suit_rank[suit_rank_key]
                deck_suit_rank_amount = len(deck_suit_rank)
                other_cards -= deck_suit_rank_amount
                good_draw *= math.comb(deck_suit_rank_amount, amount_needed)

                score += (
                    sum(card.score for card in deck_suit_rank)
                    / deck_suit_rank_amount
                    * amount_needed
                )

            good_draw *= math.comb(other_cards, 5 - fetch_count)
            probability = good_draw / total_draws

            for rank in straight:
                suit_rank_weights[suit << 4 | rank] += score * probability

            if probability > best_probability:
                best_probability = probability
                best_score = score
                best_option = 0
                for rank in straight:
                    best_option = ((best_option << 2) | suit) << 4 | rank

            elif probability == best_probability and score > best_score:
                best_score = score
                best_option = 0
                for rank in straight:
                    best_option = ((best_option << 2) | suit) << 4 | rank

    straight_weight = []
    suit_rank_count = [0] * (Suit.SPADES << 4 | Rank.ACE)
    for card in hand:
        suit_rank_key = card.suit << 4 | card.rank
        card_decay = math.pow(2, suit_rank_count[suit_rank_key])
        straight_weight.append((card, suit_rank_weights[suit_rank_key] / card_decay))
        suit_rank_count[suit_rank_key] += 1

    ordered_hand = sorted(straight_weight, key=lambda x: x[1])
    cards_to_discard = [card for card, _ in ordered_hand[:5]]

    return best_option, best_probability, cards_to_discard


def odds_for_flush_house(deck: Deck, hand: list[Card]) -> tuple[int, float, list[Card]]:
    total_cards = deck.total_cards
    total_draws = math.comb(total_cards, 5)
    suit_rank_weights = {suit << 4 | rank: 0.0 for suit in Suit for rank in Rank}

    left_amount = 3
    right_amount = 2

    best_score = -1
    best_option = -1
    best_probability = 0.0

    for suit in Suit:
        for left_rank in Rank:
            left_suit_rank_key = suit << 4 | left_rank
            left_deck = deck.suit_rank[left_suit_rank_key]
            left_deck_count = len(left_deck)

            left_count, left_score = 0, 0
            for card in hand:
                if card.rank == left_rank and card.suit == suit:
                    left_count += 1
                    left_score += card.score

            left_amount_needed = left_amount - left_count
            if left_deck_count + left_count < left_amount:
                continue  # we don't have enough cards to fetch from the deck

            right_vals = [rank for rank in Rank if rank != left_rank]

            for right_rank in right_vals:
                right_suit_rank_key = suit << 4 | right_rank
                right_deck = deck.suit_rank[right_suit_rank_key]
                right_deck_count = len(right_deck)

                right_count, right_score = 0, 0
                for card in hand:
                    if card.rank == right_rank and card.suit == suit:
                        right_count += 1
                        right_score += card.score

                right_amount_needed = right_amount - right_count
                if right_deck_count + right_count < right_amount:
                    continue  # we don't have enough cards to fetch from the deck

                if left_amount_needed + right_amount_needed == 0:
                    continue  # this is already the hand we are looking for
                    # lets try to find another

                rest_of_deck = total_cards - (left_deck_count + right_deck_count)

                right_max_iter = 5 if right_deck_count > 5 else right_deck_count
                left_max_iter = 5 if left_deck_count > 5 else left_deck_count

                good_draws = 0
                if left_amount_needed == 0:
                    rest_of_deck = total_cards - right_deck_count
                    good_draws = sum(
                        math.comb(right_deck_count, right_draw)
                        * math.comb(rest_of_deck, 5 - right_draw)
                        for right_draw in range(right_amount_needed, right_max_iter)
                    )
                elif right_amount_needed == 0:
                    rest_of_deck = total_cards - left_deck_count
                    good_draws = sum(
                        math.comb(left_deck_count, left_draw)
                        * math.comb(rest_of_deck, 5 - left_draw)
                        for left_draw in range(left_amount_needed, left_max_iter)
                    )
                else:
                    rest_of_deck = total_cards - (left_deck_count + right_deck_count)
                    good_draws = sum(
                        math.comb(left_deck_count, left_draw)
                        * math.comb(right_deck_count, right_draw)
                        * math.comb(rest_of_deck, 5 - (left_draw + right_draw))
                        for right_draw in range(right_amount_needed, right_max_iter)
                        for left_draw in range(left_amount_needed, left_max_iter)
                        if right_draw + left_draw <= 5
                    )
                probability = good_draws / total_draws

                key = left_suit_rank_key << 6 | right_suit_rank_key
                left_expected_score = (
                    sum(card.score for card in left_deck)
                    / left_deck_count
                    * left_amount_needed
                )
                right_expected_score = (
                    sum(card.score for card in right_deck)
                    / right_deck_count
                    * right_amount_needed
                )

                total_score = (
                    left_score
                    + right_score
                    + left_expected_score
                    + right_expected_score
                )

                weighted_score = total_score * probability
                suit_rank_weights[left_suit_rank_key] += weighted_score
                suit_rank_weights[right_suit_rank_key] += weighted_score

                if best_probability < probability:
                    best_probability = probability
                    best_option = key
                    best_score = total_score

                elif best_probability == probability and total_score > best_score:
                    best_score = total_score
                    best_option = key

    ordered_hand = sorted(hand, key=lambda x: suit_rank_weights[x.suit << 4 | x.rank])
    cards_to_discard = ordered_hand[:5]

    return best_option, best_probability, cards_to_discard


def odds_for_flush_five(deck: Deck, hand: list[Card]) -> tuple[int, float, list[Card]]:
    total_cards = deck.total_cards
    total_draws = math.comb(total_cards, 5)
    suit_rank_weights = {suit << 4 | rank: 0.0 for suit in Suit for rank in Rank}

    best_score = -1.0
    best_option = -1
    best_probability = 0.0

    for suit in Suit:
        for rank in Rank:
            count, score = 0, 0
            for card in hand:
                if card.rank == rank and card.suit == suit:
                    count += 1
                    score += card.score

            amount_needed = 5 - count
            if amount_needed == 0:
                continue

            suit_rank_key = suit << 4 | rank
            deck_val = deck.suit_rank[suit_rank_key]
            deck_val_count = len(deck_val)

            if deck_val_count + count < 5:
                continue  # we don't have enough cards to get this

            max_fetch_amount = 5 if deck_val_count > 5 else deck_val_count

            every_other_card = total_cards - deck_val_count
            good_draws = 0
            for fetched_rank in range(amount_needed, max_fetch_amount + 1):
                good_draws += math.comb(deck_val_count, fetched_rank) * math.comb(
                    every_other_card, 5 - fetched_rank
                )

            total_probability = good_draws / total_draws

            score_of_cards = sum(card.score for card in deck_val) / deck_val_count
            expected_card_score = score_of_cards * amount_needed
            total_card_score = score + expected_card_score
            suit_rank_weights[suit_rank_key] = total_probability * total_card_score

            if total_probability > best_probability:
                best_probability = total_probability
                best_option = rank
                best_score = total_card_score

            elif (
                total_probability == best_probability and total_card_score > best_score
            ):
                best_score = total_card_score
                best_option = rank

    ordered_hand = sorted(
        hand, key=lambda x: suit_rank_weights[x.suit << 4 | x.rank]
    )

    cards_to_discard = ordered_hand[:5]
    return best_option, best_probability, cards_to_discard


def calculate_odds(deck: Deck, dealt_cards: list[Card]):
    suit_bucket = [0] * 4
    rank_bucket = [0] * 13
    for card in dealt_cards:
        suit_bucket[card.suit] += 1
        rank_bucket[card.rank] += 1

    total_start = time.perf_counter_ns()

    pair_start = time.perf_counter_ns()
    pair_val, pair_prob, pair_discards = odds_for_single_value(
        deck, dealt_cards, rank_bucket, 2
    )
    pair_end = time.perf_counter_ns()
    print_timing("pair odds", pair_end - pair_start)

    three_start = time.perf_counter_ns()
    three_val, three_prob, three_discards = odds_for_single_value(
        deck, dealt_cards, rank_bucket, 3
    )
    three_end = time.perf_counter_ns()
    print_timing("three of a kind", three_end - three_start)

    four_start = time.perf_counter_ns()
    four_val, four_prob, four_discards = odds_for_single_value(
        deck, dealt_cards, rank_bucket, 4
    )
    four_end = time.perf_counter_ns()
    print_timing("four of a kind", four_end - four_start)

    five_start = time.perf_counter_ns()
    five_val, five_prob, five_discards = odds_for_single_value(
        deck, dealt_cards, rank_bucket, 5
    )
    five_end = time.perf_counter_ns()
    print_timing("five of a kind", five_end - five_start)

    flush_start = time.perf_counter_ns()
    flush_val, flush_prob, flush_discards = odds_for_single_value(
        deck, dealt_cards, suit_bucket, 5
    )
    flush_end = time.perf_counter_ns()
    print_timing("flush odds", flush_end - flush_start)

    two_pair_start = time.perf_counter_ns()
    two_pair_val, two_pair_prob, two_pair_discards = odds_for_double_value(
        deck, dealt_cards, rank_bucket, 2
    )
    two_pair_end = time.perf_counter_ns()
    print_timing("two pair odds", two_pair_end - two_pair_start)

    full_house_start = time.perf_counter_ns()
    full_house_val, full_house_prob, full_house_discards = odds_for_double_value(
        deck, dealt_cards, rank_bucket, 3
    )
    full_house_end = time.perf_counter_ns()
    print_timing("full house odds", full_house_end - full_house_start)

    straight_start = time.perf_counter_ns()
    straight_val, straight_prob, straight_discards = odds_for_straight(
        deck, dealt_cards, rank_bucket, 5
    )
    straight_end = time.perf_counter_ns()
    print_timing("straight odds", straight_end - straight_start)

    straight_flush_start = time.perf_counter_ns()
    straight_flush_val, straight_flush_prob, straight_flush_discards = (
        odds_for_straigh_flush(deck, dealt_cards, 5)
    )
    straight_flush_end = time.perf_counter_ns()
    print_timing("straight flush odds", straight_flush_end - straight_flush_start)

    flush_house_start = time.perf_counter_ns()
    flush_house_val, flush_house_prob, flush_house_discards = odds_for_flush_house(
        deck, dealt_cards
    )
    flush_house_end = time.perf_counter_ns()
    print_timing("flush house odds", flush_house_end - flush_house_start)

    flush_five_start = time.perf_counter_ns()
    flush_five_val, flush_five_prob, flush_five_discards = odds_for_flush_five(
        deck, dealt_cards
    )
    flush_five_end = time.perf_counter_ns()
    print_timing("flush five odds", flush_five_end - flush_five_start)

    total_end = time.perf_counter_ns()
    print_timing("total odds time", total_end - total_start)

    _rank_short = {r: CARD_STRINGS[r] for r in Rank}
    _suit_short = {s: s.name[0] for s in Suit}

    def _short(card: Card) -> str:
        return f"{_rank_short[card.rank]}{_suit_short[card.suit]}"

    def _fmt_val(val) -> str:
        if isinstance(val, tuple):
            return "(" + ", ".join(str(v) for v in val) + ")"
        return str(val)

    rows = [
        ("Pair", pair_val, pair_prob, pair_discards, pair_end - pair_start),
        ("Three of a Kind", three_val, three_prob, three_discards, three_end - three_start),
        ("Four of a Kind", four_val, four_prob, four_discards, four_end - four_start),
        ("Five of a Kind", five_val, five_prob, five_discards, five_end - five_start),
        ("Flush", flush_val, flush_prob, flush_discards, flush_end - flush_start),
        ("Two Pair", two_pair_val, two_pair_prob, two_pair_discards, two_pair_end - two_pair_start),
        ("Full House", full_house_val, full_house_prob, full_house_discards, full_house_end - full_house_start),
        ("Straight", straight_val, straight_prob, straight_discards, straight_end - straight_start),
        ("Straight Flush", straight_flush_val, straight_flush_prob, straight_flush_discards, straight_flush_end - straight_flush_start),
        ("Flush House", flush_house_val, flush_house_prob, flush_house_discards, flush_house_end - flush_house_start),
        ("Flush Five", flush_five_val, flush_five_prob, flush_five_discards, flush_five_end - flush_five_start),
    ]

    print()
    print(f"{'Hand':<18} {'Probability':>12} {'Val':>10} {'Time':>10}  Discards")
    print("-" * 80)
    for name, val, prob, discards, elapsed_ns in rows:
        disc_str = ", ".join(_short(c) for c in discards) if discards else "-"
        print(f"{name:<18} {prob:>11.2%} {_fmt_val(val):>10} {elapsed_ns / 1_000_000:>9.3f}ms  {disc_str}")
    print("-" * 80)
    print(f"Total time taken was: {(total_end - total_start) / 1_000_000:.3f}ms")


if __name__ == "__main__":
    deck = Deck()

    hand = [
        Card(
            rank=Rank.ACE,
            suit=Suit.DIAMONDS,
        ),
        Card(
            rank=Rank.QUEEN,
            suit=Suit.HEARTS,
        ),
        Card(
            rank=Rank.TEN,
            suit=Suit.DIAMONDS,
        ),
        Card(
            rank=Rank.EIGHT,
            suit=Suit.HEARTS,
        ),
        Card(
            rank=Rank.SEVEN,
            suit=Suit.HEARTS,
        ),
        Card(
            rank=Rank.SIX,
            suit=Suit.CLUBS,
        ),
        Card(
            rank=Rank.SIX,
            suit=Suit.DIAMONDS,
        ),
        Card(
            rank=Rank.FIVE,
            suit=Suit.CLUBS,
        ),
    ]

    deck.filter(hand)
    calculate_odds(deck, hand)
