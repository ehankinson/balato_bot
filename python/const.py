import os

from ultralytics import YOLO

CURR_DIR = os.path.dirname(os.path.abspath(__file__))
ROOT_DIR = os.path.join(CURR_DIR, "..")

TRAINING_CONFIG = os.path.join(ROOT_DIR, "json", "training_config.json")

FOLDER_TRAINING_NAMES = ["rank", "suit", "enhancement", "seal"]

CARD_ID = 0 # used for the yolo training
BOX_MODEL = YOLO(os.path.join(CURR_DIR, "../models/card_selector.pt"))

RANK_CROP = (0, 0, 0.28, 0.25)
SUIT_CROP = (0, 35, 0.22, 0.35)
ENHANCEMENT_CROP = (5, 35, 0.25, 0.85)
