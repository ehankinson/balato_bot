from core.enums import Edition, Enhancement, Seal

ENHANCEMENT_SCORING = {
    Enhancement.NONE: 1,
    Enhancement.WILD: 1.25,
    Enhancement.STONE: 1.25,
    Enhancement.BONUS: 2,
    Enhancement.GOLD: 3,
    Enhancement.MULT: 3,
    Enhancement.STEEL: 4,
    Enhancement.LUCKY: 4,
    Enhancement.GLASS: 5,
}

SEAL_SCORING = {Seal.NONE: 1, Seal.GOLD: 3, Seal.BLUE: 4, Seal.PURPLE: 4, Seal.RED: 5}

EDITION_SCORING = {
    Edition.NONE: 1,
    Edition.FOIL: 3,
    Edition.HOLOGRAPHIC: 4,
    Edition.POLYCHROME: 5,
}


def get_initial_card_chips(rank: int) -> int:
    if rank < 8:
        return rank + 2

    if rank < 12:
        return 10

    return 11
