import os
from functools import lru_cache

from PIL import Image

from config.settings import (
    CARD_HEIGHT,
    CARD_WIDTH,
    JOKER_CANVAS_HEIGHT,
    JOKER_CANVAS_WIDTH,
    JOKER_CONTENT_WIDTH,
    JOKER_X_PADDING,
    ROOT_DIR,
)
from core.enums import JokerEdition, JokersName
from core.models import CardAnnotation, Joker, RenderedHand
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
from utils.files import load_yaml
from utils.images import crop_image, resize_card

JOKERS = Image.open(os.path.join(ROOT_DIR, "game_images/Jokers.png")).convert("RGBA")
JOKER_LOCATIONS: dict[int, dict[str, int]] = load_yaml(
    os.path.join(ROOT_DIR, "yaml/joker_locations.yaml")
)


def crop_joker(location: dict[str, int]) -> Image.Image:
    return crop_image(
        JOKERS, location["x_pos"], location["y_pos"], CARD_WIDTH, CARD_HEIGHT
    )


@lru_cache(maxsize=None)
def render_joker_cached(
    background_image: int,
    face_image: int | None,
    edition: int,
) -> Image.Image:
    img = crop_joker(JOKER_LOCATIONS[background_image])
    if face_image is not None:
        face = crop_joker(JOKER_LOCATIONS[face_image])
        img.paste(face, (0, 0), face)

    match edition:
        case JokerEdition.FOIL:
            img = foil_effect(img)

        case JokerEdition.POLYCHROME:
            img = polychrome_effect(img)

        case JokerEdition.HOLOGRAPHIC:
            img = hologram_effect(img)

        case JokerEdition.NEGATIVE:
            img = negative_effect(img)

        case _:
            img = img

    return resize_card(img)


def render_joker(joker: Joker) -> Image.Image:
    return render_joker_cached(
        int(joker.joker_name),
        int(joker.face_image) if joker.face_image is not None else None,
        int(joker.joker_edition),
    ).copy()


def joker_gap(card_amount: int, card_width: int) -> float:
    total_card_space = card_amount * card_width
    shift_space = JOKER_CONTENT_WIDTH - total_card_space
    return (
        shift_space // (card_amount + 1)
        if card_amount <= 2
        else shift_space / (card_amount - 1)
    )


def calculate_x_pos(
    card_gap: float, image_width: int, card_amount: int, card_index: int
) -> int:
    return (
        int((card_index + 1) * card_gap + image_width * card_index)
        if card_amount <= 2
        else int(card_index * (image_width + card_gap))
    )


def render_jokers(jokers: list[Joker], training: bool = False, training_type: str = ""):
    background = render_background(JOKER_CANVAS_WIDTH, JOKER_CANVAS_HEIGHT, training)

    card_gap: float = 0.0
    joker_count = len(jokers)
    annotations: list[CardAnnotation] = []

    for i, joker in enumerate(jokers):
        joker_image = render_joker(joker)
        image_width = joker_image.width

        angle = calculate_angle(i, joker_count)
        if i == 0:
            card_gap = joker_gap(joker_count, image_width)

        x_pos = JOKER_X_PADDING + calculate_x_pos(
            card_gap, image_width, joker_count, i
        )
        y_pos = round(calculate_card_y_lift(i, joker_count) + y_jitter())

        joker_image = joker_image.rotate(angle, expand=True)
        background.paste(joker_image, (x_pos, y_pos), joker_image)

        annotations.append(
            CardAnnotation(
                card=getattr(joker, training_type),
                box=calculate_box_dimensions(
                    joker_image,
                    x_pos,
                    y_pos,
                    JOKER_CANVAS_WIDTH,
                    JOKER_CANVAS_HEIGHT,
                ),
            )
        )

    return RenderedHand(image=background, annotations=annotations)

def generate_jokers(amount: int, training_type: str):
    jokers = [Joker.random() for _ in range(amount)]
    return render_jokers(jokers, True, training_type)
    


if __name__ == "__main__":
    import random
    joker_amount = random.randint(1, 9)
    data = generate_jokers(joker_amount, "joker_name")
    data.image.save("tmp.png")
    #
    # joker = Joker.build(JokersName.CANIO_BACKGROUND)

    # img = render_joker(joker)

    # img.save("img.png")
