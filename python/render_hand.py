import random

# 1445x393 THIS IS THE DIMENSIONS OF the SS WE ARE TAKING
from PIL import Image

from render_card import render_card
from card_models import Card, Hand, RenderedHand, CardAnnotation
from const import CARD_ID, HAND_WIDTH, HAND_HEIGHT, X_RATIO_GAP, Y_RATIO_GAP


MAX_Y_LIFT = 18
Y_JITTER = 0.5
ANGLE = 5.6
ANGLE_JITTER = 0.5
X_START_GAP = int(X_RATIO_GAP * HAND_WIDTH)
Y_START_GAP = int(Y_RATIO_GAP * HAND_HEIGHT)
TOTAL_HAND_WIDTH_RATIO: float = 1372 / 1445
TOTAL_HAND_WIDTH = TOTAL_HAND_WIDTH_RATIO * HAND_WIDTH

def calculate_card_gap(card_amount: int, card_width: int) -> float:
    if card_amount <= 1:
        return 0.0

    total_card_space = card_amount * card_width
    shift_space = TOTAL_HAND_WIDTH - total_card_space
    shift_per_card = shift_space / (card_amount - 1)
    return shift_per_card



def calculate_card_angle(card_index: int, card_amount: int) -> float:
    mid = (card_amount - 1) / 2
    if mid == 0:
        return 0.0

    offset_from_center = card_index - mid
    normalized_offset = offset_from_center / mid
    return -normalized_offset * ANGLE



def calculate_card_y_lift(card_index: int, card_amount: int) -> float:
    mid = (card_amount - 1) / 2
    if mid == 0:
        return 0.0

    distance_from_center = abs(card_index - mid)
    normalized_center_lift = 1 - (distance_from_center / mid)
    return normalized_center_lift * MAX_Y_LIFT



def calculate_box_dimensions(card: Card, card_image: Image, x_pos: int, y_pos: int) -> CardAnnotation:
    card_w, card_h = card_image.width, card_image.height
    center_x = round((x_pos + card_w / 2) / HAND_WIDTH, 6)
    center_y = round((y_pos + card_h / 2) / HAND_HEIGHT, 6)
    norm_width = round(card_w / HAND_WIDTH, 6)
    norm_height = round(card_h / HAND_HEIGHT, 6)
    return CardAnnotation(
        card=card,
        box=[CARD_ID, center_x, center_y, norm_width, norm_height]
    )



def render_hand(hand: Hand) -> RenderedHand:
    img = Image.new("RGBA", (HAND_WIDTH, HAND_HEIGHT), color="green")

    card_amount = len(hand.cards)
    card_gap: float = 0.0
    annotations: list[CardAnnotation] = []

    for i, card in enumerate(hand.cards):
        card_image = render_card(card)

        angle = calculate_card_angle(i, card_amount) + random.uniform(-ANGLE_JITTER, ANGLE_JITTER)

        if i == 0:
            card_gap = calculate_card_gap(card_amount, card_image.width)

        x_pos = int(X_START_GAP + i * (card_image.width + card_gap))
        y_jitter = random.uniform(-Y_JITTER, Y_JITTER)
        y_pos = round(Y_START_GAP - calculate_card_y_lift(i, card_amount) + y_jitter)

        card_image = card_image.rotate(angle, expand=True)
        img.paste(card_image, (x_pos, y_pos), card_image)

        annotations.append(calculate_box_dimensions(
            card, card_image, x_pos, y_pos
        ))


    return RenderedHand(
        image=img,
        annotations=annotations
    )

if __name__ == '__main__':
    hand = Hand.random_hand(56)
    render_hand(hand, debug_path="image.png")
