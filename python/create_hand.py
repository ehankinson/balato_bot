# 1445x393 THIS IS THE DIMENSIONS OF the SS WE ARE TAKING
import os
import random
from PIL import Image

from create_card import create_card
from const import Card, Suit, Rank, Seal, Enhancement, CARD_ID

Y_LIFT = 5
ANGLE = 1.4
X_START_GAP = 32
Y_START_GAP = 52
IMAGE_WIDTH = 1445
IMAGE_HEIGHT = 393
WIDTH_MULT = 1.625
HEIGHT_MULT = 1.625
TOTAL_HAND_WIDTH = 1372

def calculate_card_gap(card_amount: int, card_width: int):
    total_card_space = card_amount * card_width
    shift_space = TOTAL_HAND_WIDTH - total_card_space
    shift_per_card = shift_space / (card_amount - 1)
    return shift_per_card



def create_hand(card_amount: int) -> [Image, list[list[float]]]:
    img = Image.new("RGBA", (IMAGE_WIDTH, IMAGE_HEIGHT), color="green")
    mid = (card_amount - 1) / 2
    card_gap = None
    card_data = []

    for i in range(card_amount):
        card = Card(
            random.choice(list(Rank)),
            random.choice(list(Suit)),
            random.choice(list(Enhancement)),
            random.choice(list(Seal))
        )

        card_image = create_card(card).image

        card_image = card_image.resize(
            (
                int(card_image.width * WIDTH_MULT),
                int(card_image.height * HEIGHT_MULT)
            ),
            Image.Resampling.LANCZOS
        )

        offset = i - mid
        angle = -offset * ANGLE

        if card_gap is None:
            card_gap = calculate_card_gap(card_amount, card_image.width)

        x_pos = int(X_START_GAP + i * (card_image.width + card_gap))

        offset = mid - abs(mid - i)
        y_pos = int(Y_START_GAP - (Y_LIFT * offset))

        card_image = card_image.rotate(angle, expand=True)
        img.paste(card_image, (x_pos, y_pos), card_image)

        card_w, card_h = card_image.width, card_image.height
        center_x = round((x_pos + card_w / 2) / IMAGE_WIDTH, 6)
        center_y = round((y_pos + card_h / 2) / IMAGE_HEIGHT, 6)
        norm_width = round(card_w / IMAGE_WIDTH, 6)
        norm_height = round(card_h / IMAGE_HEIGHT, 6)

        card_data.append([
            CARD_ID, center_x, center_y, norm_width, norm_height
        ])

    return img, card_data



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
                line = " ".join([str(val) for val in d])
                t.write(line + "\n")



if __name__ == '__main__':
    generate_hand_training_data(2000)