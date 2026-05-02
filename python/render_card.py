import os
from functools import lru_cache

from card_enums import Edition, Enhancement, Rank, Seal, Suit
from card_models import Card
from const import (
    CURR_DIR,
    HAND_HEIGHT,
    HAND_HEIGHT_RATION,
    HAND_WIDTH,
    HAND_WIDTH_RATIO,
)
from PIL import Image
from special_enhancemnts import foil_effect, hologram_effect, polychrome_effect
from util import crop_image, load_yaml

CARD_WITH = 142
CARD_HEIGHT = 190
WIDTH_MULT = 1.625
HEIGHT_MULT = 1.625

PLAYING_CARDS = Image.open(
    os.path.join(CURR_DIR, "../game_images/8BitDeck.png")
).convert("RGBA")
ENHANCEMENTS = Image.open(
    os.path.join(CURR_DIR, "../game_images/Enhancers.png")
).convert("RGBA")
CARD_LOCATIONS = load_yaml(os.path.join(CURR_DIR, "../yaml/card_locations.yaml"))
ENHANCEMENT_LOCATIONS = load_yaml(
    os.path.join(CURR_DIR, "../yaml/enhancements_locations.yaml")
)


def get_background(enhancement: Enhancement) -> Image.Image:
    locations = ENHANCEMENT_LOCATIONS["enhancements"][enhancement]
    return crop_image(ENHANCEMENTS, locations["x_pos"], locations["y_pos"], CARD_WITH, CARD_HEIGHT)


def get_card(img: Image.Image, rank: int, suit: int) -> Image.Image:
    y_pos = CARD_LOCATIONS["suits"][suit]
    x_pos = CARD_LOCATIONS["cards"][rank]
    card = crop_image(PLAYING_CARDS, x_pos, y_pos, CARD_WITH, CARD_HEIGHT)
    img.paste(card, (0, 0), card)
    return img


def get_seal(seal: int) -> Image.Image | None:
    location = ENHANCEMENT_LOCATIONS["seals"][seal]
    if location is None:
        return location

    return crop_image(
        ENHANCEMENTS, location["x_pos"], location["y_pos"], CARD_WITH, CARD_HEIGHT
    )


def add_seal(img: Image.Image, seal_value: int) -> Image.Image:
    seal = get_seal(seal_value)
    if seal is not None:
        img.paste(seal, (0, 0), seal)

    return img


def add_edition(img: Image.Image, edition: int) -> Image.Image:
    match edition:
        case 0:
            return img

        case 1:
            return foil_effect(img)

        case 2:
            return hologram_effect(img)

        case 3:
            return polychrome_effect(img)

    raise ValueError(f"This edition {edition} is impossible")


def resize_img(img: Image.Image) -> Image.Image:
    new_width = int(round(HAND_WIDTH * HAND_WIDTH_RATIO, 0))
    new_height = int(round(HAND_HEIGHT * HAND_HEIGHT_RATION, 0))
    return img.resize((new_width, new_height), Image.Resampling.LANCZOS)


@lru_cache(maxsize=None)
def render_card_cached(
    rank: int, suit: int, enhancement: int, seal: int, edition: int
) -> Image.Image:
    img = get_background(Enhancement(enhancement))
    
    if enhancement != Enhancement.STONE:
        img = get_card(img, rank, suit)

    img = add_edition(img, edition)
    img = add_seal(img, seal)
    
    return resize_img(img)


def render_card(card: Card) -> Image.Image:
    return render_card_cached(
        int(card.rank), int(card.suit), int(card.enhancement), int(card.seal), int(card.edition)
    ).copy()


if __name__ == "__main__":
    card = Card.random()
    card.edition = Edition.FOIL
    card_image = render_card(card)
    card_image.save("foil.png")
    