import os

from ultralytics import YOLO

from config.settings import ROOT_DIR

LOCATION_MODEL = YOLO(os.path.join(ROOT_DIR, "models/location_model.pt"))
