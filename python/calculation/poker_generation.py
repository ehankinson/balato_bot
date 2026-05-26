from itertools import combinations, permutations, product

from calculation.poker_eval import get_hand_type
from calculation.util import bucket_rank, bucket_suit
from core.enums import Enhancement, JokersName, PokerHand, Rank, Suit
from core.hand_stats import HandStats
from core.models import Card, CardBucket, HandScoring, Joker


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
    double_cards.sort(key=lambda c: c.add_mult / (c.play_x_mult - 1), reverse=True)

    return [none_cards + add_cards + double_cards + mul_cards]


def build_cards_not_played(
    main_bucket: dict[int, CardBucket], filter_cards: list[Card]
) -> list[Card]:
    for card in filter_cards:
        if card.enhancement == Enhancement.STONE:
            continue

        main_bucket[card.card_id].count -= 1

    cards_not_played = []
    for values in main_bucket.values():
        cards_not_played.extend(values.cards[: values.count])
        values.count = len(values.cards)

    return cards_not_played


def can_add_to_hand(card: Card, hand_stats: HandStats, scoring_cards: list[Card]) -> bool:
    can_add = False
    played_hand = hand_stats.name

    if played_hand == PokerHand.HIGH_CARD:
        can_add = scoring_cards[0].rank > card.rank
    elif played_hand in [PokerHand.PAIR, PokerHand.THREE_OF_A_KIND, PokerHand.FOUR_OF_A_KIND, PokerHand.TWO_PAIR]:
        can_add = all(card.rank != iter_card.rank for iter_card in scoring_cards)

    # otherwise the hand we played is 5 cards which is the max amount
    return can_add


def filter_cards(
    main_bucket: dict[int, CardBucket], filter_cards: list[Card], jokers: list[Joker]
) -> tuple[list[Card], list[Card]]:
    cards_not_played = build_cards_not_played(main_bucket, filter_cards)

    steel_cards = [
        card for card in cards_not_played if card.enhancement == Enhancement.STEEL
    ]

    important_cards = []
    if any(joker.background_image == JokersName.SHOOT_THE_MOON for joker in jokers):
        queen_cards = [
            card
            for card in cards_not_played
            if card.rank == Rank.QUEEN and card not in steel_cards
        ]
        important_cards.extend(queen_cards)

    if any(joker.background_image == JokersName.BARON for joker in jokers):
        king_cards = [
            card
            for card in cards_not_played
            if card.rank == Rank.KING and card not in steel_cards
        ]
        important_cards.extend(king_cards)

    # this is ordered from important plain cards to steel cards
    important_cards.extend(steel_cards)

    none_important_cards = [
        card for card in cards_not_played if card not in important_cards
    ]
    return important_cards, none_important_cards


def add_stone_cards(stone_cards: list[Card], hand_cache: list[HandScoring]) -> None:
    for hand_scoring in hand_cache:
        scoring_cards = hand_scoring.scored_played
        none_scoring_cards = hand_scoring.unscored_played

        max_add_cards = 5 - (len(scoring_cards) + len(none_scoring_cards))
        scoring_cards.extend(stone_cards[:max_add_cards])


def help_blackboard(hand_cache: list[HandScoring], jokers: list[Joker]) -> None:

    def update_played_cards(
        cards_not_played: list[Card],
        scoring_cards: list[Card],
        none_scoring_cards: list[Card],
        hand_stats: HandStats
    ) -> None:
        heart_diamond_cards: list[Card] = [
            card
            for card in cards_not_played
            if card.suit in [Suit.HEARTS, Suit.DIAMONDS]
        ]

        if len(heart_diamond_cards) == 0:
            return

        # We need this since if we play a queen highcard and want to throw out a king
        # that queen highcard then becomes a king, so its different
        none_altering_hd_cards = [card for card in heart_diamond_cards if can_add_to_hand(card, hand_stats, scoring_cards)]

        # we don't need to check the length of unscored_played since this function
        # is the first time we touch it
        max_add_cards = 5 - len(scoring_cards)
        none_scoring_cards.extend(none_altering_hd_cards[:max_add_cards])

        # Now we will remove all the cards that we've added to the hand
        for card in none_altering_hd_cards[:max_add_cards]:
            cards_not_played.remove(card)

    for hand_scoring in hand_cache:
        # checking if there is any unscored held cards that we can add
        update_played_cards(
            hand_scoring.unscored_held,
            hand_scoring.scored_played,
            hand_scoring.unscored_played,
            hand_scoring.hand_stats
        )

        # checking if there are scored held cards that we can add
        update_played_cards(
            hand_scoring.scored_held,
            hand_scoring.scored_played,
            hand_scoring.unscored_played,
            hand_scoring.hand_stats
        )


def help_raised_fist(hand_cache: list[HandScoring]) -> None:
    # With filter_cards, we know that scored_held can only have queens and kings and steel cards
    # so we should be checking the unscored_held first and remove as many low cards as we cards
    # We should also not remove cards from scored_held since it is rare to improve score

    def update_lowest_card(
        max_add_cards: int,
        scored_held: list[Card],
        unscored_held: list[Card],
        unscored_played: list[Card],
    ) -> None:
        if max_add_cards > len(unscored_held):
            max_add_cards = (
                len(unscored_held) - 1
            )  # since we raised fist needs to have something

        unscored_played.extend(unscored_held[:max_add_cards])

        for i in range(max_add_cards - 1):
            unscored_held.pop(i)

        # adding the lowest rank card to the scored_held_cards
        scored_held.append(unscored_held.pop())

    for hand_scoring in hand_cache:
        if len(hand_scoring.scored_held) + len(hand_scoring.unscored_held) == 0:
            return  # there is nothing to do

        # The lowest card is already in the scored_held so just sort them accending
        if len(hand_scoring.scored_held) > 0 and len(hand_scoring.unscored_held) == 0:
            hand_scoring.scored_held = sorted(
                hand_scoring.scored_held, key=lambda card: card.rank
            )
            continue

        max_add_cards = 5 - (
            len(hand_scoring.scored_played) + len(hand_scoring.unscored_held)
        )
        hand_scoring.scored_held = sorted(
            hand_scoring.scored_held, key=lambda card: card.rank
        )
        hand_scoring.unscored_held = sorted(
            hand_scoring.unscored_held, key=lambda card: card.rank
        )

        # If there is no scored_held cards, then take the lowest card
        # form the unscored_held cards. (we can do some manipulation to get that number higher)
        if len(hand_scoring.scored_held) == 0 and len(hand_scoring.unscored_held) > 0:
            update_lowest_card(
                max_add_cards,
                hand_scoring.scored_held,
                hand_scoring.unscored_held,
                hand_scoring.unscored_played,
            )
        else:
            min_scored_held = hand_scoring.scored_held[0].rank
            min_unscored_held = hand_scoring.unscored_held[0].rank

            if min_scored_held < min_unscored_held:
                continue  # adding throw away cards will do nothing to the final score

            update_lowest_card(
                max_add_cards,
                hand_scoring.scored_held,
                hand_scoring.unscored_held,
                hand_scoring.unscored_played,
            )


def build_playable_hands(cards: list[Card]) -> list[list[Card]]:
    hands: list[list[Card]] = []

    # Makes highcard
    # And we are skipping highcard to help calculations later down the line
    hands.extend([[card] for card in cards if card.enhancement != Enhancement.STONE])

    hands.extend(generate_straights(cards))

    rank_bucket = bucket_rank(cards)
    suit_bucket = bucket_suit(cards)

    hands.extend(generate_n_of_a_kind(rank_bucket))
    hands.extend(generate_flushes(suit_bucket))
    hands.extend(generate_2_pair(rank_bucket))
    hands.extend(generate_full_house(rank_bucket))

    return hands


def generate_playable_hands(
    cards: list[Card], jokers: list[Joker]
) -> list[HandScoring]:
    hands = build_playable_hands(cards)

    main_bucket: dict[int, CardBucket] = {}
    for card in cards:
        # We don't want stone cards, since none of the
        # hand generation counts them
        if card.enhancement == Enhancement.STONE:
            continue

        if card.card_id not in main_bucket:
            main_bucket[card.card_id] = CardBucket(count=0, cards=[])

        main_bucket[card.card_id].count += 1
        main_bucket[card.card_id].cards.append(card)

    hand_cache: list[HandScoring] = []
    for hand in hands:
        scored_held, unscored_held = filter_cards(main_bucket, hand, jokers)
        hand_cache.append(
            HandScoring(
                hand_stats=get_hand_type(hand),
                scored_played=hand,
                unscored_played=[],
                scored_held=scored_held,
                unscored_held=unscored_held,
            )
        )

    # Since blackboard only activates when Spades and Clubs are held in hand,
    # its benifial to add dead cards when playing (i.e. playing extra hearts and diamonds even if they don't increase score)
    if any(joker.background_image == JokersName.BLACKBOARD for joker in jokers):
        help_blackboard(hand_cache, jokers)

    # For raised fist, we will do this after blackboard since 3x mult is better 99% of the time
    # then a max of +22
    if any(joker.background_image == JokersName.RAISED_FIST for joker in jokers):
        help_raised_fist(hand_cache)

    # checking if we have stone cards, since that is always a plus
    # when optimizing for score
    stone_cards = [card for card in cards if card.enhancement == Enhancement.STONE]
    if len(stone_cards) > 0:
        add_stone_cards(stone_cards, hand_cache)

    return hand_cache
