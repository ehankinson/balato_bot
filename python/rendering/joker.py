import os
from functools import lru_cache

from config.settings import CARD_HEIGHT, CARD_WIDTH, ROOT_DIR
from core.enums import Edition, Jokers
from core.models import CardAnnotation, Joker, RenderedHand
from PIL import Image
from utils.files import load_yaml
from utils.images import crop_image, resize_card

from rendering.backgrounds import render_background
from rendering.effects import (
    foil_effect,
    hologram_effect,
    negative_effect,
    polychrome_effect,
)
from rendering.layout import (
    calculate_angle,
    calculate_box_dimensions,
    calculate_card_y_lift,
    y_jitter,
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


@lru_cache(maxsize=None)
def render_joker_cached(
    background_image: int,
    face_image: int | None,
    negative: bool,
    edition: int,
) -> Image.Image:
    img = crop_joker(JOKER_LOCATIONS[background_image])
    if face_image is not None:
        face = crop_joker(JOKER_LOCATIONS[face_image])
        img.paste(face, (0, 0), face)

    if negative:
        img = negative_effect(img)
    elif edition != Edition.NONE:
        match Edition(edition):
            case Edition.FOIL:
                img = foil_effect(img)

            case Edition.POLYCHROME:
                img = polychrome_effect(img)

            case Edition.HOLOGRAPHIC:
                img = hologram_effect(img)
    
    return resize_card(img)


def render_joker(joker: Joker) -> Image.Image:
    return render_joker_cached(
        int(joker.background_image),
        int(joker.face_image) if joker.face_image is not None else None,
        joker.negative,
        int(joker.edition),
    ).copy()


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


def render_jokers(jokers: list[Joker], training: bool = False):
    background = render_background(IMAGE_WIDTH, IMAGE_HEIGHT, training)
    
    card_gap: float = 0.0
    joker_count = len(jokers)
    annotations: list[CardAnnotation] = []
    
    for i, joker in enumerate(jokers):
        joker_image = render_joker(joker)
        image_width = joker_image.width

        angle = calculate_angle(i, joker_count)
        if i == 0:
            card_gap = joker_gap(joker_count, image_width)

        x_pos = WIDTH_PADDING + calculate_x_pos(card_gap, image_width, joker_count, i)
        y_pos = round(calculate_card_y_lift(i, joker_count) + y_jitter())

        joker_image = joker_image.rotate(angle, expand=True)
        background.paste(joker_image, (x_pos, y_pos), joker_image)

        annotations.append(CardAnnotation(
            card=joker,
            box=calculate_box_dimensions(joker_image, x_pos, y_pos, IMAGE_WIDTH, IMAGE_HEIGHT)
        ))

    return RenderedHand(
        image=background,
        annotations=annotations
    )


def main() -> None:
    print(len(list(Jokers)))
    jokers = [Joker.random() for _ in range(8)]
    render_jokers(jokers, True)


if __name__ == '__main__':
    main()
