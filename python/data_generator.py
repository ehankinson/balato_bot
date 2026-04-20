from email.policy import default
import os

from PIL import Image
from ultralytics import YOLO

from util import random_card_amount
from render_hand import render_hand
from card_models import Card, Hand, RenderedHand
from vision import get_card_locations_in_hand
from card_enums import Rank, Suit, Seal, Enhancement, TrainingType
from const import CURR_DIR, BOX_MODEL, FOLDER_TRAINING_NAMES

CUTOFF = 0.9 # split between training and val

def generate_hand_training_data(hand_amount: int) -> None:
    cutoff = hand_amount * CUTOFF
    for i in range(hand_amount):
        card_amount = random_card_amount()
        hand = Hand.random_hand(card_amount)
        hand_render: RenderedHand = render_hand(hand)

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

        hand_render.image.save(img_path)

        with open(label_path, "w", encoding="utf-8") as t:
            box_values = hand_render.annotations
            for data in box_values:
                line = " ".join([str(val) for val in data.box])
                t.write(line + "\n")



def feature_info(train_type: TrainingType, card_image: Image, card: Card) -> tuple[Rank | Suit | Enhancement | Seal, list[int]]:
    w, h = card_image.size
    match train_type:
        case TrainingType.Rank:
            return card.rank, [0, 0, int(w * 0.28), int(h * 0.25)]

        case TrainingType.Suit:
            return card.suit, [0, 35, int(w * 0.22), int(h * 0.35)]

        case TrainingType.Enhancment:
            return card.enhancement, [0, 0, 0, 0]

        case TrainingType.Seal:
            return card.seal, [0, 0, 0, 0]



def generate_card_feature_data(hand_amount: int, train_type: TrainingType) -> None:
    cutoff = hand_amount * CUTOFF
    for hand_index in range(hand_amount):
        split = "train" if hand_index < cutoff else "val"

        card_amount = random_card_amount()
        hand = Hand.random_hand(card_amount)
        hand_render: RenderedHand = render_hand(hand)

        hand_image = hand_render.image
        results = BOX_MODEL(hand_image)
        card_locations = get_card_locations_in_hand(results)
        data_path = f"training_data/{FOLDER_TRAINING_NAMES[train_type]}_data/"
        if not os.path.isdir(data_path):
            os.mkdir(data_path)

        image_path = f"{data_path}{split}/"
        if not os.path.isdir(image_path):
            os.mkdir(image_path)

        annotations = hand_render.annotations
        for card_index, annotation in enumerate(annotations):
            card_data = annotation.card
            if card_data.enhancement == Enhancement.STONE and train_type != TrainingType.Enhancment:
                continue # skipping stones

            card_image = hand_image.crop((
                card_locations[card_index][0],
                card_locations[card_index][1],
                card_locations[card_index][2],
                card_locations[card_index][3]
            ))

            feature, crop_values = feature_info(train_type, card_image, card_data)
            feature_image_path = os.path.join(image_path, f"{feature}/")
            if not os.path.isdir(feature_image_path):
                os.mkdir(feature_image_path)

            feature_image = f"{feature_image_path}{hand_index * card_index + card_index}_{feature}.png"
            crop_feature = card_image.crop((crop_values))
            crop_feature.save(feature_image)



if __name__ == '__main__':
    generate_card_feature_data(5000, TrainingType.Suit)
