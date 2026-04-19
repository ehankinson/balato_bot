import os

from ultralytics import YOLO
from const import CURR_DIR

def main():
    trainer_config = os.path.join(CURR_DIR, "../yaml/card_trainer.yaml")

    model = YOLO("yolo11n.pt")

    model.train(
        data=trainer_config,
        epochs=20,
        imgsz=640, # scales input to be this square
        batch=16,
        device=0, # 0 GPU / 1 CPU
        workers=4, # how many threads to use to load the data
        patience=20 # after x epchos if no change quite
    )

if __name__ == "__main__":
    main()