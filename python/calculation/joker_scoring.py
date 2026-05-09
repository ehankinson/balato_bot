from calculation.joker_condition import calculate_joker_condition
from core.models import Joker


def calculate_joker_scoring(
    joker: Joker, condition_args: dict
) -> tuple[int, int, float]:
    chips, add_mult, x_mult = 0, 0, 1.0
    scoring_data = joker.scoring
    assert scoring_data is not None

    condition = scoring_data.condition
    passes_condition = (
        all(
            calculate_joker_condition(key, value, condition_args, condition, joker)
            for key, value in condition.items()
        )
        if condition is not None
        else True
    )

    if not passes_condition:
        return chips, add_mult, x_mult

    if scoring_data.add_mult is not None:
        if isinstance(scoring_data.add_mult, int):
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
