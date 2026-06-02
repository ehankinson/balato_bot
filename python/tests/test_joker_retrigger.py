import pytest

from core.enums import JokersName, Rank
from core.models import Card, JokerRetrigger
from _test_util import buildable_jokers_of_type, build_card, retrigger_joker


def test_0001_every_buildable_retrigger_joker_has_direct_tests():
    tested_jokers = {
        JokersName.SOCK_AND_BUSKIN,
        JokersName.MIME,
        JokersName.HACK,
        JokersName.HANGING_CHAD,
        JokersName.DUSK,
        JokersName.SELTZER,
    }
    assert buildable_jokers_of_type(JokerRetrigger) == tested_jokers


@pytest.mark.parametrize(
    ("joker_name", "card", "expected"),
    [
        (JokersName.SOCK_AND_BUSKIN, build_card(Rank.KING), 1),
        (JokersName.SOCK_AND_BUSKIN, build_card(Rank.TEN), 0),
        (JokersName.HACK, build_card(Rank.FIVE), 1),
        (JokersName.HACK, build_card(Rank.SIX), 0),
    ],
)
def test_0002_retrigger_card_conditions(
    joker_name: JokersName,
    card: Card,
    expected: int,
):
    assert retrigger_joker(joker_name, card) == expected


@pytest.mark.parametrize("joker_name", [JokersName.MIME, JokersName.SELTZER])
def test_0003_empty_retrigger_conditions_currently_do_not_trigger(
    joker_name: JokersName,
):
    assert retrigger_joker(joker_name, build_card(Rank.ACE)) == 0


def test_0004_hanging_chad_only_retriggers_first_played_card():
    assert retrigger_joker(JokersName.HANGING_CHAD, build_card(Rank.ACE), card_index=0) == 2
    assert retrigger_joker(JokersName.HANGING_CHAD, build_card(Rank.ACE), card_index=1) == 0


def test_0005_dusk_only_retriggers_on_final_hand():
    assert retrigger_joker(JokersName.DUSK, build_card(Rank.ACE), hands_left=1) == 1
    assert retrigger_joker(JokersName.DUSK, build_card(Rank.ACE), hands_left=2) == 0
