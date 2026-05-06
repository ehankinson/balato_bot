import time
from itertools import combinations, permutations, product

from core.enums import Edition, Enhancement, PokerHand, Rank, Seal, Suit, JokerNames
from core.hand_stats import HandStats
from core.models import Card, Joker
from config.poker_hands import HAND_STATS


def get_stone_cards(cards: list[Card]) -> list[Card]:
    return [card for card in cards if card.enhancement == Enhancement.STONE]


def get_steel_cards(cards: list[Card]) -> list[Card]:
    return [card for card in cards if card.enhancement == Enhancement.STEEL]


def unique_cards(cards: list[Card]) -> int:
    return len(set(card.rank for card in cards))


def is_same_rank(cards: list[Card]) -> bool:
    initial_rank = cards[0].rank
    return all(card.rank == initial_rank for card in cards)


def is_flush(cards: list[Card]) -> bool:
    initial_suit = cards[0].suit
    return all(card.suit == initial_suit for card in cards)


def is_straight(cards: list[Card]) -> bool:
    cards = sorted(cards, key=lambda x: x.rank, reverse=True)
    for i in range(1, len(cards)):
        if cards[i - 1].rank - cards[i].rank != 1:
            return False

    return True


def generate_individual_x_of_a_kind(hand_size: int, cards: list[Card]) -> list[list[Card]]:
    return [list(val) for val in permutations(cards, hand_size)]


def filter_steel(hand_steel_cards: list[Card], steel_cards: list[Card]) -> list[int]:
    min_length = min(len(hand_steel_cards), len(steel_cards))
    skip_index = []
    for i in range(min_length):
        if hand_steel_cards[i] == steel_cards[i]:
            skip_index.append(i)

    return skip_index


def filter_cards(played_cards: list[Card], cards_in_hand: list[Card]) -> list[Card]:
    hand_card_count = bucket_id(cards_in_hand)
    played_card_count = bucket_id(played_cards)

    cards_held_in_hand: list[Card] = []
    for key, curr_cards in hand_card_count.items():
        hand_cards = played_card_count.get(key, [])

        # if we played this card
        if len(curr_cards) == len(hand_cards):
            continue

        diff = len(curr_cards) - len(hand_cards)
        cards_held_in_hand.extend(curr_cards[:diff])

    return cards_held_in_hand


def bucket_id(cards: list[Card] | tuple[Card, ...]) -> dict[int, list[Card]]:
    bucket: dict[int, list[Card]] = {}

    for card in cards:
        if card.card_id not in bucket:
            bucket[card.card_id] = []

        bucket[card.card_id].append(card)

    return bucket


def bucket_rank(cards: list[Card]) -> dict[Rank, list[Card]]:
    bucket: dict[Rank, list[Card]] = {}
    for card in cards:
        if card.enhancement == Enhancement.STONE:
            continue

        if card.rank not in bucket:
            bucket[card.rank] = []

        bucket[card.rank].append(card)

    return bucket


def bucket_suit(cards: list[Card]) -> dict[Suit, list[Card]]:
    bucket: dict[Suit, list[Card]] = {}
    for card in cards:
        if card.enhancement == Enhancement.STONE:
            continue

        if card.suit not in bucket:
            bucket[card.suit] = []

        bucket[card.suit].append(card)

    return bucket


def get_x_of_a_kind(bucket: dict[Rank, list[Card]]) -> list[list[Card]]:
    x_of_a_kind: list[list[Card]] = []
    for card_values in bucket.values():
        for size in range(2, len(card_values) + 1):
            x_of_a_kind.extend(generate_individual_x_of_a_kind(size, card_values))

    return x_of_a_kind


def get_flushes(bucket: dict[Suit, list[Card]]) -> list[list[Card]]:
    flushes: list[list[Card]] = []
    for card_values in bucket.values():
        if len(card_values) > 4:
            flushes.extend([list(val) for val in permutations(card_values, 5)])

    return flushes


def get_straights(cards: list[Card]) -> list[list[Card]]:
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
                straights.extend([list(val) for val in permutations(straight, 5)])

    return straights


def get_2_pair(bucket: dict[Rank, list[Card]]) -> list[list[Card]]:
    pair_options: dict[Rank, list[list[Card]]] = {
        rank: generate_individual_x_of_a_kind(2, cards)
        for rank, cards in bucket.items()
        if len(cards) >= 2
    }

    return [
        pair1 + pair2
        for rank1, rank2 in combinations(pair_options.keys(), 2)
        for pair1, pair2 in product(pair_options[rank1], pair_options[rank2])
    ]


def get_full_house(bucket: dict[Rank, list[Card]]) -> list[list[Card]]:
    pair_options: dict[Rank, list[list[Card]]] = {}
    triple_options: dict[Rank, list[list[Card]]] = {}

    for rank, cards in bucket.items():
        if len(cards) >= 2:
            pair_options[rank] = generate_individual_x_of_a_kind(2, cards)

        if len(cards) >= 3:
            triple_options[rank] = generate_individual_x_of_a_kind(3, cards)

    hands: list[list[Card]] = []

    for triple_rank, triples in triple_options.items():
        for pair_rank, pairs in pair_options.items():
            if triple_rank == pair_rank:
                continue

            for triple, pair in product(triples, pairs):
                hands.append(triple + pair)

    return hands
    

def generate_playable_hands(cards: list[Card]) -> list[list[Card]]:
    hands: list[list[Card]] = []
    hands.extend([[card] for card in cards])

    hands.extend(get_straights(cards))

    rank_bucket = bucket_rank(cards)
    suit_bucket = bucket_suit(cards)

    hands.extend(get_x_of_a_kind(rank_bucket))
    hands.extend(get_flushes(suit_bucket))
    hands.extend(get_2_pair(rank_bucket))
    hands.extend(get_full_house(rank_bucket))

    return hands


def get_hand_type(hand: list[Card]) -> HandStats:
    hand_len = len(hand)
    unique_rank_count = unique_cards(hand)

    all_same_rank = unique_rank_count == 1
    has_two_ranks = unique_rank_count == 2

    match hand_len:
        case 5:
            flush = is_flush(hand)
            straight = is_straight(list(hand))
            
            checks = [
                (flush and all_same_rank, PokerHand.FLUSH_FIVE),
                (flush and has_two_ranks and not straight, PokerHand.FLUSH_HOUSE),
                (all_same_rank, PokerHand.FIVE_OF_A_KIND),
                (flush and straight, PokerHand.STRAIGHT_FLUSH),
                (has_two_ranks and not flush and not straight, PokerHand.FULL_HOUSE),
                (flush and not straight, PokerHand.FLUSH),
                (straight, PokerHand.STRAIGHT),
            ]
        
            for condition, hand_type in checks:
                if condition:
                    return HAND_STATS[hand_type]

        case 4:
            return HAND_STATS[
                PokerHand.FOUR_OF_A_KIND if all_same_rank else PokerHand.TWO_PAIR
            ]

        case _:
            return HAND_STATS[PokerHand(hand_len)]

    raise ValueError(f"The current hand: {hand} does not have a possible hand, This hand is impossible")



def calculate_score(
    hands: list[list[Card]],
    cards: list[Card],
    stone_cards: list[Card],
    steel_cards: list[Card],
) -> tuple[float, list[list[Card]]]:
    best_score = 0.0
    best_hand: list[list[Card]] = []

    for hand in hands:
        hand_stats = get_hand_type(hand)
        chips, mult = hand_stats.chips, hand_stats.mult

        for card in hand:
            chips += card.chips
            mult += card.add_mult
            mult *= card.play_times_mult

        # check for stone cards since they are free to add
        for stone_card in stone_cards:
            # Can't have more then 5 playing cards
            if len(hand) == 5:
                break

            chips += stone_card.chips
            hand.append(stone_card)

        # check for steal cards to apply mult
        hand_steel_cards = get_steel_cards(hand)
        skip_index = filter_steel(hand_steel_cards, steel_cards)

        for i, steel_card in enumerate(steel_cards):
            if i in skip_index:
                continue

            mult *= steel_card.hand_times_mult
        score = chips * mult

        if best_score < score:
            best_score = score
            best_hand = [hand, filter_cards(hand, cards)]

    return best_score, best_hand



def get_best_scoring_hand(cards: list[Card]) -> tuple[float, list[list[Card]]]:
    hands = generate_playable_hands(cards)
    stone_cards = get_stone_cards(cards)
    steel_cards = get_steel_cards(cards)
    return calculate_score(hands, cards, stone_cards, steel_cards)


if __name__ == "__main__":
    cards = [
        Card(Rank.KING, Suit.CLUBS, Enhancement.GLASS, Seal.RED, Edition.NONE),
        Card(Rank.KING, Suit.CLUBS, Enhancement.NONE, Seal.NONE, Edition.NONE),
        Card(Rank.QUEEN, Suit.CLUBS, Enhancement.STEEL, Seal.RED, Edition.NONE),
        Card(Rank.QUEEN, Suit.CLUBS, Enhancement.STEEL, Seal.RED, Edition.NONE),
        Card(Rank.QUEEN, Suit.CLUBS, Enhancement.STEEL, Seal.RED, Edition.NONE),
        Card(Rank.JACK, Suit.CLUBS, Enhancement.NONE, Seal.NONE, Edition.NONE),
        Card(Rank.TEN, Suit.CLUBS, Enhancement.NONE, Seal.NONE, Edition.NONE),
        Card(Rank.ACE, Suit.CLUBS, Enhancement.GLASS, Seal.GOLD, Edition.NONE),
    ]

    # hand = Hand.random_hand(8)
    # cards = hand.cards
    hands = generate_playable_hands(cards)
    stone_cards = get_stone_cards(cards)
    steel_cards = get_steel_cards(cards)

    start_time = time.perf_counter()
    best_score, played_hand = calculate_score(hands, cards, stone_cards, steel_cards)
    end_time = time.perf_counter()

    print(
        f"The time taken to calcualte the best was {end_time - start_time}, going through {len(hands)} hands"
    )
    print(best_score)
    print(f"Playing Hand was {played_hand[0]}")
    print(f"Cards Held in Hand were {played_hand[1]}")
