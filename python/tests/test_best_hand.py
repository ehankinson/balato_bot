import pytest
from _test_util import build_card, build_jokers

from best_hand import get_best_scoring_hand
from core.enums import Edition, Enhancement, JokersName, Rank, Seal, Suit
from core.models import BestHand


def best_hand(
    cards,
    joker_names: tuple[JokersName, ...] = (),
    *,
    scoring_type: str = "best",
) -> BestHand:
    return get_best_scoring_hand(
        cards,
        build_jokers(joker_names),
        scoring_type=scoring_type,
    )


def assert_score(
    score: BestHand,
    *,
    chips: int,
    best_mult: float,
    avg_mult: float | None = None,
    worst_mult: float | None = None,
):
    assert score.chips == chips
    assert score.best_case_mult == pytest.approx(best_mult)
    assert score.avg_case_mult == pytest.approx(
        avg_mult if avg_mult is not None else best_mult
    )
    assert score.worst_case_mult == pytest.approx(
        worst_mult if worst_mult is not None else best_mult
    )


def test_0001_high_card_scores_best_ace():
    score = best_hand(
        [
            build_card(Rank.ACE, Suit.HEARTS),
            build_card(Rank.KING, Suit.CLUBS),
        ]
    )

    assert_score(score, chips=16, best_mult=1)


def test_0002_pair_scores_best_pair():
    score = best_hand(
        [
            build_card(Rank.ACE, Suit.HEARTS),
            build_card(Rank.ACE, Suit.CLUBS),
            build_card(Rank.KING, Suit.SPADES),
        ]
    )

    assert_score(score, chips=32, best_mult=2)


def test_0003_two_pair_scores_best_two_pair():
    score = best_hand(
        [
            build_card(Rank.ACE, Suit.HEARTS),
            build_card(Rank.ACE, Suit.CLUBS),
            build_card(Rank.KING, Suit.SPADES),
            build_card(Rank.KING, Suit.DIAMONDS),
        ]
    )

    assert_score(score, chips=62, best_mult=2)


def test_0004_three_kind_scores_best_three_kind():
    score = best_hand(
        [
            build_card(Rank.QUEEN, Suit.HEARTS),
            build_card(Rank.QUEEN, Suit.CLUBS),
            build_card(Rank.QUEEN, Suit.SPADES),
            build_card(Rank.ACE, Suit.DIAMONDS),
        ]
    )

    assert_score(score, chips=60, best_mult=3)


def test_0005_four_kind_scores_best_four_kind():
    score = best_hand(
        [
            build_card(Rank.KING, Suit.HEARTS),
            build_card(Rank.KING, Suit.CLUBS),
            build_card(Rank.KING, Suit.SPADES),
            build_card(Rank.KING, Suit.DIAMONDS),
            build_card(Rank.QUEEN, Suit.HEARTS),
        ]
    )

    assert_score(score, chips=100, best_mult=7)


def test_0006_straight_scores_best_straight():
    score = best_hand(
        [
            build_card(Rank.ACE, Suit.HEARTS),
            build_card(Rank.KING, Suit.CLUBS),
            build_card(Rank.QUEEN, Suit.SPADES),
            build_card(Rank.JACK, Suit.DIAMONDS),
            build_card(Rank.TEN, Suit.CLUBS),
        ]
    )

    assert_score(score, chips=81, best_mult=4)


def test_0007_flush_scores_best_flush():
    score = best_hand(
        [
            build_card(Rank.ACE, Suit.HEARTS),
            build_card(Rank.KING, Suit.HEARTS),
            build_card(Rank.QUEEN, Suit.HEARTS),
            build_card(Rank.JACK, Suit.HEARTS),
            build_card(Rank.NINE, Suit.HEARTS),
        ]
    )

    assert_score(score, chips=85, best_mult=4)


def test_0008_full_house_scores_best_full_house():
    score = best_hand(
        [
            build_card(Rank.ACE, Suit.HEARTS),
            build_card(Rank.ACE, Suit.CLUBS),
            build_card(Rank.ACE, Suit.SPADES),
            build_card(Rank.KING, Suit.HEARTS),
            build_card(Rank.KING, Suit.CLUBS),
        ]
    )

    assert_score(score, chips=93, best_mult=4)


def test_0009_straight_flush_scores_best_straight_flush():
    score = best_hand(
        [
            build_card(Rank.ACE, Suit.HEARTS),
            build_card(Rank.KING, Suit.HEARTS),
            build_card(Rank.QUEEN, Suit.HEARTS),
            build_card(Rank.JACK, Suit.HEARTS),
            build_card(Rank.TEN, Suit.HEARTS),
        ]
    )

    assert_score(score, chips=151, best_mult=8)


def test_0010_five_kind_scores_best_five_kind():
    score = best_hand(
        [
            build_card(Rank.ACE, Suit.HEARTS),
            build_card(Rank.ACE, Suit.CLUBS),
            build_card(Rank.ACE, Suit.SPADES),
            build_card(Rank.ACE, Suit.DIAMONDS),
            build_card(Rank.ACE, Suit.HEARTS),
        ]
    )

    assert_score(score, chips=175, best_mult=12)


def test_0011_bonus_card_increases_chips():
    score = best_hand([build_card(Rank.ACE, Suit.HEARTS, Enhancement.BONUS)])

    assert_score(score, chips=46, best_mult=1)


def test_0012_foil_card_increases_chips():
    score = best_hand(
        [
            build_card(Rank.ACE, Suit.HEARTS, edition=Edition.FOIL),
        ]
    )

    assert_score(score, chips=66, best_mult=1)


def test_0013_mult_card_increases_mult():
    score = best_hand([build_card(Rank.ACE, Suit.HEARTS, Enhancement.MULT)])

    assert_score(score, chips=16, best_mult=5)


def test_0014_holographic_card_increases_mult():
    score = best_hand([build_card(Rank.ACE, Suit.HEARTS, edition=Edition.HOLOGRAPHIC)])

    assert_score(score, chips=16, best_mult=11)


def test_0015_glass_card_doubles_mult():
    score = best_hand([build_card(Rank.ACE, Suit.HEARTS, Enhancement.GLASS)])

    assert_score(score, chips=16, best_mult=2)


def test_0016_polychrome_card_multiplies_mult():
    score = best_hand([build_card(Rank.ACE, Suit.HEARTS, edition=Edition.POLYCHROME)])

    assert_score(score, chips=16, best_mult=1.5)


def test_0017_red_seal_retriggers_played_card_chips():
    score = best_hand([build_card(Rank.ACE, Suit.HEARTS, seal=Seal.RED)])

    assert_score(score, chips=27, best_mult=1)


def test_0018_red_seal_glass_retriggers_played_x_mult():
    score = best_hand([build_card(Rank.ACE, Suit.HEARTS, Enhancement.GLASS, Seal.RED)])

    assert_score(score, chips=27, best_mult=4)


def test_0019_red_seal_polychrome_glass_retriggers_played_x_mult():
    score = best_hand(
        [
            build_card(
                Rank.ACE, Suit.HEARTS, Enhancement.GLASS, Seal.RED, Edition.POLYCHROME
            ),
        ]
    )

    assert_score(score, chips=27, best_mult=9)


def test_0020_steel_card_scores_when_held():
    score = best_hand(
        [
            build_card(Rank.ACE, Suit.HEARTS),
            build_card(Rank.KING, Suit.CLUBS, Enhancement.STEEL),
        ]
    )

    assert_score(score, chips=16, best_mult=1.5)


def test_0021_red_seal_steel_card_scores_when_held():
    score = best_hand(
        [
            build_card(Rank.ACE, Suit.HEARTS),
            build_card(Rank.KING, Suit.CLUBS, Enhancement.STEEL, Seal.RED),
        ]
    )

    assert_score(score, chips=16, best_mult=2.25)


def test_0022_multiple_steel_cards_score_when_held():
    score = best_hand(
        [
            build_card(Rank.ACE, Suit.HEARTS),
            build_card(Rank.KING, Suit.CLUBS, Enhancement.STEEL),
            build_card(Rank.QUEEN, Suit.SPADES, Enhancement.STEEL),
        ]
    )

    assert_score(score, chips=16, best_mult=2.25)


def test_0023_stone_card_adds_to_played_chips():
    score = best_hand(
        [
            build_card(Rank.KING, Suit.CLUBS),
            build_card(Rank.ACE, Suit.HEARTS, Enhancement.STONE),
        ]
    )

    assert_score(score, chips=65, best_mult=1)


def test_0024_bonus_pair_scores_bonus_chips():
    score = best_hand(
        [
            build_card(Rank.ACE, Suit.HEARTS, Enhancement.BONUS),
            build_card(Rank.ACE, Suit.CLUBS),
        ]
    )

    assert_score(score, chips=62, best_mult=2)


def test_0025_mult_pair_scores_added_mult():
    score = best_hand(
        [
            build_card(Rank.ACE, Suit.HEARTS, Enhancement.MULT),
            build_card(Rank.ACE, Suit.CLUBS),
        ]
    )

    assert_score(score, chips=32, best_mult=6)


def test_0026_glass_pair_scores_x_mult():
    score = best_hand(
        [
            build_card(Rank.ACE, Suit.HEARTS, Enhancement.GLASS),
            build_card(Rank.ACE, Suit.CLUBS),
        ]
    )

    assert_score(score, chips=32, best_mult=4)


def test_0027_glass_and_mult_pair_orders_add_before_x_mult():
    score = best_hand(
        [
            build_card(Rank.ACE, Suit.HEARTS, Enhancement.GLASS),
            build_card(Rank.ACE, Suit.CLUBS, Enhancement.MULT),
        ]
    )

    assert_score(score, chips=32, best_mult=12)


def test_0028_lucky_card_tracks_best_avg_and_worst_mult_cases():
    score = best_hand([build_card(Rank.ACE, Suit.HEARTS, Enhancement.LUCKY)])

    assert_score(score, chips=16, best_mult=1, avg_mult=1, worst_mult=1)


def test_0029_best_scoring_type_currently_treats_lucky_card_as_neutral_mult():
    score = best_hand(
        [
            build_card(Rank.KING, Suit.CLUBS),
            build_card(Rank.ACE, Suit.HEARTS, Enhancement.LUCKY),
        ]
    )

    assert_score(score, chips=16, best_mult=1, avg_mult=1, worst_mult=1)


def test_0030_worst_scoring_type_chooses_plain_ace_over_lucky_king():
    score = best_hand(
        [
            build_card(Rank.KING, Suit.HEARTS, Enhancement.LUCKY),
            build_card(Rank.ACE, Suit.CLUBS),
        ],
        scoring_type="worst",
    )

    assert_score(score, chips=16, best_mult=1)


def test_0031_avg_scoring_type_currently_treats_lucky_card_as_neutral_mult():
    score = best_hand(
        [
            build_card(Rank.ACE, Suit.HEARTS, Enhancement.LUCKY),
            build_card(Rank.KING, Suit.CLUBS),
        ],
        scoring_type="avg",
    )

    assert_score(score, chips=16, best_mult=1, avg_mult=1, worst_mult=1)


def test_0032_flush_with_bonus_cards_scores_best_bonus_flush():
    score = best_hand(
        [
            build_card(Rank.ACE, Suit.HEARTS, Enhancement.BONUS),
            build_card(Rank.KING, Suit.HEARTS),
            build_card(Rank.QUEEN, Suit.HEARTS),
            build_card(Rank.JACK, Suit.HEARTS),
            build_card(Rank.NINE, Suit.HEARTS),
        ]
    )

    assert_score(score, chips=115, best_mult=4)


def test_0033_flush_with_mult_card_scores_best_mult_flush():
    score = best_hand(
        [
            build_card(Rank.ACE, Suit.HEARTS, Enhancement.MULT),
            build_card(Rank.KING, Suit.HEARTS),
            build_card(Rank.QUEEN, Suit.HEARTS),
            build_card(Rank.JACK, Suit.HEARTS),
            build_card(Rank.NINE, Suit.HEARTS),
        ]
    )

    assert_score(score, chips=85, best_mult=8)


def test_0034_flush_with_glass_card_scores_best_glass_flush():
    score = best_hand(
        [
            build_card(Rank.ACE, Suit.HEARTS, Enhancement.GLASS),
            build_card(Rank.KING, Suit.HEARTS),
            build_card(Rank.QUEEN, Suit.HEARTS),
            build_card(Rank.JACK, Suit.HEARTS),
            build_card(Rank.NINE, Suit.HEARTS),
        ]
    )

    assert_score(score, chips=85, best_mult=8)


def test_0035_flush_with_polychrome_glass_card_scores_best_flush():
    score = best_hand(
        [
            build_card(
                Rank.ACE, Suit.HEARTS, Enhancement.GLASS, edition=Edition.POLYCHROME
            ),
            build_card(Rank.KING, Suit.HEARTS),
            build_card(Rank.QUEEN, Suit.HEARTS),
            build_card(Rank.JACK, Suit.HEARTS),
            build_card(Rank.NINE, Suit.HEARTS),
        ]
    )

    assert_score(score, chips=85, best_mult=12)


def test_0036_full_house_with_steel_held_card_scores_steel_multiplier():
    score = best_hand(
        [
            build_card(Rank.ACE, Suit.HEARTS),
            build_card(Rank.ACE, Suit.CLUBS),
            build_card(Rank.ACE, Suit.SPADES),
            build_card(Rank.KING, Suit.HEARTS),
            build_card(Rank.KING, Suit.CLUBS),
            build_card(Rank.QUEEN, Suit.SPADES, Enhancement.STEEL),
        ]
    )

    assert_score(score, chips=93, best_mult=6)


def test_0037_straight_flush_with_stone_extra_card_adds_stone():
    score = best_hand(
        [
            build_card(Rank.ACE, Suit.HEARTS),
            build_card(Rank.KING, Suit.HEARTS),
            build_card(Rank.QUEEN, Suit.HEARTS),
            build_card(Rank.JACK, Suit.HEARTS),
            build_card(Rank.TEN, Suit.HEARTS),
            build_card(Rank.THREE, Suit.CLUBS, Enhancement.STONE),
        ]
    )

    assert_score(score, chips=151, best_mult=8)


def test_0038_best_score_result_is_besthand_object():
    score = best_hand([build_card(Rank.ACE, Suit.HEARTS)])

    assert isinstance(score, BestHand)


def test_0039_empty_joker_list_uses_no_joker_path():
    score = best_hand([build_card(Rank.ACE, Suit.HEARTS)], ())

    assert_score(score, chips=16, best_mult=1)


def test_0040_high_card_can_choose_foil_over_plain_ace():
    score = best_hand(
        [
            build_card(Rank.ACE, Suit.HEARTS),
            build_card(Rank.KING, Suit.CLUBS, edition=Edition.FOIL),
        ]
    )

    assert_score(score, chips=65, best_mult=1)


def test_0041_high_card_can_choose_holographic_king_over_plain_ace():
    score = best_hand(
        [
            build_card(Rank.ACE, Suit.HEARTS),
            build_card(Rank.KING, Suit.CLUBS, edition=Edition.HOLOGRAPHIC),
        ]
    )

    assert_score(score, chips=15, best_mult=11)


def test_0042_high_card_can_choose_polychrome_ace_over_plain_king():
    score = best_hand(
        [
            build_card(Rank.ACE, Suit.HEARTS, edition=Edition.POLYCHROME),
            build_card(Rank.KING, Suit.CLUBS),
        ]
    )

    assert_score(score, chips=16, best_mult=1.5)


def test_0043_pair_with_red_seal_card_retriggers_pair_card():
    score = best_hand(
        [
            build_card(Rank.ACE, Suit.HEARTS, seal=Seal.RED),
            build_card(Rank.ACE, Suit.CLUBS),
        ]
    )

    assert_score(score, chips=43, best_mult=2)


def test_0044_pair_with_red_seal_mult_card_retriggers_add_mult():
    score = best_hand(
        [
            build_card(Rank.ACE, Suit.HEARTS, Enhancement.MULT, Seal.RED),
            build_card(Rank.ACE, Suit.CLUBS),
        ]
    )

    assert_score(score, chips=43, best_mult=10)


def test_0045_pair_with_red_seal_glass_card_retriggers_x_mult():
    score = best_hand(
        [
            build_card(Rank.ACE, Suit.HEARTS, Enhancement.GLASS, Seal.RED),
            build_card(Rank.ACE, Suit.CLUBS),
        ]
    )

    assert_score(score, chips=43, best_mult=8)


def test_0046_stone_card_does_not_create_high_card_by_itself():
    score = best_hand(
        [
            build_card(Rank.ACE, Suit.HEARTS, Enhancement.STONE),
            build_card(Rank.KING, Suit.CLUBS),
        ]
    )

    assert_score(score, chips=65, best_mult=1)


def test_0047_multiple_stone_cards_can_fill_played_hand_slots():
    score = best_hand(
        [
            build_card(Rank.KING, Suit.CLUBS),
            build_card(Rank.ACE, Suit.HEARTS, Enhancement.STONE),
            build_card(Rank.QUEEN, Suit.SPADES, Enhancement.STONE),
        ]
    )

    assert_score(score, chips=115, best_mult=1)


def test_0048_straight_ace_low_currently_raises_value_error():
    with pytest.raises(ValueError):
        best_hand(
            [
                build_card(Rank.ACE, Suit.HEARTS),
                build_card(Rank.FIVE, Suit.CLUBS),
                build_card(Rank.FOUR, Suit.SPADES),
                build_card(Rank.THREE, Suit.DIAMONDS),
                build_card(Rank.TWO, Suit.CLUBS),
            ]
        )


def test_0049_best_hand_with_more_than_five_cards_chooses_best_available_hand():
    score = best_hand(
        [
            build_card(Rank.ACE, Suit.HEARTS),
            build_card(Rank.ACE, Suit.CLUBS),
            build_card(Rank.ACE, Suit.SPADES),
            build_card(Rank.KING, Suit.DIAMONDS),
            build_card(Rank.KING, Suit.HEARTS),
            build_card(Rank.QUEEN, Suit.CLUBS),
        ]
    )

    assert_score(score, chips=93, best_mult=4)


def test_0050_best_hand_with_many_cards_keeps_best_straight_flush_score():
    score = best_hand(
        [
            build_card(Rank.ACE, Suit.HEARTS),
            build_card(Rank.KING, Suit.HEARTS),
            build_card(Rank.QUEEN, Suit.HEARTS),
            build_card(Rank.JACK, Suit.HEARTS),
            build_card(Rank.TEN, Suit.HEARTS),
            build_card(Rank.ACE, Suit.CLUBS),
            build_card(Rank.ACE, Suit.SPADES),
            build_card(Rank.KING, Suit.CLUBS),
        ]
    )

    assert_score(score, chips=151, best_mult=8)


def test_0051_joker_add_mult_public_best_hand_currently_raises_name_error():
    with pytest.raises(NameError):
        best_hand([build_card(Rank.ACE, Suit.HEARTS)], (JokersName.JOKER,))


def test_0052_jolly_joker_pair_public_best_hand_currently_raises_name_error():
    with pytest.raises(NameError):
        best_hand(
            [
                build_card(Rank.ACE, Suit.HEARTS),
                build_card(Rank.ACE, Suit.CLUBS),
            ],
            (JokersName.JOLLY_JOKER,),
        )


def test_0053_zany_joker_three_kind_public_best_hand_currently_raises_name_error():
    with pytest.raises(NameError):
        best_hand(
            [
                build_card(Rank.QUEEN, Suit.HEARTS),
                build_card(Rank.QUEEN, Suit.CLUBS),
                build_card(Rank.QUEEN, Suit.SPADES),
            ],
            (JokersName.ZANY_JOKER,),
        )


def test_0054_blackboard_baron_mime_public_best_hand_currently_raises_name_error():
    with pytest.raises(NameError):
        best_hand(
            [
                build_card(Rank.ACE, Suit.HEARTS),
                build_card(Rank.KING, Suit.DIAMONDS),
                build_card(Rank.KING, Suit.CLUBS, Enhancement.STEEL, Seal.RED),
                build_card(Rank.QUEEN, Suit.SPADES),
                build_card(Rank.JACK, Suit.CLUBS),
                build_card(Rank.NINE, Suit.SPADES),
            ],
            (JokersName.BLACKBOARD, JokersName.BARON, JokersName.MIME),
        )


def test_0055_raised_fist_blackboard_baron_mime_public_best_hand_currently_raises_name_error():
    with pytest.raises(NameError):
        best_hand(
            [
                build_card(Rank.ACE, Suit.HEARTS),
                build_card(Rank.KING, Suit.HEARTS),
                build_card(Rank.KING, Suit.CLUBS, Enhancement.STEEL, Seal.RED),
                build_card(Rank.QUEEN, Suit.SPADES),
                build_card(Rank.JACK, Suit.CLUBS),
                build_card(Rank.NINE, Suit.SPADES),
            ],
            (
                JokersName.RAISED_FIST,
                JokersName.BLACKBOARD,
                JokersName.BARON,
                JokersName.MIME,
            ),
        )


def test_0056_triboulet_photograph_hanging_chad_public_best_hand_currently_raises_name_error():
    with pytest.raises(NameError):
        best_hand(
            [
                build_card(
                    Rank.KING,
                    Suit.HEARTS,
                    Enhancement.GLASS,
                    Seal.RED,
                    Edition.POLYCHROME,
                ),
                build_card(Rank.QUEEN, Suit.CLUBS, Enhancement.GLASS),
                build_card(Rank.ACE, Suit.SPADES),
            ],
            (
                JokersName.TRIBOULET_BACKGROUND,
                JokersName.PHOTOGRAPH,
                JokersName.HANGING_CHAD,
            ),
        )


def test_0057_two_blueprints_baron_mime_public_best_hand_complex_case():
    score = best_hand(
        [
            build_card(Rank.ACE, Suit.HEARTS),
            build_card(Rank.KING, Suit.CLUBS, Enhancement.STEEL, Seal.RED),
            build_card(Rank.KING, Suit.SPADES, Enhancement.STEEL, Seal.RED),
            build_card(Rank.QUEEN, Suit.DIAMONDS),
        ],
        (JokersName.BLUEPRINT, JokersName.BLUEPRINT, JokersName.BARON, JokersName.MIME),
    )

    assert score.chips > 0
