import pytest

from core.enums import Enhancement, JokersName, Rank, Suit
from core.models import Card, JokerScoring
from _test_util import buildable_jokers_of_type, build_card, score_joker


def test_0001_every_buildable_scoring_joker_has_direct_tests():
    tested_jokers = {
        JokersName.JOKER,
        JokersName.JOLLY_JOKER,
        JokersName.ZANY_JOKER,
        JokersName.MAD_JOKER,
        JokersName.CRAZY_JOKER,
        JokersName.DROLL_JOKER,
        JokersName.HALF_JOKER,
        JokersName.STONE_JOKER,
        JokersName.ACROBAT,
        JokersName.GREEDY_JOKER,
        JokersName.LUSTY_JOKER,
        JokersName.WRATHFUL_JOKER,
        JokersName.GLUTTONOUS_JOKER,
        JokersName.BANNER,
        JokersName.MYSTIC_SUMMIT,
        JokersName.LOYALTY_CARD,
        JokersName.MISPRINT,
        JokersName.STEEL_JOKER,
        JokersName.RAISED_FIST,
        JokersName.SCARY_FACE,
        JokersName.ABSTRACT_JOKER,
        JokersName.EVEN_STEVEN,
        JokersName.ODD_TODD,
        JokersName.SCHOLAR,
        JokersName.SUPERNOVA,
        JokersName.SEEING_DOUBLE,
        JokersName.THE_DUO,
        JokersName.THE_TRIO,
        JokersName.THE_FAMILY,
        JokersName.THE_ORDER,
        JokersName.THE_TRIBE,
        JokersName.FIBONACCI,
        JokersName.JOKER_STENCIL,
        JokersName.CEREMONIAL_DAGGER,
        JokersName.SWASHBUCKLER,
        JokersName.FLOWER_POT,
        JokersName.RID_THE_BUS,
        JokersName.SHOOT_THE_MOON,
        JokersName.GROS_MICHEL,
        JokersName.STUNTMAN,
        JokersName.DRIVERS_LICENSE,
        JokersName.THROWBACK,
        JokersName.THE_IDOL,
        JokersName.BLOODSTONE,
        JokersName.ARROWHEAD,
        JokersName.ONYX_AGATE,
        JokersName.CANIO_BACKGROUND,
        JokersName.TRIBOULET_BACKGROUND,
        JokersName.YORICK_BACKGROUND,
        JokersName.BOOTSTRAPS,
        JokersName.BLACKBOARD,
        JokersName.RUNNER,
        JokersName.ICE_CREAM,
        JokersName.CONSTELLATION,
        JokersName.HIKER,
        JokersName.GREEN_JOKER,
        JokersName.CAVENDISH,
        JokersName.CARD_SHARP,
        JokersName.RED_CARD,
        JokersName.MADNESS,
        JokersName.SQUARE_JOKER,
        JokersName.VAMPIRE,
        JokersName.HOLOGRAM_BACKGROUND,
        JokersName.BARON,
        JokersName.OBELISK,
        JokersName.PHOTOGRAPH,
        JokersName.EROSION,
        JokersName.SLY_JOKER,
        JokersName.WILY_JOKER,
        JokersName.CLEVER_JOKER,
        JokersName.DEVIOUS_JOKER,
        JokersName.CRAFTY_JOKER,
        JokersName.LUCKY_CAT,
        JokersName.BASEBALL_CARD,
        JokersName.BULL,
        JokersName.FLASH_CARD,
        JokersName.POPCORN,
        JokersName.RAMEN,
        JokersName.SPARE_TROUSERS,
        JokersName.CAMPFIRE,
        JokersName.SMILEY_FACE,
        JokersName.ANCIENT_JOKER,
        JokersName.WALKIE_TALKIE,
        JokersName.CASTLE,
    }
    assert buildable_jokers_of_type(JokerScoring) == tested_jokers


@pytest.mark.parametrize(
    ("joker_name", "expected"),
    [
        (JokersName.JOKER, (0, 4, 1.0)),
        (JokersName.STONE_JOKER, (0, 0, 1.0)),
        (JokersName.ACROBAT, (0, 0, 1)),
        (JokersName.BANNER, (0, 0, 1.0)),
        (JokersName.MYSTIC_SUMMIT, (0, 0, 1.0)),
        (JokersName.LOYALTY_CARD, (0, 0, 1)),
        (JokersName.MISPRINT, (0, 0, 1.0)),
        (JokersName.STEEL_JOKER, (0, 0, 1)),
        (JokersName.ABSTRACT_JOKER, (0, 0, 1.0)),
        (JokersName.SUPERNOVA, (0, 0, 1.0)),
        (JokersName.JOKER_STENCIL, (0, 0, 1)),
        (JokersName.CEREMONIAL_DAGGER, (0, 0, 1.0)),
        (JokersName.SWASHBUCKLER, (0, 0, 1.0)),
        (JokersName.RID_THE_BUS, (0, 0, 1.0)),
        (JokersName.STUNTMAN, (250, 0, 1.0)),
        (JokersName.THROWBACK, (0, 0, 1)),
        (JokersName.CANIO_BACKGROUND, (0, 0, 1)),
        (JokersName.YORICK_BACKGROUND, (0, 0, 1)),
        (JokersName.BOOTSTRAPS, (0, 0, 1.0)),
        (JokersName.RUNNER, (0, 0, 1.0)),
        (JokersName.ICE_CREAM, (100, 0, 1.0)),
        (JokersName.CONSTELLATION, (0, 0, 1)),
        (JokersName.HIKER, (0, 0, 1.0)),
        (JokersName.GREEN_JOKER, (0, 0, 1.0)),
        (JokersName.RED_CARD, (0, 0, 1.0)),
        (JokersName.MADNESS, (0, 0, 1)),
        (JokersName.SQUARE_JOKER, (0, 0, 1.0)),
        (JokersName.VAMPIRE, (0, 0, 1)),
        (JokersName.HOLOGRAM_BACKGROUND, (0, 0, 1)),
        (JokersName.OBELISK, (0, 0, 1)),
        (JokersName.EROSION, (0, 0, 1.0)),
        (JokersName.LUCKY_CAT, (0, 0, 1)),
        (JokersName.BULL, (0, 0, 1.0)),
        (JokersName.FLASH_CARD, (0, 0, 1.0)),
        (JokersName.POPCORN, (0, 20, 1.0)),
        (JokersName.RAMEN, (0, 0, 2)),
        (JokersName.SPARE_TROUSERS, (0, 0, 1.0)),
        (JokersName.CAMPFIRE, (0, 0, 1)),
        (JokersName.CASTLE, (0, 0, 1.0)),
    ],
)
def test_0002_static_scoring_jokers(joker_name: JokersName, expected: tuple[int, int, float]):
    assert score_joker(joker_name) == expected


@pytest.mark.parametrize(
    ("joker_name", "cards", "expected"),
    [
        (JokersName.JOLLY_JOKER, [build_card(Rank.ACE), build_card(Rank.ACE)], (0, 8, 1.0)),
        (JokersName.ZANY_JOKER, [build_card(Rank.ACE), build_card(Rank.ACE), build_card(Rank.ACE)], (0, 12, 1.0)),
        (JokersName.MAD_JOKER, [build_card(Rank.ACE), build_card(Rank.ACE), build_card(Rank.KING), build_card(Rank.KING)], (0, 10, 1.0)),
        (JokersName.CRAZY_JOKER, [build_card(Rank.ACE), build_card(Rank.KING), build_card(Rank.QUEEN), build_card(Rank.JACK), build_card(Rank.TEN)], (0, 12, 1.0)),
        (JokersName.DROLL_JOKER, [build_card(Rank.ACE), build_card(Rank.KING), build_card(Rank.NINE), build_card(Rank.SEVEN), build_card(Rank.THREE)], (0, 10, 1.0)),
        (JokersName.THE_DUO, [build_card(Rank.ACE), build_card(Rank.ACE)], (0, 0, 2)),
        (JokersName.THE_TRIO, [build_card(Rank.ACE), build_card(Rank.ACE), build_card(Rank.ACE)], (0, 0, 3)),
        (JokersName.THE_FAMILY, [build_card(Rank.ACE), build_card(Rank.ACE), build_card(Rank.ACE), build_card(Rank.ACE)], (0, 0, 4)),
        (JokersName.THE_ORDER, [build_card(Rank.ACE), build_card(Rank.KING), build_card(Rank.QUEEN), build_card(Rank.JACK), build_card(Rank.TEN)], (0, 0, 3)),
        (JokersName.THE_TRIBE, [build_card(Rank.ACE), build_card(Rank.KING), build_card(Rank.NINE), build_card(Rank.SEVEN), build_card(Rank.THREE)], (0, 0, 2)),
        (JokersName.SLY_JOKER, [build_card(Rank.ACE), build_card(Rank.ACE)], (50, 0, 1.0)),
        (JokersName.WILY_JOKER, [build_card(Rank.ACE), build_card(Rank.ACE), build_card(Rank.ACE)], (100, 0, 1.0)),
        (JokersName.CLEVER_JOKER, [build_card(Rank.ACE), build_card(Rank.ACE), build_card(Rank.KING), build_card(Rank.KING)], (80, 0, 1.0)),
        (JokersName.DEVIOUS_JOKER, [build_card(Rank.ACE), build_card(Rank.KING), build_card(Rank.QUEEN), build_card(Rank.JACK), build_card(Rank.TEN)], (100, 0, 1.0)),
        (JokersName.CRAFTY_JOKER, [build_card(Rank.ACE), build_card(Rank.KING), build_card(Rank.NINE), build_card(Rank.SEVEN), build_card(Rank.THREE)], (80, 0, 1.0)),
    ],
)
def test_0003_hand_type_condition_hits(
    joker_name: JokersName,
    cards: list[Card],
    expected: tuple[int, int, float],
):
    assert score_joker(joker_name, scoring_played=cards) == expected


def test_0004_hand_type_condition_misses():
    high_card = [build_card(Rank.ACE), build_card(Rank.KING)]

    assert score_joker(JokersName.JOLLY_JOKER, scoring_played=high_card) == (0, 0, 1.0)
    assert score_joker(JokersName.THE_DUO, scoring_played=high_card) == (0, 0, 1.0)
    assert score_joker(JokersName.SLY_JOKER, scoring_played=high_card) == (0, 0, 1.0)


def test_0005_half_joker_hits_only_when_playing_three_or_fewer_cards():
    assert score_joker(JokersName.HALF_JOKER, scoring_played=[build_card(Rank.ACE), build_card(Rank.KING), build_card(Rank.QUEEN)]) == (0, 20, 1.0)
    assert score_joker(JokersName.HALF_JOKER, scoring_played=[build_card(Rank.ACE), build_card(Rank.KING), build_card(Rank.QUEEN), build_card(Rank.JACK)]) == (0, 0, 1.0)


@pytest.mark.parametrize(
    ("joker_name", "card", "expected"),
    [
        (JokersName.SCARY_FACE, build_card(Rank.KING), (30, 0, 1.0)),
        (JokersName.SCARY_FACE, build_card(Rank.TEN), (0, 0, 1.0)),
        (JokersName.SMILEY_FACE, build_card(Rank.QUEEN), (0, 5, 1.0)),
        (JokersName.SMILEY_FACE, build_card(Rank.ACE), (0, 0, 1.0)),
        (JokersName.SHOOT_THE_MOON, build_card(Rank.QUEEN), (0, 13, 1.0)),
        (JokersName.BARON, build_card(Rank.KING), (0, 0, 1.5)),
        (JokersName.TRIBOULET_BACKGROUND, build_card(Rank.KING), (0, 0, 2)),
        (JokersName.TRIBOULET_BACKGROUND, build_card(Rank.QUEEN), (0, 0, 2)),
        (JokersName.TRIBOULET_BACKGROUND, build_card(Rank.JACK), (0, 0, 1.0)),
    ],
)
def test_0006_rank_condition_jokers(
    joker_name: JokersName,
    card: Card,
    expected: tuple[int, int, float],
):
    assert score_joker(joker_name, card=card) == expected


@pytest.mark.parametrize(
    ("joker_name", "card", "expected"),
    [
        (JokersName.EVEN_STEVEN, build_card(Rank.TEN), (0, 0, 1.0)),
        (JokersName.ODD_TODD, build_card(Rank.ACE), (0, 0, 1.0)),
        (JokersName.SCHOLAR, build_card(Rank.ACE), (0, 0, 1.0)),
        (JokersName.FIBONACCI, build_card(Rank.FIVE), (0, 0, 1.0)),
        (JokersName.WALKIE_TALKIE, build_card(Rank.TEN), (0, 0, 1.0)),
    ],
)
def test_0007_rank_alias_condition_jokers(
    joker_name: JokersName,
    card: Card,
    expected: tuple[int, int, float],
):
    assert score_joker(joker_name, card=card) == expected


def test_0008_photograph_only_hits_first_played_face_card():
    assert score_joker(JokersName.PHOTOGRAPH, card=build_card(Rank.KING), face_card_count=0) == (0, 0, 2)
    assert score_joker(JokersName.PHOTOGRAPH, card=build_card(Rank.KING), face_card_count=1) == (0, 0, 1.0)
    assert score_joker(JokersName.PHOTOGRAPH, card=build_card(Rank.TEN), face_card_count=0) == (0, 0, 1.0)


@pytest.mark.parametrize(
    ("joker_name", "card", "expected"),
    [
        (JokersName.GREEDY_JOKER, build_card(Rank.ACE, Suit.DIAMONDS), (0, 3, 1.0)),
        (JokersName.LUSTY_JOKER, build_card(Rank.ACE, Suit.HEARTS), (0, 3, 1.0)),
        (JokersName.WRATHFUL_JOKER, build_card(Rank.ACE, Suit.SPADES), (0, 3, 1.0)),
        (JokersName.GLUTTONOUS_JOKER, build_card(Rank.ACE, Suit.CLUBS), (0, 3, 1.0)),
        (JokersName.SEEING_DOUBLE, build_card(Rank.ACE, Suit.CLUBS), (0, 0, 2)),
        (JokersName.BLOODSTONE, build_card(Rank.ACE, Suit.HEARTS), (0, 0, 1.5)),
        (JokersName.ARROWHEAD, build_card(Rank.ACE, Suit.SPADES), (50, 0, 1.0)),
        (JokersName.ONYX_AGATE, build_card(Rank.ACE, Suit.CLUBS), (0, 7, 1.0)),
    ],
)
def test_0009_suit_condition_jokers(
    joker_name: JokersName,
    card: Card,
    expected: tuple[int, int, float],
):
    assert score_joker(joker_name, card=card) == expected


def test_0010_suit_condition_misses_and_wild_cards_hit():
    assert score_joker(JokersName.GREEDY_JOKER, card=build_card(Rank.ACE, Suit.HEARTS)) == (0, 0, 1.0)
    assert score_joker(JokersName.GREEDY_JOKER, card=build_card(Rank.ACE, Suit.HEARTS, Enhancement.WILD)) == (0, 3, 1.0)
    assert score_joker(JokersName.GREEDY_JOKER, card=build_card(Rank.ACE, Suit.DIAMONDS, Enhancement.STONE)) == (0, 0, 1.0)


def test_0011_flower_pot_requires_four_played_suits():
    four_suits = [
        build_card(Rank.ACE, Suit.HEARTS),
        build_card(Rank.KING, Suit.DIAMONDS),
        build_card(Rank.QUEEN, Suit.CLUBS),
        build_card(Rank.JACK, Suit.SPADES),
    ]
    three_suits = four_suits[:3]

    assert score_joker(JokersName.FLOWER_POT, scoring_played=four_suits) == (0, 0, 3)
    assert score_joker(JokersName.FLOWER_POT, scoring_played=three_suits) == (0, 0, 1.0)


def test_0012_blackboard_requires_only_clubs_and_spades_held():
    assert score_joker(
        JokersName.BLACKBOARD,
        unscoring_held=[build_card(Rank.ACE, Suit.CLUBS), build_card(Rank.KING, Suit.SPADES)],
    ) == (0, 0, 3)
    assert score_joker(
        JokersName.BLACKBOARD,
        unscoring_held=[build_card(Rank.ACE, Suit.CLUBS), build_card(Rank.KING, Suit.HEARTS)],
    ) == (0, 0, 1.0)


def test_0013_the_idol_currently_does_not_hit_when_rank_and_suit_req_are_set():
    idol_card = build_card(Rank.ACE, Suit.HEARTS)

    assert score_joker(
        JokersName.THE_IDOL,
        card=idol_card,
        req_rank=Rank.ACE,
        req_suit=Suit.HEARTS,
    ) == (0, 0, 1.0)


def test_0014_ancient_joker_can_hit_when_suit_req_is_set():
    assert score_joker(
        JokersName.ANCIENT_JOKER,
        card=build_card(Rank.ACE, Suit.SPADES),
        req_suit=Suit.SPADES,
    ) == (0, 0, 1.5)


def test_0015_raised_fist_scores_lowest_held_card():
    lowest_held = build_card(Rank.FIVE)
    higher_held = build_card(Rank.KING)

    assert score_joker(
        JokersName.RAISED_FIST,
        card=lowest_held,
        scoring_held=[lowest_held, higher_held],
    ) == (0, 10, 1.0)
    assert score_joker(
        JokersName.RAISED_FIST,
        card=higher_held,
        scoring_held=[lowest_held, higher_held],
    ) == (0, 0, 1.0)


@pytest.mark.parametrize(
    "joker_name",
    [
        JokersName.GROS_MICHEL,
        JokersName.CAVENDISH,
        JokersName.DRIVERS_LICENSE,
        JokersName.CARD_SHARP,
        JokersName.BASEBALL_CARD,
    ],
)
def test_0016_runtime_state_condition_jokers_currently_raise_value_error(
    joker_name: JokersName,
):
    with pytest.raises(ValueError):
        score_joker(joker_name)
