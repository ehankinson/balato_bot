import random

from config.settings import (
    HAND_HEIGHT,
    HAND_WIDTH,
)
from PIL import Image

ANGLE = 5.6
Y_JITTER = 5
MAX_Y_LIFT = 18
ANGLE_JITTER = 0.5

def calculate_angle(card_index: int, card_amount: int) -> float:
    mid = (card_amount - 1) / 2
    if mid == 0:
        return 0.0

    offset_from_center = card_index - mid
    normalized_offset = offset_from_center / mid
    jitter = random.uniform(-ANGLE_JITTER, ANGLE_JITTER)
    return -normalized_offset * ANGLE + jitter


def calculate_card_y_lift(card_index: int, card_amount: int) -> float:
    mid = (card_amount - 1) / 2
    if mid == 0:
        return 0.0

    distance_from_center = abs(card_index - mid)
    normalized_center_lift = 1 - (distance_from_center / mid)
    return normalized_center_lift * MAX_Y_LIFT


def calculate_box_dimensions(img: Image.Image, x_pos: int, y_pos: int, card_id: int) -> list[int | float]:
    card_w, card_h = img.width, img.height
    center_x = round((x_pos + card_w / 2) / HAND_WIDTH, 6)
    center_y = round((y_pos + card_h / 2) / HAND_HEIGHT, 6)
    norm_width = round(card_w / HAND_WIDTH, 6)
    norm_height = round(card_h / HAND_HEIGHT, 6)
    return [card_id, center_x, center_y, norm_width, norm_height]


def y_jitter() -> float:
    return random.uniform(-Y_JITTER, Y_JITTER)