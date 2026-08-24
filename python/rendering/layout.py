import random

from config.settings import (
    BOX_ID,
    LAYOUT_ANGLE,
    LAYOUT_ANGLE_JITTER,
    LAYOUT_MAX_Y_LIFT,
    LAYOUT_Y_JITTER,
)
from PIL import Image

def calculate_angle(card_index: int, card_amount: int, custom_angle: float | None = None) -> float:
    angle = custom_angle if custom_angle is not None else LAYOUT_ANGLE
    mid = (card_amount - 1) / 2
    if mid == 0:
        return 0.0

    offset_from_center = card_index - mid
    normalized_offset = offset_from_center / mid
    jitter = random.uniform(-LAYOUT_ANGLE_JITTER, LAYOUT_ANGLE_JITTER)
    return -normalized_offset * angle + jitter


def calculate_card_y_lift(card_index: int, card_amount: int) -> float:
    mid = (card_amount - 1) / 2
    if mid == 0:
        return 0.0

    distance_from_center = abs(card_index - mid)
    normalized_center_lift = 1 - (distance_from_center / mid)
    return normalized_center_lift * LAYOUT_MAX_Y_LIFT


def calculate_box_dimensions(
    img: Image.Image,
    x_pos: int,
    y_pos: int,
    canvas_width: int,
    canvas_height: int,
) -> list[int | float]:
    card_w, card_h = img.width, img.height
    center_x = round((x_pos + card_w / 2) / canvas_width, 6)
    center_y = round((y_pos + card_h / 2) / canvas_height, 6)
    norm_width = round(card_w / canvas_width, 6)
    norm_height = round(card_h / canvas_height, 6)
    return [BOX_ID, center_x, center_y, norm_width, norm_height]


def y_jitter() -> float:
    return random.uniform(-LAYOUT_Y_JITTER, LAYOUT_Y_JITTER)
