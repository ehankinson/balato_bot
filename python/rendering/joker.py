import os

from config.settings import CARD_HEIGHT, CARD_WIDTH, ROOT_DIR
from core.enums import Jokers
from core.models import Joker
from PIL import Image
from utils.files import load_yaml
from utils.images import crop_image, resize_card

from rendering.backgrounds import render_background
from rendering.layout import (
    calculate_box_dimensions,
    calculate_card_angle,
    calculate_card_y_lift,
)

JOKERS = Image.open(os.path.join(ROOT_DIR, "game_images/Jokers.png")).convert("RGBA")
JOKER_LOCATIONS: dict[int, dict[str, int]] = load_yaml(os.path.join(ROOT_DIR, "yaml/joker_locations.yaml"))


IMAGE_WIDTH = 1150
IMAGE_HEIGHT = 350
WIDTH_PADDING = 18
JOKERS_WIDTH = 1120
JOKERS_HEIGHT = 220


def crop_joker(location: dict[str, int]) -> Image.Image:
    return crop_image(JOKERS, location["x_pos"], location["y_pos"], CARD_WIDTH, CARD_HEIGHT)


def render_joker(joker: Joker) -> Image.Image:
    img = crop_joker(JOKER_LOCATIONS[joker.background_image])
    if joker.face_image is not None:
        face = crop_joker(JOKER_LOCATIONS[joker.face_image])
        img.paste(face, (0, 0), face)
    
    return resize_card(img)


def joker_gap(card_amount: int, card_width: int) -> float:
    total_card_space = card_amount * card_width
    shift_space = JOKERS_WIDTH - total_card_space
    return shift_space // (card_amount + 1) \
        if card_amount <= 2 \
            else shift_space / (card_amount - 1)


def calculate_x_pos(card_gap: float, image_width: int, card_amount: int, card_index: int) -> int:
    return int((card_index + 1) * card_gap + image_width * card_index) \
        if card_amount <= 2 \
            else int(card_index * (image_width + card_gap)) 


def render_jokers(jokers: list[Joker]):
    card_amount = len(jokers)
    background = render_background(IMAGE_WIDTH, IMAGE_HEIGHT)
    card_gap: float = 0.0
    for i, joker in enumerate(jokers):
        joker_image = render_joker(joker)
        image_width = joker_image.width
        if i == 0:
            card_gap = joker_gap(card_amount, image_width)

        x_pos = WIDTH_PADDING + calculate_x_pos(card_gap, image_width, card_amount, i)
        background.paste(joker_image, (x_pos, 20), joker_image)
        background.save(os.path.join(ROOT_DIR, "image.png"))


def main() -> None:
    jokers = [
        Joker(Jokers.TROUBADOUR),
        Joker(Jokers.BLUEPRINT),
        Joker(Jokers.BARON),
        Joker(Jokers.MIME),
        Joker(Jokers.DRUNKARD),
        Joker(Jokers.BRAIN_STORM)
    ]
    render_jokers(jokers)


if __name__ == '__main__':
    main()
