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



def create_hand(card_amount: int) -> [Image, list[dict[int, list[float]]]]:
    img = Image.new("RGBA", (IMAGE_WIDTH, IMAGE_HEIGHT), color="green")
    mid = (card_amount - 1) / 2
    card_gap = None
    card_data = []

    for i in range(card_amount):
        rank = random.choice(list(Rank))
        suit = random.choice(list(Suit))
        enha = random.choice(list(Enhancement))
        seal = random.choice(list(Seal))

        card = Card(rank, suit, enha, seal)
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

        info = {
            "box": [CARD_ID, center_x, center_y, norm_width, norm_height],
            "features": [rank, suit, enha, seal]
        }
        card_data.append(info)

    return img, card_data