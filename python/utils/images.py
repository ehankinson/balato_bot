from PIL import Image

from config.settings import (
    RENDERED_CARD_HEIGHT,
    RENDERED_CARD_WIDTH,
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
    return img.resize(
        (RENDERED_CARD_WIDTH, RENDERED_CARD_HEIGHT), Image.Resampling.LANCZOS
    )


def yolo_box_to_crop(box: list[float], image: Image.Image) -> tuple[int, int, int, int]:
    _, center_x, center_y, width, height = box
    image_width, image_height = image.size

    box_width = width * image_width
    box_height = height * image_height
    left = round(center_x * image_width - box_width / 2)
    top = round(center_y * image_height - box_height / 2)
    right = round(center_x * image_width + box_width / 2)
    bottom = round(center_y * image_height + box_height / 2)

    left = max(0, min(left, image_width - 1))
    top = max(0, min(top, image_height - 1))
    right = max(left + 1, min(right, image_width))
    bottom = max(top + 1, min(bottom, image_height))

    return left, top, right, bottom
