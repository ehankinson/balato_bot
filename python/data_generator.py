import os
import random

from ultralytics import YOLO

from create_hand import create_hand
from vision import get_card_locations_in_hand
from const import CURR_DIR, Enhancement

def generate_hand_training_data(hand_amount: int) -> None:
    cutoff = hand_amount * 0.9
    for i in range(hand_amount):
        card_amount = random.randint(2, 15)
        image, data = create_hand(card_amount)

        name = f"{i}_{card_amount}"
        split = "train" if i < cutoff else "val"

        image_path = f"training_data/card_data/images/{split}"
        if not os.path.isdir(image_path):
            os.mkdir(image_path)

        label_path = f"training_data/card_data/labels/{split}"
        if not os.path.isdir(label_path):
            os.mkdir(label_path)

        img_path = f"{image_path}/{name}.png"
        label_path = f"{label_path}/{name}.txt"

        image.save(img_path)

        with open(label_path, "w", encoding="utf-8") as t:
            for d in data:
                line = " ".join([str(val) for val in d["box"]])
                t.write(line + "\n")



def generate_rank_training_data(hand_amount: int, model: YOLO) -> None:
    cutoff = hand_amount * 0.9
    for i in range(hand_amount):
        split = "train" if i < cutoff else "val"

        card_amount = random.randint(2, 15)
        image, data = create_hand(card_amount)

        results = model(image)
        cards = get_card_locations_in_hand(results)

        image_path = f"training_data/rank_data/{split}/"
        if not os.path.isdir(image_path):
            os.mkdir(image_path)

        for j, d in enumerate(data):
            rank = d["features"][0]
            if d["features"][2] == Enhancement.STONE:
                continue # skipping stones

            rank_image_path = os.path.join(image_path, f"{rank}/")
            if not os.path.isdir(rank_image_path):
                os.mkdir(rank_image_path)

            card = image.crop((cards[j][0], cards[j][1], cards[j][2], cards[j][3]))

            w, h = card.size
            rank_crop = card.crop((
                0,
                0,
                int(w * 0.28),
                int(h * 0.25),
            ))
            rank_crop.save(f"{rank_image_path}{i*j + j}_{rank}.png")


if __name__ == '__main__':
    card_model = YOLO(os.path.join(CURR_DIR, "../models/card_selector.pt"))
    generate_rank_training_data(5000, card_model)
