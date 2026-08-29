from core.enums import (
    Edition,
    Enhancement,
    JokerEdition,
    JokerName,
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
    | JokerName
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
    JokerName,
    JokerEdition,
)
