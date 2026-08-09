import os
import random
from functools import lru_cache

from PIL import Image

from config.settings import CARD_HEIGHT, CARD_WIDTH, ROOT_DIR
from core.enums import Consumables, Planet, Spectral, Tarot
from core.models import CardAnnotation, RenderedHand
from rendering.backgrounds import render_background
from rendering.layout import calculate_angle, calculate_box_dimensions, y_jitter
from utils.files import load_yaml
from utils.images import crop_image, resize_card

Consumable = Tarot | Planet | Spectral

CONSUMABLES = Image.open(os.path.join(ROOT_DIR, "game_images", "Tarots.png")).convert(
    "RGBA"
)
CONSUMABLES_LOCATIONS: dict[int, dict[int, dict[str, int]]] = load_yaml(
    os.path.join(ROOT_DIR, "yaml", "consumable_locations.yaml")
)

IMAGE_WIDTH = 535
IMAGE_HEIGHT = 310


def crop_consumable(
    location: dict[int, dict[int, dict[str, int]]],
    consumable: Consumable,
) -> Image.Image:
    is_soul = False
    if isinstance(consumable, Tarot):
        consumable_type = Consumables.TAROT
        is_soul = consumable == Tarot.SOUL

    elif isinstance(consumable, Planet):
        consumable_type = Consumables.PLANET
    else:
        consumable_type = Consumables.SPECTRAL

    inner_location = location[consumable_type][consumable]
    return crop_image(
        CONSUMABLES, inner_location["x"], inner_location["y"], CARD_WIDTH, CARD_HEIGHT
    )


@lru_cache(maxsize=None, typed=True)
def render_consumable_cached(consumable: Consumable) -> Image.Image:
    img = crop_consumable(CONSUMABLES_LOCATIONS, consumable)
    return resize_card(img)


def render_consumable(consumable: Consumable) -> Image.Image:
    return render_consumable_cached(consumable).copy()


def render_consumables(
    consumables: list[Consumable], training: bool = False
) -> RenderedHand:
    """Render consumables on a background and annotate each complete card."""
    background = render_background(IMAGE_WIDTH, IMAGE_HEIGHT, training)
    annotations: list[CardAnnotation] = []

    if not consumables:
        return RenderedHand(image=background, annotations=annotations)

    card_width, card_height = render_consumable(consumables[0]).size
    remaining_width = IMAGE_WIDTH - card_width * len(consumables)
    if remaining_width >= 0:
        x_gap = remaining_width / (len(consumables) + 1)
        x_start = x_gap
        x_step = card_width + x_gap
    else:
        x_start = 0
        x_step = (IMAGE_WIDTH - card_width) / max(1, len(consumables) - 1)

    card_amount = len(consumables)

    for index, consumable in enumerate(consumables):
        consumable_image = render_consumable(consumable)
        angle = calculate_angle(index, card_amount, 1.75)

        x_pos = round(x_start + x_step * index)
        y_pos = round((IMAGE_HEIGHT - card_height) / 2 + y_jitter())

        consumable_image = consumable_image.rotate(angle, expand=True)

        background.paste(consumable_image, (x_pos, y_pos), consumable_image)
        annotations.append(
            CardAnnotation(
                card=consumable,
                box=calculate_box_dimensions(
                    consumable_image, x_pos, y_pos, IMAGE_WIDTH, IMAGE_HEIGHT
                ),
            )
        )

    return RenderedHand(image=background, annotations=annotations)


if __name__ == "__main__":
    target = None
    values = []
    for _ in range(random.randint(1, 5)):
        var = random.choice([0, 1, 2])
        if var == 1:
            target = Tarot(random.choice(list(Tarot)))
        elif var == 2:
            target = Planet(random.choice(list(Planet)))
        else:
            target = Spectral(random.choice(list(Spectral)))

        values.append(target)

    img = render_consumables(values)
    img.image.save("img.png")
