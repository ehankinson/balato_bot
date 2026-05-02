import os
import sys

from ultralytics import YOLO
from card_enums import PokerHand
from card_models import HandStats

if sys.platform == "darwin":
    hand_dimenstions = [ 845, 224 ]
elif sys.platform.startswith("linux"):
    hand_dimenstions = [ 1445, 393 ]
else:
    raise OSError("Don't support this system")

HAND_WIDTH = hand_dimenstions[0]
HAND_HEIGHT = hand_dimenstions[1]

HAND_WIDTH_RATIO: float = 230.75 / 1445
HAND_HEIGHT_RATION: float = 308.75 / 393

X_RATIO_GAP: float = 32 / 1445
Y_RATIO_GAP: float = 52 / 393

CURR_DIR = os.path.dirname(os.path.abspath(__file__))
ROOT_DIR = os.path.join(CURR_DIR, "..")

TRAINING_CONFIG = os.path.join(ROOT_DIR, "json", "training_config.json")

FOLDER_TRAINING_NAMES = ["rank", "suit", "enhancement", "seal", "edition"]

CARD_ID = 0 # used for the yolo training
BOX_MODEL = YOLO(os.path.join(ROOT_DIR, "models/card_selector.pt"))

RANK_CROP = [0.0, 0.0, 0.42, 0.35]
SEAL_CROP = [55.0, 35.0, 0.6, 0.45]
SUIT_CROP = [12.0, 45.0, 0.28, 0.32]
ENHANCEMENT_CROP = [5.0, 75.0, 0.25, 0.85]
EDITION_CROP = [5.0, 75.0, 0.25, 0.85]

HAND_STATS: dict[PokerHand, HandStats] = {
    PokerHand.FLUSH_FIVE: HandStats(160, 16),
    PokerHand.FLUSH_HOUSE: HandStats(140, 14),
    PokerHand.FIVE_OF_A_KIND: HandStats(120, 12),
    PokerHand.STRAIGHT_FLUSH: HandStats(100, 8),
    PokerHand.FOUR_OF_A_KIND: HandStats(60, 7),
    PokerHand.FULL_HOUSE: HandStats(40, 4),
    PokerHand.FLUSH: HandStats(35, 4),
    PokerHand.STRAIGHT: HandStats(30, 4),
    PokerHand.THREE_OF_A_KIND: HandStats(30, 3),
    PokerHand.TWO_PAIR: HandStats(20, 2),
    PokerHand.PAIR: HandStats(10, 2),
    PokerHand.HIGH_CARD: HandStats(5, 1),
}
