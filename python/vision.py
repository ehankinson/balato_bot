# 1445x393 THIS IS THE DIMENSIONS OF the SS WE ARE TAKING
import random
from PIL import Image

from create_card import create_card
from const import Card, Suit, Rank, Seal, Enhancement


WIDTH_MULT = 1.625
HEIGHT_MULT = 1.625
ANGLE = 1.4
TOTAL_HAND_WIDTH = 1372
X_START_GAP = 32
Y_START_GAP = 52
Y_LIFT = 5

def calculate_card_gap(card_amount: int, card_width: int):
    total_card_space = card_amount * card_width
    shift_space = TOTAL_HAND_WIDTH - total_card_space
    shift_per_card = shift_space / (card_amount - 1)
    return shift_per_card


img = Image.new("RGB", (1445, 393), color="green").convert("RGBA")
card_amount = random.randint(5, 12)
mid = card_amount // 2
card_gap = None
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

    angle = (mid - i) * ANGLE if i < mid \
        else -((i - mid) * ANGLE) if card_amount % 2 == 1 \
            else -((i - mid + 1) * ANGLE)

    if card_gap is None:
        card_gap = calculate_card_gap(card_amount, card_image.width)

    x_pos = int(X_START_GAP + card_image.width * i + card_gap * i)

    y_add = Y_LIFT * i if i < mid \
        else Y_LIFT * (mid - (i - mid + 1)) if card_amount % 2 == 0 \
            else Y_LIFT * (mid - (i - mid))
    y_pos = Y_START_GAP - y_add

    card_image = card_image.rotate(angle, expand=True)
    img.paste(card_image, (x_pos, y_pos), card_image)

img.save(f"{card_amount}_cards.png")