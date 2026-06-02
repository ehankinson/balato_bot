from dataclasses import dataclass


@dataclass(frozen=True)
class HandStats:
    name: int = -1
    chips: int = -1
    mult: int = -1
    level: int = 1
