from core.enums import (
    Edition,
    Enhancement,
    JokerEdition,
    JokersName,
    Planet,
    Rank,
    Seal,
    Spectral,
    Suit,
    Tarot,
)

type Feature = (
    Rank
    | Suit
    | Enhancement
    | Edition
    | Seal
    | Tarot
    | Planet
    | Spectral
    | JokersName
    | JokerEdition
)

FEATURE_TYPES = (
    Rank,
    Suit,
    Enhancement,
    Edition,
    Seal,
    Tarot,
    Planet,
    Spectral,
    JokersName,
    JokerEdition,
)
