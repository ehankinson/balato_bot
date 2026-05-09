from core.models import Joker
from calculation.poker_eval import contain_n_of_a_kind

def calculate_joker_condition(
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

        case "rank":
            if condition_value == "facecard":
                return condition_args["card"].is_facecard

        case "suit":
            if isinstance(condition, dict):
                value = condition["suit"]["req"]
                if value == "suit" and joker.req is not None:
                    return joker.req["suit"] == condition_args["card"].suit

                return condition_args["card"].suit == value

    raise ValueError(f"This condition key {condition_key} does not exist")