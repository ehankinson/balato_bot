from core.enums import Enhancement, JokerName, JokerTriggers
from core.models import HandScoring, JokerScoring, JokerUpdate


def calculate_joker_update(
    jokers: list[JokerScoring | JokerUpdate],
    hand_scoring: HandScoring,
    trigger: JokerTriggers,
) -> None:
    for joker in jokers:
        if isinstance(joker, JokerUpdate) and joker.trigger == trigger:
            if joker.joker_name == JokerName.MIDAS_MASK:
                for card in hand_scoring.scored_played:
                    if card.is_facecard:
                        card.remove_enhancement()
                        card.enhancement = Enhancement(joker.enhacnement)
                        card.add_enhancement()

        elif (
            isinstance(joker, JokerScoring)
            and joker.update is not None
            and joker.update.trigger == trigger
        ):
            for card in hand_scoring.scored_played:
                if card.enhancement == Enhancement.NONE:
                    continue

                card.remove_enhancement()
                match joker.update.effect:

                    case "x_mult":
                        joker.x_mult += joker.update.change
