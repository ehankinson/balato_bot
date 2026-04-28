import os
import sys

if sys.platform == "darwin":
    hand_dimenstions = [ 845, 224 ]
elif sys.platform.startswith("linux"):
    hand_dimenstions = [ 1445, 393 ]
else:
    raise OSError("Don't support this system")

from ultralytics import YOLO

HAND_WIDTH = hand_dimenstions[0]
HAND_HEIGHT = hand_dimenstions[1]

HAND_WIDTH_RATIO: float = 230.75 / 1445
HAND_HEIGHT_RATION: float = 308.75 / 393

X_RATIO_GAP: float = 32 / 1445
Y_RATIO_GAP: float = 52 / 393

CURR_DIR = os.path.dirname(os.path.abspath(__file__))
ROOT_DIR = os.path.join(CURR_DIR, "..")

TRAINING_CONFIG = os.path.join(ROOT_DIR, "json", "training_config.json")

FOLDER_TRAINING_NAMES = ["rank", "suit", "enhancement", "seal"]

CARD_ID = 0 # used for the yolo training
BOX_MODEL = YOLO(os.path.join(CURR_DIR, "../models/card_selector.pt"))

RANK_CROP = [0.0, 0.0, 0.30, 0.25]
SEAL_CROP = [55.0, 35.0, 0.6, 0.45]
SUIT_CROP = [10.0, 45.0, 0.25, 0.3]
ENHANCEMENT_CROP = [5.0, 75.0, 0.25, 0.85]
