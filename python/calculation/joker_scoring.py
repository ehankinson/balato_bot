from calculation.poker_eval import (
    contain_n_of_a_kind,
    contains_2_pair,
    is_flush,
    is_straight,
)
from core.models import Card, Joker


def calculate_scoring_condition(
    condition_key: str,
    condition_value: int,
    condition_args: dict,
    condition: dict,
    joker: Joker,
) -> bool:
    match condition_key:
        case "hand_type":
            if condition_value < 5:
                return contain_n_of_a_kind(condition_value, condition_args["hand"])

            match condition_value:
                case 6:
                    return contains_2_pair(condition_args["hand"])

                case 7:
                    return is_straight(condition_args["hand"])

                case 8:
                    return is_flush(condition_args["hand"])

        case "rank":
            if isinstance(condition_value, list):
                return condition_args["card"].rank in condition_value

            if condition_value == "facecard":
                return condition_args["card"].is_facecard

        case "suit":
            value = condition["suit"]

            if isinstance(value, dict):
                if value == "suit" and joker.req is not None:
                    return joker.req["suit"] == condition_args["card"].suit

            return (
                condition_args["card"].suit == value
                or condition_args["card"].is_any_suit
            )

        case "cards_played":
            return len(condition_args["hand"]) <= condition_value

    raise ValueError(f"This condition key {condition_key} does not exist")


def calculate_joker_scoring(
    joker: Joker, condition_args: dict
) -> tuple[int, int, float]:
    chips, add_mult, x_mult = 0, 0, 1.0
    scoring_data = joker.scoring
    assert scoring_data is not None

    condition = scoring_data.condition
    passes_condition = (
        all(
            calculate_scoring_condition(key, value, condition_args, condition, joker)
            for key, value in condition.items()
        )
        if condition is not None
        else True
    )

    if not passes_condition:
        return chips, add_mult, x_mult

    if scoring_data.add_mult is not None:
        if isinstance(scoring_data.add_mult, str):
            lowest_rank = float('inf') 
            lowest_card: Card = Card.random()
            for card in condition_args["cards_not_played"]:
                if card.rank < lowest_rank:
                    lowest_card = card
                    lowest_rank = card.rank
                
            rank = lowest_card.rank + 2 if not lowest_card.is_facecard else 10
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
