from dataclasses import dataclass


@dataclass(frozen=True)
class HandStats:
    name: int
    chips: int
    mult: int
    level: int = 1
