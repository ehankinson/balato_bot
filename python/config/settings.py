import os

import mss

# All render and capture geometry is measured against this 1440p baseline.
# Desktop-global mouse coordinates below are intentionally excluded: they depend
# on the user's multi-monitor layout rather than the size of the game capture.
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

# Helpers for dimensions that are expressed in baseline screen pixels.
def scale_width(value: int | float) -> int:
    return round(value * W_RATIO)


def scale_height(value: int | float) -> int:
    return round(value * H_RATIO)


# Hand capture and playing-card render geometry.
_BASE_HAND_WIDTH = 1445
_BASE_HAND_HEIGHT = 393
_BASE_HAND_CROP_LEFT = 670
_BASE_HAND_CROP_TOP = 800
_BASE_RENDERED_CARD_WIDTH = 230.75
_BASE_RENDERED_CARD_HEIGHT = 308.75
_BASE_HAND_X_START_GAP = 32
_BASE_HAND_Y_START_GAP = 52
_BASE_HAND_CONTENT_WIDTH = 1372

HAND_WIDTH = scale_width(_BASE_HAND_WIDTH)
HAND_HEIGHT = scale_height(_BASE_HAND_HEIGHT)
HAND_CROP_LEFT = scale_width(_BASE_HAND_CROP_LEFT)
HAND_CROP_TOP = scale_height(_BASE_HAND_CROP_TOP)
RENDERED_CARD_WIDTH = scale_width(_BASE_RENDERED_CARD_WIDTH)
RENDERED_CARD_HEIGHT = scale_height(_BASE_RENDERED_CARD_HEIGHT)
HAND_X_START_GAP = scale_width(_BASE_HAND_X_START_GAP)
HAND_Y_START_GAP = scale_height(_BASE_HAND_Y_START_GAP)
HAND_CONTENT_WIDTH = scale_width(_BASE_HAND_CONTENT_WIDTH)

# Joker render geometry.
_BASE_JOKER_CANVAS_WIDTH = 1150
_BASE_JOKER_CANVAS_HEIGHT = 350
_BASE_JOKER_CONTENT_WIDTH = 1120
_BASE_JOKER_X_PADDING = 18

JOKER_CANVAS_WIDTH = scale_width(_BASE_JOKER_CANVAS_WIDTH)
JOKER_CANVAS_HEIGHT = scale_height(_BASE_JOKER_CANVAS_HEIGHT)
JOKER_CONTENT_WIDTH = scale_width(_BASE_JOKER_CONTENT_WIDTH)
JOKER_X_PADDING = scale_width(_BASE_JOKER_X_PADDING)

# Consumable render geometry.
_BASE_CONSUMABLE_CANVAS_WIDTH = 535
_BASE_CONSUMABLE_CANVAS_HEIGHT = 310

CONSUMABLE_CANVAS_WIDTH = scale_width(_BASE_CONSUMABLE_CANVAS_WIDTH)
CONSUMABLE_CANVAS_HEIGHT = scale_height(_BASE_CONSUMABLE_CANVAS_HEIGHT)

# Shared layout variation. Angles are resolution-independent; vertical pixel
# offsets use the vertical screen scale.
LAYOUT_ANGLE = 5.6
LAYOUT_ANGLE_JITTER = 0.5
LAYOUT_Y_JITTER = scale_height(5)
LAYOUT_MAX_Y_LIFT = scale_height(18)

# Desktop-global coordinates for the user's multi-monitor layout. Do not scale.
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

# Pixel offsets are measured on a baseline rendered card; right/bottom are
# fractional bounds and therefore already resolution-independent.
RANK_CROP = [0, 0, 0.42, 0.35]
SEAL_CROP = [scale_width(55), scale_height(35), 0.6, 0.45]
SUIT_CROP = [scale_width(8), 0, 0.25, 0.36]
ENHANCEMENT_CROP = [scale_width(5), scale_height(75), 0.25, 0.85]
EDITION_CROP = [scale_width(5), scale_height(75), 0.25, 0.85]
JOKER_NAME_CROP = [scale_width(10), scale_height(10), 0.6, 0.95]
JOKER_EDITION_CROP = JOKER_NAME_CROP
CONSUMABLE_CROP = [0, scale_height(115), 0.55, 0.95]

BACKGROUND_PALETTES = [
    ((31, 122, 77), (68, 164, 95), (18, 78, 62)),
    ((142, 120, 28), (205, 175, 54), (86, 72, 24)),
    ((121, 41, 55), (183, 62, 79), (73, 29, 43)),
    ((43, 75, 137), (77, 126, 193), (26, 47, 92)),
    ((84, 52, 135), (135, 82, 186), (48, 34, 86)),
]
BACKGROUND_POOL_SIZE = 32
