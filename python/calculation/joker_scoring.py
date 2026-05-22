from calculation.poker_eval import (
    contain_n_of_a_kind,
    contains_2_pair,
    is_flush,
    is_straight,
)
from core.enums import Enhancement, Rank, Suit
from core.models import Card, Joker, JokerScoringConditions


def calculate_scoring_condition(
    condition_key: str,
    condition_value: int,
    condition_args: JokerScoringConditions,
    condition: dict,
    joker: Joker,
) -> bool:
    card = condition_args.card
    match condition_key:
        case "hand_type":
            if condition_value < 5:
                return contain_n_of_a_kind(
                    condition_value, condition_args.scoring_played
                )

            match condition_value:
                case 6:
                    return contains_2_pair(condition_args.scoring_played)

                case 7:
                    return is_straight(condition_args.scoring_played)

                case 8:
                    return is_flush(condition_args.scoring_played)

        case "rank":
            if card.enhancement == Enhancement.STONE:
                return False

            if isinstance(condition_value, list):
                return card.rank in condition_value

            if condition_value == "facecard":
                return card.is_facecard
            else:
                return card.rank == condition_value

        case "suit":
            if card.enhancement == Enhancement.STONE:
                return False

            value = condition["suit"]
            if isinstance(value, str):
                return joker.req["suit"] == card.suit

            elif isinstance(value, int):
                return card.is_any_suit or card.suit == value

            else:
                if "forced" in condition:
                    cards_not_played = (
                        condition_args.scoring_held + condition_args.unscoring_held
                    )
                    return len(cards_not_played) == 0 or all(
                        card.suit in [Suit.SPADES, Suit.CLUBS]
                        for card in cards_not_played
                    )

                else:
                    played_suits = {card.suit for card in condition_args.scoring_played}
                    return len(played_suits) == len(value)

        case "cards_played":
            return len(condition_args.scoring_played) <= condition_value

    raise ValueError(f"This condition key {condition_key} does not exist")


def calculate_joker_scoring(
    joker: Joker, condition_args: JokerScoringConditions
) -> tuple[int, int, float]:
    chips, add_mult, x_mult = 0, 0, 1.0
    scoring_data = joker.scoring
    assert scoring_data is not None

    condition = scoring_data.condition
    passes_condition = (
        all(
            calculate_scoring_condition(key, value, condition_args, condition, joker)
            for key, value in condition.items()
            if key != "forced"
        )
        if condition is not None
        else True
    )

    if not passes_condition:
        return chips, add_mult, x_mult

    if scoring_data.add_mult is not None:
        if isinstance(scoring_data.add_mult, str):
            lowest_card = condition_args.scoring_held[
                0
            ]  # this will always be the lowest card

            if lowest_card == condition_args.card:
                rank = (
                    11
                    if lowest_card.rank == Rank.ACE
                    else lowest_card.rank + 2
                    if not lowest_card.is_facecard
                    else 10
                )
                add_mult += rank * 2

        elif isinstance(scoring_data.add_mult, int):
            add_mult += scoring_data.add_mult

    if scoring_data.chips is not None:
        if isinstance(scoring_data.chips, int):
            chips += scoring_data.chips

    if scoring_data.x_mult is not None:
        if isinstance(scoring_data.x_mult, dict):
            x_mult *= 1
        else:
            x_mult *= scoring_data.x_mult

    return chips, add_mult, x_mult
