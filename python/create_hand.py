# 1445x393 THIS IS THE DIMENSIONS OF the SS WE ARE TAKING
import random
from PIL import Image

from create_card import create_card
from const import Card, Suit, Rank, Seal, Enhancement

Y_LIFT = 5
ANGLE = 1.4
X_START_GAP = 32
Y_START_GAP = 52
HAND_WIDTH = 1445
HAND_HEIGHT = 393
WIDTH_MULT = 1.625
HEIGHT_MULT = 1.625
TOTAL_HAND_WIDTH = 1372

def calculate_card_gap(card_amount: int, card_width: int):
    total_card_space = card_amount * card_width
    shift_space = TOTAL_HAND_WIDTH - total_card_space
    shift_per_card = shift_space / (card_amount - 1)
    return shift_per_card



def create_hand(card_amount: int) -> Image:
    img = Image.new("RGBA", (HAND_WIDTH, HAND_HEIGHT), color="green")
    mid = (card_amount - 1) / 2
    card_gap = None

    for i in range(card_amount):
        card = Card(
            random.choice(list(Rank)),
            random.choice(list(Suit)),
            Enhancement.NONE,
            Seal.NONE
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
    
    return img