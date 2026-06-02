import pytest
from _test_util import build_card, buildable_jokers_of_type

from calculation.joker_scoring import calculate_joker_scoring
from core.enums import Enhancement, JokersName, Rank, Suit
from core.models import (
    Card,
    Joker,
    JokerReq,
    JokerScoring,
    JokerScoringConditions,
)


def _build_scoring_joker(
    joker_name: JokersName,
    *,
    req_rank: Rank = Rank.NONE,
    req_suit: Suit = Suit.NONE,
) -> JokerScoring:
    joker = Joker.build(joker_name)
    assert isinstance(joker, JokerScoring), (
        f"The joker {joker_name} is not type JokerScoring"
    )
    joker.req = JokerReq(rank=req_rank, suit=req_suit)
    return joker


def _score_joker(
    joker_name: JokersName,
    *,
    card: Card | None = None,
    scoring_played: list[Card] | None = None,
    scoring_held: list[Card] | None = None,
    unscoring_held: list[Card] | None = None,
    face_card_count: int = -1,
    req_rank: Rank = Rank.NONE,
    req_suit: Suit = Suit.NONE,
) -> tuple[int, int, float]:
    scoring_played = scoring_played or [card or build_card(Rank.ACE)]
    card = card or scoring_played[0]
    condition_args = JokerScoringConditions(
        card=card,
        face_card_count=face_card_count,
        scoring_played=scoring_played,
        scoring_held=scoring_held or [],
        unscoring_held=unscoring_held or [],
    )
    joker = _build_scoring_joker(
        joker_name,
        req_rank=req_rank,
        req_suit=req_suit,
    )

    return calculate_joker_scoring(joker, condition_args)


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
        (JokersName.MISPRINT, (0, 23, 1.0)),
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
def test_0002_static_scoring_jokers(
    joker_name: JokersName,
    expected: tuple[int, int, float],
):
    assert _score_joker(joker_name) == expected


@pytest.mark.parametrize(
    ("joker_name", "hit_cards", "miss_cards", "expected"),
    [
        (
            JokersName.JOLLY_JOKER,
            [build_card(Rank.ACE), build_card(Rank.ACE)],
            [build_card(Rank.ACE), build_card(Rank.KING)],
            (0, 8, 1.0),
        ),
        (
            JokersName.ZANY_JOKER,
            [build_card(Rank.ACE), build_card(Rank.ACE), build_card(Rank.ACE)],
            [build_card(Rank.ACE), build_card(Rank.ACE)],
            (0, 12, 1.0),
        ),
        (
            JokersName.MAD_JOKER,
            [
                build_card(Rank.ACE),
                build_card(Rank.ACE),
                build_card(Rank.KING),
                build_card(Rank.KING),
            ],
            [build_card(Rank.ACE), build_card(Rank.ACE), build_card(Rank.KING)],
            (0, 10, 1.0),
        ),
        (
            JokersName.CRAZY_JOKER,
            [
                build_card(Rank.ACE),
                build_card(Rank.KING),
                build_card(Rank.QUEEN),
                build_card(Rank.JACK),
                build_card(Rank.TEN),
            ],
            [
                build_card(Rank.ACE),
                build_card(Rank.KING),
                build_card(Rank.QUEEN),
                build_card(Rank.JACK),
                build_card(Rank.NINE),
            ],
            (0, 12, 1.0),
        ),
        (
            JokersName.DROLL_JOKER,
            [
                build_card(Rank.ACE),
                build_card(Rank.KING),
                build_card(Rank.NINE),
                build_card(Rank.SEVEN),
                build_card(Rank.THREE),
            ],
            [
                build_card(Rank.ACE, Suit.HEARTS),
                build_card(Rank.KING, Suit.CLUBS),
                build_card(Rank.NINE, Suit.SPADES),
                build_card(Rank.SEVEN, Suit.DIAMONDS),
                build_card(Rank.THREE, Suit.HEARTS),
            ],
            (0, 10, 1.0),
        ),
        (
            JokersName.THE_DUO,
            [build_card(Rank.ACE), build_card(Rank.ACE)],
            [build_card(Rank.ACE), build_card(Rank.KING)],
            (0, 0, 2),
        ),
        (
            JokersName.THE_TRIO,
            [build_card(Rank.ACE), build_card(Rank.ACE), build_card(Rank.ACE)],
            [build_card(Rank.ACE), build_card(Rank.ACE)],
            (0, 0, 3),
        ),
        (
            JokersName.THE_FAMILY,
            [
                build_card(Rank.ACE),
                build_card(Rank.ACE),
                build_card(Rank.ACE),
                build_card(Rank.ACE),
            ],
            [build_card(Rank.ACE), build_card(Rank.ACE), build_card(Rank.ACE)],
            (0, 0, 4),
        ),
        (
            JokersName.THE_ORDER,
            [
                build_card(Rank.ACE),
                build_card(Rank.KING),
                build_card(Rank.QUEEN),
                build_card(Rank.JACK),
                build_card(Rank.TEN),
            ],
            [
                build_card(Rank.ACE),
                build_card(Rank.KING),
                build_card(Rank.QUEEN),
                build_card(Rank.JACK),
                build_card(Rank.NINE),
            ],
            (0, 0, 3),
        ),
        (
            JokersName.THE_TRIBE,
            [
                build_card(Rank.ACE),
                build_card(Rank.KING),
                build_card(Rank.NINE),
                build_card(Rank.SEVEN),
                build_card(Rank.THREE),
            ],
            [
                build_card(Rank.ACE, Suit.HEARTS),
                build_card(Rank.KING, Suit.CLUBS),
                build_card(Rank.NINE, Suit.SPADES),
                build_card(Rank.SEVEN, Suit.DIAMONDS),
                build_card(Rank.THREE, Suit.HEARTS),
            ],
            (0, 0, 2),
        ),
        (
            JokersName.SLY_JOKER,
            [build_card(Rank.ACE), build_card(Rank.ACE)],
            [build_card(Rank.ACE), build_card(Rank.KING)],
            (50, 0, 1.0),
        ),
        (
            JokersName.WILY_JOKER,
            [build_card(Rank.ACE), build_card(Rank.ACE), build_card(Rank.ACE)],
            [build_card(Rank.ACE), build_card(Rank.ACE)],
            (100, 0, 1.0),
        ),
        (
            JokersName.CLEVER_JOKER,
            [
                build_card(Rank.ACE),
                build_card(Rank.ACE),
                build_card(Rank.KING),
                build_card(Rank.KING),
            ],
            [build_card(Rank.ACE), build_card(Rank.ACE), build_card(Rank.KING)],
            (80, 0, 1.0),
        ),
        (
            JokersName.DEVIOUS_JOKER,
            [
                build_card(Rank.ACE),
                build_card(Rank.KING),
                build_card(Rank.QUEEN),
                build_card(Rank.JACK),
                build_card(Rank.TEN),
            ],
            [
                build_card(Rank.ACE),
                build_card(Rank.KING),
                build_card(Rank.QUEEN),
                build_card(Rank.JACK),
                build_card(Rank.NINE),
            ],
            (100, 0, 1.0),
        ),
        (
            JokersName.CRAFTY_JOKER,
            [
                build_card(Rank.ACE),
                build_card(Rank.KING),
                build_card(Rank.NINE),
                build_card(Rank.SEVEN),
                build_card(Rank.THREE),
            ],
            [
                build_card(Rank.ACE, Suit.HEARTS),
                build_card(Rank.KING, Suit.CLUBS),
                build_card(Rank.NINE, Suit.SPADES),
                build_card(Rank.SEVEN, Suit.DIAMONDS),
                build_card(Rank.THREE, Suit.HEARTS),
            ],
            (80, 0, 1.0),
        ),
    ],
)
def test_0003_hand_type_condition_jokers(
    joker_name: JokersName,
    hit_cards: list[Card],
    miss_cards: list[Card],
    expected: tuple[int, int, float],
):
    assert _score_joker(joker_name, scoring_played=hit_cards) == expected
    assert _score_joker(joker_name, scoring_played=miss_cards) == (0, 0, 1.0)


def test_0004_cards_played_condition_jokers():
    assert _score_joker(
        JokersName.HALF_JOKER,
        scoring_played=[
            build_card(Rank.ACE),
            build_card(Rank.KING),
            build_card(Rank.QUEEN),
        ],
    ) == (0, 20, 1.0)
    assert _score_joker(
        JokersName.HALF_JOKER,
        scoring_played=[
            build_card(Rank.ACE),
            build_card(Rank.KING),
            build_card(Rank.QUEEN),
            build_card(Rank.JACK),
        ],
    ) == (0, 0, 1.0)


@pytest.mark.parametrize(
    ("joker_name", "hit_card", "miss_card", "expected"),
    [
        (
            JokersName.SCARY_FACE,
            build_card(Rank.KING),
            build_card(Rank.TEN),
            (30, 0, 1.0),
        ),
        (
            JokersName.SMILEY_FACE,
            build_card(Rank.QUEEN),
            build_card(Rank.ACE),
            (0, 5, 1.0),
        ),
        (
            JokersName.SHOOT_THE_MOON,
            build_card(Rank.QUEEN),
            build_card(Rank.KING),
            (0, 13, 1.0),
        ),
        (JokersName.BARON, build_card(Rank.KING), build_card(Rank.QUEEN), (0, 0, 1.5)),
        (
            JokersName.TRIBOULET_BACKGROUND,
            build_card(Rank.KING),
            build_card(Rank.JACK),
            (0, 0, 2),
        ),
    ],
)
def test_0005_rank_condition_jokers(
    joker_name: JokersName,
    hit_card: Card,
    miss_card: Card,
    expected: tuple[int, int, float],
):
    assert _score_joker(joker_name, card=hit_card) == expected
    assert _score_joker(joker_name, card=miss_card) == (0, 0, 1.0)


@pytest.mark.parametrize(
    ("joker_name", "hit_card", "expected"),
    [
        (JokersName.EVEN_STEVEN, build_card(Rank.TEN), (0, 0, 1.0)),
        (JokersName.ODD_TODD, build_card(Rank.ACE), (0, 0, 1.0)),
        (JokersName.SCHOLAR, build_card(Rank.ACE), (0, 0, 1.0)),
        (JokersName.FIBONACCI, build_card(Rank.FIVE), (0, 0, 1.0)),
        (JokersName.WALKIE_TALKIE, build_card(Rank.TEN), (0, 0, 1.0)),
    ],
)
def test_0006_rank_alias_condition_jokers_currently_do_not_hit(
    joker_name: JokersName,
    hit_card: Card,
    expected: tuple[int, int, float],
):
    assert _score_joker(joker_name, card=hit_card) == expected


def test_0007_photograph_only_hits_first_played_face_card():
    assert _score_joker(
        JokersName.PHOTOGRAPH,
        card=build_card(Rank.KING),
        face_card_count=0,
    ) == (0, 0, 2)
    assert _score_joker(
        JokersName.PHOTOGRAPH,
        card=build_card(Rank.KING),
        face_card_count=1,
    ) == (0, 0, 1.0)
    assert _score_joker(
        JokersName.PHOTOGRAPH,
        card=build_card(Rank.TEN),
        face_card_count=0,
    ) == (0, 0, 1.0)


@pytest.mark.parametrize(
    ("joker_name", "hit_card", "miss_card", "expected"),
    [
        (
            JokersName.GREEDY_JOKER,
            build_card(Rank.ACE, Suit.DIAMONDS),
            build_card(Rank.ACE, Suit.HEARTS),
            (0, 3, 1.0),
        ),
        (
            JokersName.LUSTY_JOKER,
            build_card(Rank.ACE, Suit.HEARTS),
            build_card(Rank.ACE, Suit.SPADES),
            (0, 3, 1.0),
        ),
        (
            JokersName.WRATHFUL_JOKER,
            build_card(Rank.ACE, Suit.SPADES),
            build_card(Rank.ACE, Suit.CLUBS),
            (0, 3, 1.0),
        ),
        (
            JokersName.GLUTTONOUS_JOKER,
            build_card(Rank.ACE, Suit.CLUBS),
            build_card(Rank.ACE, Suit.DIAMONDS),
            (0, 3, 1.0),
        ),
        (
            JokersName.SEEING_DOUBLE,
            build_card(Rank.ACE, Suit.CLUBS),
            build_card(Rank.ACE, Suit.HEARTS),
            (0, 0, 2),
        ),
        (
            JokersName.BLOODSTONE,
            build_card(Rank.ACE, Suit.HEARTS),
            build_card(Rank.ACE, Suit.CLUBS),
            (0, 0, 1.5),
        ),
        (
            JokersName.ARROWHEAD,
            build_card(Rank.ACE, Suit.SPADES),
            build_card(Rank.ACE, Suit.CLUBS),
            (50, 0, 1.0),
        ),
        (
            JokersName.ONYX_AGATE,
            build_card(Rank.ACE, Suit.CLUBS),
            build_card(Rank.ACE, Suit.SPADES),
            (0, 7, 1.0),
        ),
    ],
)
def test_0008_suit_condition_jokers(
    joker_name: JokersName,
    hit_card: Card,
    miss_card: Card,
    expected: tuple[int, int, float],
):
    assert _score_joker(joker_name, card=hit_card) == expected
    assert _score_joker(joker_name, card=miss_card) == (0, 0, 1.0)


def test_0009_suit_condition_wild_cards_hit_and_stone_cards_miss():
    assert _score_joker(
        JokersName.GREEDY_JOKER,
        card=build_card(Rank.ACE, Suit.HEARTS, Enhancement.WILD),
    ) == (0, 3, 1.0)
    assert _score_joker(
        JokersName.GREEDY_JOKER,
        card=build_card(Rank.ACE, Suit.DIAMONDS, Enhancement.STONE),
    ) == (0, 0, 1.0)


def test_0010_four_suit_condition_jokers():
    four_suits = [
        build_card(Rank.ACE, Suit.HEARTS),
        build_card(Rank.KING, Suit.DIAMONDS),
        build_card(Rank.QUEEN, Suit.CLUBS),
        build_card(Rank.JACK, Suit.SPADES),
    ]
    three_suits = four_suits[:3]

    assert _score_joker(JokersName.FLOWER_POT, scoring_played=four_suits) == (
        0,
        0,
        3,
    )
    assert _score_joker(JokersName.FLOWER_POT, scoring_played=three_suits) == (
        0,
        0,
        1.0,
    )


def test_0011_forced_suit_condition_jokers():
    assert _score_joker(
        JokersName.BLACKBOARD,
        unscoring_held=[
            build_card(Rank.ACE, Suit.CLUBS),
            build_card(Rank.KING, Suit.SPADES),
        ],
    ) == (0, 0, 3)
    assert _score_joker(
        JokersName.BLACKBOARD,
        unscoring_held=[
            build_card(Rank.ACE, Suit.CLUBS),
            build_card(Rank.KING, Suit.HEARTS),
        ],
    ) == (0, 0, 1.0)


def test_0012_required_suit_condition_jokers():
    assert _score_joker(
        JokersName.ANCIENT_JOKER,
        card=build_card(Rank.ACE, Suit.SPADES),
        req_suit=Suit.SPADES,
    ) == (0, 0, 1.5)
    assert _score_joker(
        JokersName.ANCIENT_JOKER,
        card=build_card(Rank.ACE, Suit.HEARTS),
        req_suit=Suit.SPADES,
    ) == (0, 0, 1.0)


def test_0013_required_rank_and_suit_condition_jokers_currently_do_not_hit():
    idol_card = build_card(Rank.ACE, Suit.HEARTS)

    assert _score_joker(
        JokersName.THE_IDOL,
        card=idol_card,
        req_rank=Rank.ACE,
        req_suit=Suit.HEARTS,
    ) == (0, 0, 1.0)


def test_0014_raised_fist_scores_lowest_held_card():
    lowest_held = build_card(Rank.FIVE)
    higher_held = build_card(Rank.KING)

    assert _score_joker(
        JokersName.RAISED_FIST,
        card=lowest_held,
        scoring_held=[lowest_held, higher_held],
    ) == (0, 10, 1.0)
    assert _score_joker(
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
def test_0015_runtime_state_condition_jokers_currently_raise_value_error(
    joker_name: JokersName,
):
    with pytest.raises(ValueError):
        _score_joker(joker_name)
