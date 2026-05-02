import os
import sys


if sys.platform == "darwin":
    hand_dimenstions = [845, 224]
elif sys.platform.startswith("linux"):
    hand_dimenstions = [1445, 393]
else:
    raise OSError("Don't support this system")

HAND_WIDTH = hand_dimenstions[0]
HAND_HEIGHT = hand_dimenstions[1]

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

FOLDER_TRAINING_NAMES = ["rank", "suit", "enhancement", "seal", "edition"]

CARD_ID = 0
JOKER_ID = 1

RANK_CROP = [0.0, 0.0, 0.42, 0.35]
SEAL_CROP = [55.0, 35.0, 0.6, 0.45]
SUIT_CROP = [12.0, 45.0, 0.28, 0.32]
ENHANCEMENT_CROP = [5.0, 75.0, 0.25, 0.85]
EDITION_CROP = [5.0, 75.0, 0.25, 0.85]

BACKGROUND_PALETTES = [
    ((31, 122, 77), (68, 164, 95), (18, 78, 62)),
    ((142, 120, 28), (205, 175, 54), (86, 72, 24)),
    ((121, 41, 55), (183, 62, 79), (73, 29, 43)),
    ((43, 75, 137), (77, 126, 193), (26, 47, 92)),
    ((84, 52, 135), (135, 82, 186), (48, 34, 86)),
]
BACKGROUND_POOL_SIZE = 32

