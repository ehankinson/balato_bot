from PIL import Image

from config.settings import (
    HAND_HEIGHT,
    HAND_HEIGHT_RATION,
    HAND_WIDTH,
    HAND_WIDTH_RATIO,
)


def card_crop(width: int, height: int, crop_values: list[float]) -> tuple[int, int, int, int]:
    left = int(crop_values[0])
    top = int(crop_values[1])
    right = int(width * crop_values[2])
    bottom = int(height * crop_values[3])

    left = max(0, min(left, width - 1))
    top = max(0, min(top, height - 1))
    right = max(left + 1, min(right, width))
    bottom = max(top + 1, min(bottom, height))

    return left, top, right, bottom


def crop_image(image: Image.Image, x_pos: int, y_pos: int, width: int, height: int) -> Image.Image:
    return image.crop((x_pos, y_pos, x_pos + width, y_pos + height))


def resize_card(img: Image.Image) -> Image.Image:
    new_width = int(round(HAND_WIDTH * HAND_WIDTH_RATIO, 0))
    new_height = int(round(HAND_HEIGHT * HAND_HEIGHT_RATION, 0))
    return img.resize((new_width, new_height), Image.Resampling.LANCZOS)

