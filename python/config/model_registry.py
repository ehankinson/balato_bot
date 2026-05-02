import os

from ultralytics import YOLO

from config.settings import ROOT_DIR


BOX_MODEL = YOLO(os.path.join(ROOT_DIR, "models/card_selector.pt"))

