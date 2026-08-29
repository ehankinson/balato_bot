import pytest
from _test_util import build_card, buildable_jokers_of_type

from calculation.joker_scoring import calculate_joker_scoring
from core.enums import Enhancement, JokerName, Rank, Suit
from core.models import (
    Card,
    Joker,
    JokerReq,
    JokerScoring,
    JokerScoringConditions,
)


def _build_scoring_joker(
    joker_name: JokerName,
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
    joker_name: JokerName,
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
        JokerName.JOKER,
        JokerName.JOLLY_JOKER,
        JokerName.ZANY_JOKER,
        JokerName.MAD_JOKER,
        JokerName.CRAZY_JOKER,
        JokerName.DROLL_JOKER,
        JokerName.HALF_JOKER,
        JokerName.STONE_JOKER,
        JokerName.ACROBAT,
        JokerName.GREEDY_JOKER,
        JokerName.LUSTY_JOKER,
        JokerName.WRATHFUL_JOKER,
        JokerName.GLUTTONOUS_JOKER,
        JokerName.BANNER,
        JokerName.MYSTIC_SUMMIT,
        JokerName.LOYALTY_CARD,
        JokerName.MISPRINT,
        JokerName.STEEL_JOKER,
        JokerName.RAISED_FIST,
        JokerName.SCARY_FACE,
        JokerName.ABSTRACT_JOKER,
        JokerName.EVEN_STEVEN,
        JokerName.ODD_TODD,
        JokerName.SCHOLAR,
        JokerName.SUPERNOVA,
        JokerName.SEEING_DOUBLE,
        JokerName.THE_DUO,
        JokerName.THE_TRIO,
        JokerName.THE_FAMILY,
        JokerName.THE_ORDER,
        JokerName.THE_TRIBE,
        JokerName.FIBONACCI,
        JokerName.JOKER_STENCIL,
        JokerName.CEREMONIAL_DAGGER,
        JokerName.SWASHBUCKLER,
        JokerName.FLOWER_POT,
        JokerName.RID_THE_BUS,
        JokerName.SHOOT_THE_MOON,
        JokerName.GROS_MICHEL,
        JokerName.STUNTMAN,
        JokerName.DRIVERS_LICENSE,
        JokerName.THROWBACK,
        JokerName.THE_IDOL,
        JokerName.BLOODSTONE,
        JokerName.ARROWHEAD,
        JokerName.ONYX_AGATE,
        JokerName.CANIO_BACKGROUND,
        JokerName.TRIBOULET_BACKGROUND,
        JokerName.YORICK_BACKGROUND,
        JokerName.BOOTSTRAPS,
        JokerName.BLACKBOARD,
        JokerName.RUNNER,
        JokerName.ICE_CREAM,
        JokerName.CONSTELLATION,
        JokerName.HIKER,
        JokerName.GREEN_JOKER,
        JokerName.CAVENDISH,
        JokerName.CARD_SHARP,
        JokerName.RED_CARD,
        JokerName.MADNESS,
        JokerName.SQUARE_JOKER,
        JokerName.VAMPIRE,
        JokerName.HOLOGRAM_BACKGROUND,
        JokerName.BARON,
        JokerName.OBELISK,
        JokerName.PHOTOGRAPH,
        JokerName.EROSION,
        JokerName.SLY_JOKER,
        JokerName.WILY_JOKER,
        JokerName.CLEVER_JOKER,
        JokerName.DEVIOUS_JOKER,
        JokerName.CRAFTY_JOKER,
        JokerName.LUCKY_CAT,
        JokerName.BASEBALL_CARD,
        JokerName.BULL,
        JokerName.FLASH_CARD,
        JokerName.POPCORN,
        JokerName.RAMEN,
        JokerName.SPARE_TROUSERS,
        JokerName.CAMPFIRE,
        JokerName.SMILEY_FACE,
        JokerName.ANCIENT_JOKER,
        JokerName.WALKIE_TALKIE,
        JokerName.CASTLE,
    }
    assert buildable_jokers_of_type(JokerScoring) == tested_jokers


@pytest.mark.parametrize(
    ("joker_name", "expected"),
    [
        (JokerName.JOKER, (0, 4, 1.0)),
        (JokerName.STONE_JOKER, (0, 0, 1.0)),
        (JokerName.ACROBAT, (0, 0, 1)),
        (JokerName.BANNER, (0, 0, 1.0)),
        (JokerName.MYSTIC_SUMMIT, (0, 0, 1.0)),
        (JokerName.LOYALTY_CARD, (0, 0, 1)),
        (JokerName.MISPRINT, (0, 23, 1.0)),
        (JokerName.STEEL_JOKER, (0, 0, 1)),
        (JokerName.ABSTRACT_JOKER, (0, 0, 1.0)),
        (JokerName.SUPERNOVA, (0, 0, 1.0)),
        (JokerName.JOKER_STENCIL, (0, 0, 1)),
        (JokerName.CEREMONIAL_DAGGER, (0, 0, 1.0)),
        (JokerName.SWASHBUCKLER, (0, 0, 1.0)),
        (JokerName.RID_THE_BUS, (0, 0, 1.0)),
        (JokerName.STUNTMAN, (250, 0, 1.0)),
        (JokerName.THROWBACK, (0, 0, 1)),
        (JokerName.CANIO_BACKGROUND, (0, 0, 1)),
        (JokerName.YORICK_BACKGROUND, (0, 0, 1)),
        (JokerName.BOOTSTRAPS, (0, 0, 1.0)),
        (JokerName.RUNNER, (0, 0, 1.0)),
        (JokerName.ICE_CREAM, (100, 0, 1.0)),
        (JokerName.CONSTELLATION, (0, 0, 1)),
        (JokerName.HIKER, (0, 0, 1.0)),
        (JokerName.GREEN_JOKER, (0, 0, 1.0)),
        (JokerName.RED_CARD, (0, 0, 1.0)),
        (JokerName.MADNESS, (0, 0, 1)),
        (JokerName.SQUARE_JOKER, (0, 0, 1.0)),
        (JokerName.VAMPIRE, (0, 0, 1)),
        (JokerName.HOLOGRAM_BACKGROUND, (0, 0, 1)),
        (JokerName.OBELISK, (0, 0, 1)),
        (JokerName.EROSION, (0, 0, 1.0)),
        (JokerName.LUCKY_CAT, (0, 0, 1)),
        (JokerName.BULL, (0, 0, 1.0)),
        (JokerName.FLASH_CARD, (0, 0, 1.0)),
        (JokerName.POPCORN, (0, 20, 1.0)),
        (JokerName.RAMEN, (0, 0, 2)),
        (JokerName.SPARE_TROUSERS, (0, 0, 1.0)),
        (JokerName.CAMPFIRE, (0, 0, 1)),
        (JokerName.CASTLE, (0, 0, 1.0)),
    ],
)
def test_0002_static_scoring_jokers(
    joker_name: JokerName,
    expected: tuple[int, int, float],
):
    assert _score_joker(joker_name) == expected


@pytest.mark.parametrize(
    ("joker_name", "hit_cards", "miss_cards", "expected"),
    [
        (
            JokerName.JOLLY_JOKER,
            [build_card(Rank.ACE), build_card(Rank.ACE)],
            [build_card(Rank.ACE), build_card(Rank.KING)],
            (0, 8, 1.0),
        ),
        (
            JokerName.ZANY_JOKER,
            [build_card(Rank.ACE), build_card(Rank.ACE), build_card(Rank.ACE)],
            [build_card(Rank.ACE), build_card(Rank.ACE)],
            (0, 12, 1.0),
        ),
        (
            JokerName.MAD_JOKER,
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
            JokerName.CRAZY_JOKER,
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
            JokerName.DROLL_JOKER,
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
            JokerName.THE_DUO,
            [build_card(Rank.ACE), build_card(Rank.ACE)],
            [build_card(Rank.ACE), build_card(Rank.KING)],
            (0, 0, 2),
        ),
        (
            JokerName.THE_TRIO,
            [build_card(Rank.ACE), build_card(Rank.ACE), build_card(Rank.ACE)],
            [build_card(Rank.ACE), build_card(Rank.ACE)],
            (0, 0, 3),
        ),
        (
            JokerName.THE_FAMILY,
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
            JokerName.THE_ORDER,
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
            JokerName.THE_TRIBE,
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
            JokerName.SLY_JOKER,
            [build_card(Rank.ACE), build_card(Rank.ACE)],
            [build_card(Rank.ACE), build_card(Rank.KING)],
            (50, 0, 1.0),
        ),
        (
            JokerName.WILY_JOKER,
            [build_card(Rank.ACE), build_card(Rank.ACE), build_card(Rank.ACE)],
            [build_card(Rank.ACE), build_card(Rank.ACE)],
            (100, 0, 1.0),
        ),
        (
            JokerName.CLEVER_JOKER,
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
            JokerName.DEVIOUS_JOKER,
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
            JokerName.CRAFTY_JOKER,
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
    joker_name: JokerName,
    hit_cards: list[Card],
    miss_cards: list[Card],
    expected: tuple[int, int, float],
):
    assert _score_joker(joker_name, scoring_played=hit_cards) == expected
    assert _score_joker(joker_name, scoring_played=miss_cards) == (0, 0, 1.0)


def test_0004_cards_played_condition_jokers():
    assert _score_joker(
        JokerName.HALF_JOKER,
        scoring_played=[
            build_card(Rank.ACE),
            build_card(Rank.KING),
            build_card(Rank.QUEEN),
        ],
    ) == (0, 20, 1.0)
    assert _score_joker(
        JokerName.HALF_JOKER,
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
            JokerName.SCARY_FACE,
            build_card(Rank.KING),
            build_card(Rank.TEN),
            (30, 0, 1.0),
        ),
        (
            JokerName.SMILEY_FACE,
            build_card(Rank.QUEEN),
            build_card(Rank.ACE),
            (0, 5, 1.0),
        ),
        (
            JokerName.SHOOT_THE_MOON,
            build_card(Rank.QUEEN),
            build_card(Rank.KING),
            (0, 13, 1.0),
        ),
        (JokerName.BARON, build_card(Rank.KING), build_card(Rank.QUEEN), (0, 0, 1.5)),
        (
            JokerName.TRIBOULET_BACKGROUND,
            build_card(Rank.KING),
            build_card(Rank.JACK),
            (0, 0, 2),
        ),
    ],
)
def test_0005_rank_condition_jokers(
    joker_name: JokerName,
    hit_card: Card,
    miss_card: Card,
    expected: tuple[int, int, float],
):
    assert _score_joker(joker_name, card=hit_card) == expected
    assert _score_joker(joker_name, card=miss_card) == (0, 0, 1.0)


@pytest.mark.parametrize(
    ("joker_name", "hit_card", "expected"),
    [
        (JokerName.EVEN_STEVEN, build_card(Rank.TEN), (0, 0, 1.0)),
        (JokerName.ODD_TODD, build_card(Rank.ACE), (0, 0, 1.0)),
        (JokerName.SCHOLAR, build_card(Rank.ACE), (0, 0, 1.0)),
        (JokerName.FIBONACCI, build_card(Rank.FIVE), (0, 0, 1.0)),
        (JokerName.WALKIE_TALKIE, build_card(Rank.TEN), (0, 0, 1.0)),
    ],
)
def test_0006_rank_alias_condition_jokers_currently_do_not_hit(
    joker_name: JokerName,
    hit_card: Card,
    expected: tuple[int, int, float],
):
    assert _score_joker(joker_name, card=hit_card) == expected


def test_0007_photograph_only_hits_first_played_face_card():
    assert _score_joker(
        JokerName.PHOTOGRAPH,
        card=build_card(Rank.KING),
        face_card_count=0,
    ) == (0, 0, 2)
    assert _score_joker(
        JokerName.PHOTOGRAPH,
        card=build_card(Rank.KING),
        face_card_count=1,
    ) == (0, 0, 1.0)
    assert _score_joker(
        JokerName.PHOTOGRAPH,
        card=build_card(Rank.TEN),
        face_card_count=0,
    ) == (0, 0, 1.0)


@pytest.mark.parametrize(
    ("joker_name", "hit_card", "miss_card", "expected"),
    [
        (
            JokerName.GREEDY_JOKER,
            build_card(Rank.ACE, Suit.DIAMONDS),
            build_card(Rank.ACE, Suit.HEARTS),
            (0, 3, 1.0),
        ),
        (
            JokerName.LUSTY_JOKER,
            build_card(Rank.ACE, Suit.HEARTS),
            build_card(Rank.ACE, Suit.SPADES),
            (0, 3, 1.0),
        ),
        (
            JokerName.WRATHFUL_JOKER,
            build_card(Rank.ACE, Suit.SPADES),
            build_card(Rank.ACE, Suit.CLUBS),
            (0, 3, 1.0),
        ),
        (
            JokerName.GLUTTONOUS_JOKER,
            build_card(Rank.ACE, Suit.CLUBS),
            build_card(Rank.ACE, Suit.DIAMONDS),
            (0, 3, 1.0),
        ),
        (
            JokerName.SEEING_DOUBLE,
            build_card(Rank.ACE, Suit.CLUBS),
            build_card(Rank.ACE, Suit.HEARTS),
            (0, 0, 2),
        ),
        (
            JokerName.BLOODSTONE,
            build_card(Rank.ACE, Suit.HEARTS),
            build_card(Rank.ACE, Suit.CLUBS),
            (0, 0, 1.5),
        ),
        (
            JokerName.ARROWHEAD,
            build_card(Rank.ACE, Suit.SPADES),
            build_card(Rank.ACE, Suit.CLUBS),
            (50, 0, 1.0),
        ),
        (
            JokerName.ONYX_AGATE,
            build_card(Rank.ACE, Suit.CLUBS),
            build_card(Rank.ACE, Suit.SPADES),
            (0, 7, 1.0),
        ),
    ],
)
def test_0008_suit_condition_jokers(
    joker_name: JokerName,
    hit_card: Card,
    miss_card: Card,
    expected: tuple[int, int, float],
):
    assert _score_joker(joker_name, card=hit_card) == expected
    assert _score_joker(joker_name, card=miss_card) == (0, 0, 1.0)


def test_0009_suit_condition_wild_cards_hit_and_stone_cards_miss():
    assert _score_joker(
        JokerName.GREEDY_JOKER,
        card=build_card(Rank.ACE, Suit.HEARTS, Enhancement.WILD),
    ) == (0, 3, 1.0)
    assert _score_joker(
        JokerName.GREEDY_JOKER,
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

    assert _score_joker(JokerName.FLOWER_POT, scoring_played=four_suits) == (
        0,
        0,
        3,
    )
    assert _score_joker(JokerName.FLOWER_POT, scoring_played=three_suits) == (
        0,
        0,
        1.0,
    )


def test_0011_forced_suit_condition_jokers():
    assert _score_joker(
        JokerName.BLACKBOARD,
        unscoring_held=[
            build_card(Rank.ACE, Suit.CLUBS),
            build_card(Rank.KING, Suit.SPADES),
        ],
    ) == (0, 0, 3)
    assert _score_joker(
        JokerName.BLACKBOARD,
        unscoring_held=[
            build_card(Rank.ACE, Suit.CLUBS),
            build_card(Rank.KING, Suit.HEARTS),
        ],
    ) == (0, 0, 1.0)


def test_0012_required_suit_condition_jokers():
    assert _score_joker(
        JokerName.ANCIENT_JOKER,
        card=build_card(Rank.ACE, Suit.SPADES),
        req_suit=Suit.SPADES,
    ) == (0, 0, 1.5)
    assert _score_joker(
        JokerName.ANCIENT_JOKER,
        card=build_card(Rank.ACE, Suit.HEARTS),
        req_suit=Suit.SPADES,
    ) == (0, 0, 1.0)


def test_0013_required_rank_and_suit_condition_jokers_currently_do_not_hit():
    idol_card = build_card(Rank.ACE, Suit.HEARTS)

    assert _score_joker(
        JokerName.THE_IDOL,
        card=idol_card,
        req_rank=Rank.ACE,
        req_suit=Suit.HEARTS,
    ) == (0, 0, 1.0)


def test_0014_raised_fist_scores_lowest_held_card():
    lowest_held = build_card(Rank.FIVE)
    higher_held = build_card(Rank.KING)

    assert _score_joker(
        JokerName.RAISED_FIST,
        card=lowest_held,
        scoring_held=[lowest_held, higher_held],
    ) == (0, 10, 1.0)
    assert _score_joker(
        JokerName.RAISED_FIST,
        card=higher_held,
        scoring_held=[lowest_held, higher_held],
    ) == (0, 0, 1.0)


@pytest.mark.parametrize(
    "joker_name",
    [
        JokerName.GROS_MICHEL,
        JokerName.CAVENDISH,
        JokerName.DRIVERS_LICENSE,
        JokerName.CARD_SHARP,
        JokerName.BASEBALL_CARD,
    ],
)
def test_0015_runtime_state_condition_jokers_currently_raise_value_error(
    joker_name: JokerName,
):
    with pytest.raises(ValueError):
        _score_joker(joker_name)
