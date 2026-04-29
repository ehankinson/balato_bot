import os
from functools import lru_cache

from PIL import Image
from util import load_yaml
from card_models import Card
from card_enums import Enhancement
from const import CURR_DIR, HAND_WIDTH, HAND_HEIGHT, HAND_WIDTH_RATIO, HAND_HEIGHT_RATION


CARD_WITH = 142
CARD_HEIGHT = 190
WIDTH_MULT = 1.625
HEIGHT_MULT = 1.625

PLAYING_CARDS = Image.open(os.path.join(CURR_DIR, "../game_images/8BitDeck.png")).convert("RGBA")
ENHANCEMENTS = Image.open(os.path.join(CURR_DIR, "../game_images/Enhancers.png")).convert("RGBA")
CARD_LOCATIONS = load_yaml(os.path.join(CURR_DIR, "../yaml/card_locations.yaml"))
ENHANCEMENT_LOCATIONS = load_yaml(os.path.join(CURR_DIR, "../yaml/enhancements_locations.yaml"))




def crop_image(image: Image.Image, x_pos: int, y_pos: int) -> Image:
    return image.crop((
        x_pos, y_pos, x_pos + CARD_WITH, y_pos + CARD_HEIGHT
    ))



def get_background(enhancement: Enhancement) -> Image:
    locations = ENHANCEMENT_LOCATIONS['enhancements'][enhancement]
    return crop_image(ENHANCEMENTS, locations["x_pos"], locations["y_pos"])



def get_card(rank: int, suit: int) -> Image:
    y_pos = CARD_LOCATIONS["suits"][suit]
    x_pos = CARD_LOCATIONS["cards"][rank]
    return crop_image(PLAYING_CARDS, x_pos, y_pos)



def get_seal(seal: int) -> Image | None:
    location = ENHANCEMENT_LOCATIONS['seals'][seal]
    if location is None:
        return location

    return crop_image(ENHANCEMENTS, location["x_pos"], location["y_pos"])



def add_seal(img: Image.Image, seal_value: int) -> Image:
    seal = get_seal(seal_value)
    if seal is not None:
        img.paste(seal, (0, 0), seal)

    return img



def resize_img(img: Image.Image) -> Image:
    new_width = int(round(HAND_WIDTH * HAND_WIDTH_RATIO, 0))
    new_height = int(round(HAND_HEIGHT * HAND_HEIGHT_RATION, 0))
    return img.resize(
        (new_width, new_height),
        Image.Resampling.LANCZOS
    )



@lru_cache(maxsize=None)
def render_card_cached(rank: int, suit: int, enhancement: int, seal: int) -> Image:
    img = get_background(Enhancement(enhancement))
    if enhancement == Enhancement.STONE:
        img = add_seal(img, seal)
        return resize_img(img)

    card_image = get_card(rank, suit)

    img.paste(card_image, (0, 0), card_image)
    img = add_seal(img, seal)

    return resize_img(img)



def render_card(card: Card) -> Image:
    return render_card_cached(
        int(card.rank),
        int(card.suit),
        int(card.enhancement),
        int(card.seal)
    ).copy()
