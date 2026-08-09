import os

import mss

# Baseline resolution the bot was originally tuned for
BASE_SCREEN_WIDTH = 2560
BASE_SCREEN_HEIGHT = 1440


def _get_screen_ratios() -> tuple[float, float]:
    """Return width and height scale ratios relative to the baseline resolution.

    Falls back to 1.0 (baseline) if no physical monitor is detected.
    """
    with mss.mss() as sct:
        if len(sct.monitors) < 2:
            return 1.0, 1.0
        primary = sct.monitors[1]
        screen_w = primary["width"]
        screen_h = primary["height"]
    return screen_w / BASE_SCREEN_WIDTH, screen_h / BASE_SCREEN_HEIGHT


W_RATIO, H_RATIO = _get_screen_ratios()

# Original hand crop dimensions at the baseline 2560x1440 resolution
_BASE_HAND_WIDTH = 1445
_BASE_HAND_HEIGHT = 393

HAND_WIDTH = int(_BASE_HAND_WIDTH * W_RATIO)
HAND_HEIGHT = int(_BASE_HAND_HEIGHT * H_RATIO)

HAND_CROP_TOP = 800
HAND_CROP_LEFT = 670

PLAY_HAND_X = 3672
PLAY_HAND_Y = 1824

SELECT_BLIND_1_X = 3474
SELECT_BLIND_1_Y = 1084

SELECT_BLIND_2_X = 3944
SELECT_BLIND_2_Y = 1085

NEXT_ROUND_X = 3521
NEXT_ROUND_Y = 1182

CASH_OUT_X = 3813
CASH_OUT_Y = 1181

CARD_WIDTH = 142
CARD_HEIGHT = 190
WIDTH_MULT = 1.625
HEIGHT_MULT = 1.625

HAND_WIDTH_RATIO: float = 230.75 / 1445
HAND_HEIGHT_RATION: float = 308.75 / 393

X_RATIO_GAP: float = 32 / 1445
Y_RATIO_GAP: float = 52 / 393

CURR_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ROOT_DIR = os.path.join(CURR_DIR, "..")

TRAINING_CONFIG = os.path.join(ROOT_DIR, "json", "training_config.json")
JOKER_CONFIG = os.path.join(ROOT_DIR, "json", "main_joker_config.json")

FOLDER_TRAINING_NAMES = [
    "rank",
    "suit",
    "enhancement",
    "seal",
    "edition",
    "joker_type",
    "joker_edition",
]

BOX_ID = 0

RANK_CROP = [0.0, 0.0, 0.42, 0.35]
SEAL_CROP = [55.0, 35.0, 0.6, 0.45]
SUIT_CROP = [12.0, 45.0, 0.28, 0.32]
ENHANCEMENT_CROP = [5.0, 75.0, 0.25, 0.85]
EDITION_CROP = [5.0, 75.0, 0.25, 0.85]
JOKER_TYPE_CROP = [10.0, 10.0, 0.6, 0.95]
JOKER_EDITION_CROP = JOKER_TYPE_CROP
CONSUMABLE_CROP = [0.0, 115.0, 0.25, 0.95]

BACKGROUND_PALETTES = [
    ((31, 122, 77), (68, 164, 95), (18, 78, 62)),
    ((142, 120, 28), (205, 175, 54), (86, 72, 24)),
    ((121, 41, 55), (183, 62, 79), (73, 29, 43)),
    ((43, 75, 137), (77, 126, 193), (26, 47, 92)),
    ((84, 52, 135), (135, 82, 186), (48, 34, 86)),
]
BACKGROUND_POOL_SIZE = 32
