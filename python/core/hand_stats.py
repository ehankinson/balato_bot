from dataclasses import dataclass


@dataclass(frozen=True)
class HandStats:
    chips: int
    mult: int
    level: int = 1

