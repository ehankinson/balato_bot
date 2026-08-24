import os
import random
from functools import lru_cache

from PIL import Image

from config.settings import (
    CARD_HEIGHT,
    CARD_WIDTH,
    CONSUMABLE_CANVAS_HEIGHT,
    CONSUMABLE_CANVAS_WIDTH,
    CONSUMABLE_CROP,
    ROOT_DIR,
)
from core.enums import Consumables, Planet, Spectral, Tarot
from core.models import CardAnnotation, RenderedHand
from rendering.backgrounds import render_background
from rendering.card import ENHANCEMENT_LOCATIONS, ENHANCEMENTS
from rendering.layout import calculate_angle, calculate_box_dimensions, y_jitter
from utils.files import load_yaml
from utils.images import card_crop, crop_image, resize_card, yolo_box_to_crop

Consumable = Tarot | Planet | Spectral

CONSUMABLES = Image.open(os.path.join(ROOT_DIR, "game_images", "Tarots.png")).convert(
    "RGBA"
)
CONSUMABLES_LOCATIONS: dict[int, dict[int, dict[str, int]]] = load_yaml(
    os.path.join(ROOT_DIR, "yaml", "consumable_locations.yaml")
)

def crop_consumable(
    location: dict[int, dict[int, dict[str, int]]],
    consumable: Consumable,
) -> Image.Image:
    if isinstance(consumable, Tarot):
        consumable_type = Consumables.TAROT

    elif isinstance(consumable, Planet):
        consumable_type = Consumables.PLANET
    else:
        consumable_type = Consumables.SPECTRAL

    inner_location = location[consumable_type][consumable]
    img = crop_image(
        CONSUMABLES, inner_location["x"], inner_location["y"], CARD_WIDTH, CARD_HEIGHT
    )
    if consumable == Tarot.SOUL:
        soul = crop_image(
            ENHANCEMENTS,
            ENHANCEMENT_LOCATIONS["soul"]["x_pos"],
            ENHANCEMENT_LOCATIONS["soul"]["y_pos"],
            CARD_WIDTH,
            CARD_HEIGHT,
        )
        img.paste(soul, (0, 0), soul)

    return img


@lru_cache(maxsize=None, typed=True)
def render_consumable_cached(consumable: Consumable) -> Image.Image:
    img = crop_consumable(CONSUMABLES_LOCATIONS, consumable)
    return resize_card(img)


def render_consumable(consumable: Consumable) -> Image.Image:
    return render_consumable_cached(consumable).copy()


def render_consumables(
    consumables: list[Consumable] | list[Tarot] | list[Planet] | list[Spectral],
    training: bool = False,
) -> RenderedHand:
    """Render consumables on a background and annotate each complete card."""
    background = render_background(
        CONSUMABLE_CANVAS_WIDTH, CONSUMABLE_CANVAS_HEIGHT, training
    )
    annotations: list[CardAnnotation] = []

    if not consumables:
        return RenderedHand(image=background, annotations=annotations)

    card_width, card_height = render_consumable(consumables[0]).size
    remaining_width = CONSUMABLE_CANVAS_WIDTH - card_width * len(consumables)
    if remaining_width >= 0:
        x_gap = remaining_width / (len(consumables) + 1)
        x_start = x_gap
        x_step = card_width + x_gap
    else:
        x_start = 0
        x_step = (CONSUMABLE_CANVAS_WIDTH - card_width) / max(
            1, len(consumables) - 1
        )

    card_amount = len(consumables)

    for index, consumable in enumerate(consumables):
        consumable_image = render_consumable(consumable)
        angle = calculate_angle(index, card_amount, 1.75)

        x_pos = round(x_start + x_step * index)
        y_pos = round((CONSUMABLE_CANVAS_HEIGHT - card_height) / 2 + y_jitter())

        consumable_image = consumable_image.rotate(angle, expand=True)

        background.paste(consumable_image, (x_pos, y_pos), consumable_image)
        annotations.append(
            CardAnnotation(
                card=consumable,
                box=calculate_box_dimensions(
                    consumable_image,
                    x_pos,
                    y_pos,
                    CONSUMABLE_CANVAS_WIDTH,
                    CONSUMABLE_CANVAS_HEIGHT,
                ),
            )
        )

    return RenderedHand(image=background, annotations=annotations)


def generate_consumables(amount_of_tarots: int, training_type: str):
    consumable_type = Tarot if training_type == "tarot" else Planet if training_type == "planet" else Spectral
    consumables = [random.choice(list(consumable_type)) for _ in range(amount_of_tarots)]
    return render_consumables(consumables, True)


if __name__ == "__main__":
    target = None
    values = []
    for _ in range(4):
        var = random.choice([0, 1, 2])
        if var == 1:
            target = Tarot(random.choice(list(Tarot)))
        elif var == 2:
            target = Planet(random.choice(list(Planet)))
        else:
            target = Spectral(random.choice(list(Spectral)))

        values.append(target)

    data = render_consumables(values)
    img = data.image

    for i, card in enumerate(data.annotations):
        box = card.box
        consu = img.crop(yolo_box_to_crop(box, img))
        w, h = consu.size
        crop = consu.crop(card_crop(w, h, CONSUMABLE_CROP))
        crop.save(f"{i}.png")
        
    data.image.save("img.png")
