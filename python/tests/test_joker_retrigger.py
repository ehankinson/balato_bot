import pytest
from _test_util import build_card

from calculation.joker_retrigger import calculate_joker_retrigger
from core.enums import JokerName, Rank
from core.models import Card, Joker, JokerRetrigger, JokerScoringConditions


def _build_retrigger_joker(joker_name: JokerName) -> JokerRetrigger:
    joker = Joker.build(joker_name)
    assert isinstance(joker, JokerRetrigger), (
        f"The joker {joker_name} is not type JokerRetrigger"
    )
    return joker


@pytest.mark.parametrize(
    ("joker_name", "card", "expected"),
    [
        (JokerName.SOCK_AND_BUSKIN, build_card(Rank.KING), 1),
        (JokerName.SOCK_AND_BUSKIN, build_card(Rank.TEN), 0),
        (JokerName.SOCK_AND_BUSKIN, build_card(Rank.ACE), 0),
        (JokerName.HACK, build_card(Rank.FIVE), 1),
        (JokerName.HACK, build_card(Rank.SIX), 0),
    ],
)
def test_0001_retrigger_card_conditions(
    joker_name: JokerName,
    card: Card,
    expected: int,
):
    condition_args = JokerScoringConditions(card=card)
    joker = _build_retrigger_joker(joker_name)
    retrigger = calculate_joker_retrigger(joker, condition_args)
    assert retrigger == expected, (
        f"We expected to have a value of '{expected}' but got '{retrigger}'"
    )


def test_0002_hanging_chad_only_retriggers_first_played_card():
    condition_args = JokerScoringConditions(card_index=0)
    joker = _build_retrigger_joker(JokerName.HANGING_CHAD)

    retrigger = calculate_joker_retrigger(joker, condition_args)
    assert retrigger == 2, f"We expected to have a value of '2' but got '{retrigger}'"

    condition_args.card_index = 1
    retrigger = calculate_joker_retrigger(joker, condition_args)
    assert retrigger == 0, f"We expected to have a value of '0' but got '{retrigger}'"


def test_0003_dusk_triggers_on_the_last_hand():
    condition_args = JokerScoringConditions(hands_left=2)
    joker = _build_retrigger_joker(JokerName.DUSK)

    retrigger = calculate_joker_retrigger(joker, condition_args)
    assert retrigger == 0, f"We expected to have a value of '0' but got '{retrigger}'"

    condition_args.hands_left = 1
    retrigger = calculate_joker_retrigger(joker, condition_args)
    assert retrigger == 1, f"We expected to have a value of '1' but got '{retrigger}'"
