import os

from ultralytics import YOLO

from config.settings import ROOT_DIR


CARD_BOX_MODEL = YOLO(os.path.join(ROOT_DIR, "models/card_selector.pt"))
JOKER_BOX_MODEL = YOLO(os.path.join(ROOT_DIR, "models/joker_selector.pt"))
