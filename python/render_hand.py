# 1445x393 THIS IS THE DIMENSIONS OF the SS WE ARE TAKING
from PIL import Image

from const import CARD_ID
from render_card import render_card
from card_models import Card, Hand, RenderedHand, CardAnnotation


Y_LIFT = 5
ANGLE = 1.4
X_START_GAP = 32
Y_START_GAP = 52
IMAGE_WIDTH = 1445
IMAGE_HEIGHT = 393
TOTAL_HAND_WIDTH = 1372

def calculate_card_gap(card_amount: int, card_width: int) -> float:
    total_card_space = card_amount * card_width
    shift_space = TOTAL_HAND_WIDTH - total_card_space
    shift_per_card = shift_space / (card_amount - 1)
    return shift_per_card



def calculate_box_dimensions(card: Card, card_image: Image, x_pos: int, y_pos: int) -> CardAnnotation:
    card_w, card_h = card_image.width, card_image.height
    center_x = round((x_pos + card_w / 2) / IMAGE_WIDTH, 6)
    center_y = round((y_pos + card_h / 2) / IMAGE_HEIGHT, 6)
    norm_width = round(card_w / IMAGE_WIDTH, 6)
    norm_height = round(card_h / IMAGE_HEIGHT, 6)
    return CardAnnotation(
        card=card,
        box=[CARD_ID, center_x, center_y, norm_width, norm_height]
    )



def render_hand(hand: Hand) -> RenderedHand:
    img = Image.new("RGBA", (IMAGE_WIDTH, IMAGE_HEIGHT), color="green")

    card_amount = len(hand.cards)
    mid = (card_amount - 1) / 2
    card_gap: float = 0.0
    annotations: list[CardAnnotation] = []

    for i, card in enumerate(hand.cards):
        card_image = render_card(card)

        offset = i - mid
        angle = -offset * ANGLE

        if i == 0:
            card_gap = calculate_card_gap(card_amount, card_image.width)

        x_pos = int(X_START_GAP + i * (card_image.width + card_gap))

        offset = mid - abs(mid - i)
        y_pos = int(Y_START_GAP - (Y_LIFT * offset))

        card_image = card_image.rotate(angle, expand=True)
        img.paste(card_image, (x_pos, y_pos), card_image)

        annotations.append(calculate_box_dimensions(
            card, card_image, x_pos, y_pos
        ))

    return RenderedHand(
        image=img,
        annotations=annotations
    )
