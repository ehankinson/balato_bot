from itertools import combinations, permutations, product

from calculation.util import bucket_rank, bucket_suit
from core.enums import Enhancement, Rank, Suit
from core.models import Card


def is_same_enhancement(cards: list[Card]) -> bool:
    check = cards[0].enhancement
    return all(card.enhancement == check for card in cards)


def is_same_edition(cards: list[Card]) -> bool:
    check = cards[0].edition
    return all(card.edition == check for card in cards)


def generate_same_rank_groups(hand_size: int, cards: list[Card]) -> list[list[Card]]:
    if is_same_enhancement(cards) and is_same_edition(cards):
        return [cards[:hand_size]]

    final_results = []
    options = [list(val) for val in combinations(cards, hand_size)]
    seen: set[tuple[int, ...]] = set()

    for opt in options:
        order = calculate_order(opt)
        for possible_hand in order:
            hash_obj = tuple(card.card_id for card in possible_hand)
            if hash_obj not in seen:
                seen.add(hash_obj)
                final_results.append(possible_hand)

    return final_results


def generate_n_of_a_kind(bucket: dict[Rank, list[Card]]) -> list[list[Card]]:
    x_of_a_kind: list[list[Card]] = []
    for card_values in bucket.values():
        for size in range(2, len(card_values) + 1):
            x_of_a_kind.extend(generate_same_rank_groups(size, card_values))

    return x_of_a_kind


def generate_flushes(bucket: dict[Suit, list[Card]]) -> list[list[Card]]:
    flushes: list[list[Card]] = []
    for card_values in bucket.values():
        if len(card_values) > 4:
            combos = [list(flush) for flush in combinations(card_values, 5)]
            for com in combos:
                order = calculate_order(com)
                flushes.extend(order)

    return flushes


def generate_2_pair(bucket: dict[Rank, list[Card]]) -> list[list[Card]]:
    pair_options: dict[Rank, list[list[Card]]] = {
        rank: generate_same_rank_groups(2, cards)
        for rank, cards in bucket.items()
        if len(cards) >= 2
    }

    seen: set[tuple[int, ...]] = set()
    final_results: list[list[Card]] = []
    for rank1, rank2 in combinations(pair_options.keys(), 2):
        for pair1, pair2 in product(pair_options[rank1], pair_options[rank2]):
            order = calculate_order(pair1 + pair2)
            for ord in order:
                hash_obj = tuple(card.card_id for card in pair1 + pair2)
                if hash_obj not in seen:
                    seen.add(hash_obj)
                    final_results.append(ord)

    return final_results


def generate_full_house(bucket: dict[Rank, list[Card]]) -> list[list[Card]]:
    pair_options: dict[Rank, list[list[Card]]] = {}
    triple_options: dict[Rank, list[list[Card]]] = {}

    for rank, cards in bucket.items():
        if len(cards) >= 2:
            pair_options[rank] = generate_same_rank_groups(2, cards)

        if len(cards) >= 3:
            triple_options[rank] = generate_same_rank_groups(3, cards)

    hands: list[list[Card]] = []

    for triple_rank, triples in triple_options.items():
        for pair_rank, pairs in pair_options.items():
            if triple_rank == pair_rank:
                continue

            for triple, pair in product(triples, pairs):
                hands.append(triple + pair)

    return hands


def generate_straights(cards: list[Card]) -> list[list[Card]]:
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

    straights: list[list[Card]] = []

    for i in range(len(rank_order) - 4):
        straight_ranks = rank_order[i : i + 5]

        buckets: list[list[Card]] = []

        for rank in straight_ranks:
            matching_cards = [
                card
                for card in cards
                if card.rank == rank and card.enhancement != Enhancement.STONE
            ]

            if not matching_cards:
                break

            buckets.append(matching_cards)
        else:
            for straight in product(*buckets):
                order = calculate_order(list(straight))
                straights.extend(order)

    return straights


def calculate_order(cards: list[Card]) -> list[list[Card]]:
    double_cards = []
    add_cards = []
    mul_cards = []
    none_cards = []

    for card in cards:
        if card.play_x_mult > 1 and card.add_mult > 0:
            double_cards.append(card)
        # x_mult has higher priority
        elif card.play_x_mult > 1:
            mul_cards.append(card)

        elif card.add_mult > 0:
            add_cards.append(card)

        else:
            none_cards.append(card)

    # The doulbes need sorting since a luck polychrome (x + 20) * 1.5 is greater
    # then a hologram glass (x + 10) * 2
    double_cards.sort(
        key=lambda c: c.add_mult / (c.play_x_mult - 1),
        reverse=True
    )

    return [none_cards + add_cards + double_cards + mul_cards]


def generate_playable_hands(cards: list[Card]) -> list[list[Card]]:
    hands: list[list[Card]] = []
    hands.extend([[card] for card in cards])

    hands.extend(generate_straights(cards))

    rank_bucket = bucket_rank(cards)
    suit_bucket = bucket_suit(cards)

    hands.extend(generate_n_of_a_kind(rank_bucket))
    hands.extend(generate_flushes(suit_bucket))
    hands.extend(generate_2_pair(rank_bucket))
    hands.extend(generate_full_house(rank_bucket))

    return hands
