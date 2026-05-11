from core.models import Joker


def passed_retrigger_condition(condition: str, condition_args: dict) -> bool:
    val = False
    
    match condition:
        case "first_played_card":
            val = condition_args["card_pos"] == 0

        case "final_hand":
            val = condition_args["hands_left"] == 1

        case "low_card":
            val = condition_args["card"].is_low_card

        case "facecard":
            val = condition_args["card"].is_facecard
            
    return val


def calculate_joker_retrigger(joker: Joker, condition_args: dict) -> int:
    retrigger_data = joker.retrigger
    assert retrigger_data is not None

    condition = retrigger_data.condition
    passes_condition = (
        passed_retrigger_condition(condition, condition_args)
        if condition is not None
        else True
    )

    return retrigger_data.times if passes_condition else 0
