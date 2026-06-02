from core.models import JokerRetrigger, JokerScoringConditions


def passed_retrigger_condition(
    condition: str, condition_args: JokerScoringConditions
) -> bool:
    val = False

    match condition:
        case "first_played_card":
            val = condition_args.card_index == 0

        case "final_hand":
            val = condition_args.hands_left == 1

        case "low_card":
            val = condition_args.card.is_low_card

        case "facecard":
            val = condition_args.card.is_facecard

    return val


def calculate_joker_retrigger(
    joker: JokerRetrigger, condition_args: JokerScoringConditions
) -> int:
    condition = joker.condition
    passes_condition = (
        passed_retrigger_condition(condition, condition_args)
        if condition is not None
        else True
    )

    return joker.times if passes_condition else 0
