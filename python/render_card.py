import os

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




def crop_image(image: Image, x_pos: int, y_pos: int) -> Image:
    return image.crop((
        x_pos, y_pos, x_pos + CARD_WITH, y_pos + CARD_HEIGHT
    ))



def get_background(card: Card) -> Image:
    locations = ENHANCEMENT_LOCATIONS['enhancements'][card.enhancement]
    return crop_image(ENHANCEMENTS, locations["x_pos"], locations["y_pos"])



def get_card(card: Card) -> Image:
    y_pos = CARD_LOCATIONS["suits"][card.suit]
    x_pos = CARD_LOCATIONS["cards"][card.rank]
    return crop_image(PLAYING_CARDS, x_pos, y_pos)



def get_seal(card: Card) -> Image | None:
    location = ENHANCEMENT_LOCATIONS['seals'][card.seal]
    if location is None:
        return location

    return crop_image(ENHANCEMENTS, location["x_pos"], location["y_pos"])



def add_seal(img: Image, card: Card) -> Image:
    seal = get_seal(card)
    if seal is not None:
        img.paste(seal, (0, 0), seal)

    return img



def resize_img(img: Image) -> Image:
    new_width = int(round(HAND_WIDTH * HAND_WIDTH_RATIO, 0))
    new_height = int(round(HAND_HEIGHT * HAND_HEIGHT_RATION, 0))
    return img.resize(
        (new_width, new_height),
        Image.Resampling.LANCZOS
    )



def render_card(card: Card) -> Image:
    img = get_background(card)
    if card.enhancement == Enhancement.STONE:
        img = add_seal(img, card)
        return resize_img(img)

    card_image = get_card(card)

    img.paste(card_image, (0, 0), card_image)
    img = add_seal(img, card)

    return resize_img(img)
