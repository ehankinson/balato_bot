from typing import Iterable

from calculation.joker_retrigger import calculate_joker_retrigger
from calculation.joker_scoring import calculate_joker_scoring
from core.enums import Edition, Enhancement, JokersName, Rank, Seal, Suit
from core.models import (
    Card,
    Joker,
    JokerReq,
    JokerRetrigger,
    JokerScoring,
    JokerScoringConditions,
)


def build_card(
    rank: Rank,
    suit: Suit = Suit.HEARTS,
    enhancement: Enhancement = Enhancement.NONE,
    seal: Seal = Seal.NONE,
    edition: Edition = Edition.NONE,
) -> Card:
    return Card(rank, suit, enhancement, seal, edition)


def build_jokers(joker_names: Iterable[JokersName]) -> list[Joker]:
    return [Joker.build(joker_name) for joker_name in joker_names]


def buildable_jokers_of_type(joker_type: type[Joker]) -> set[JokersName]:
    buildable_jokers = set()

    for joker_name in JokersName:
        try:
            joker = Joker.build(joker_name)
        except Exception:
            continue

        if isinstance(joker, joker_type):
            buildable_jokers.add(joker_name)

    return buildable_jokers


def scoring_joker(
    joker_name: JokersName,
    *,
    req_rank: Rank = Rank.NONE,
    req_suit: Suit = Suit.NONE,
) -> JokerScoring:
    joker = Joker.build(joker_name)
    assert isinstance(joker, JokerScoring)
    joker.req = JokerReq(rank=req_rank, suit=req_suit)
    return joker


def score_joker(
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
    played = scoring_played or [card or build_card(Rank.ACE)]
    scoring_card = card or played[0]

    return calculate_joker_scoring(
        scoring_joker(joker_name, req_rank=req_rank, req_suit=req_suit),
        JokerScoringConditions(
            card=scoring_card,
            face_card_count=face_card_count,
            scoring_played=played,
            scoring_held=scoring_held or [],
            unscoring_held=unscoring_held or [],
        ),
    )
