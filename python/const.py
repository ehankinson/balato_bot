import os

from ultralytics import YOLO

CURR_DIR = os.path.dirname(os.path.abspath(__file__))

FOLDER_TRAINING_NAMES = ["rank", "suit", "enhancement", "seal"]

CARD_ID = 0 # used for the yolo training
BOX_MODEL = YOLO(os.path.join(CURR_DIR, "../models/card_selector.pt"))