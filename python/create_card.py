import os
import random
from PIL import Image

from util import join_path, load_json
from const import (
    MARGIN,
    SPACING,
    ENHANCERS,
    TILE_WIDTH,
    TILE_HEIGHT,
    PLAYING_CARDS,
    IMAGE_OUTPUT_DIR,
    VALID_CARD_BACKGROUNDS,
)

CURR_DIR = os.path.dirname(os.path.abspath(__file__))

SUITS = ["hearts", "diamonds", "clubs", "spades"]
RANKS = [2, 3, 4, 5, 6, 7, 8, 9, 10, "J", "Q", "K", "A"]


def crop_tiles(img_path: str):
    """Crop a spritesheet image into individual tiles and save them.
    
    Args:
        img_path: Relative path to the spritesheet image file.
    """
    if not os.path.exists(IMAGE_OUTPUT_DIR):
        os.makedirs(IMAGE_OUTPUT_DIR, exist_ok=True)

    abs_path = join_path(CURR_DIR, f"../{img_path}")
    img = Image.open(abs_path).convert("RGBA")
    img_w, img_h = img.size

    tile_id = 0
    for y in range(MARGIN, img_h - TILE_HEIGHT + 1, TILE_HEIGHT + SPACING):
        for x in range(MARGIN, img_w - TILE_WIDTH + 1, TILE_WIDTH + SPACING):
            tile = img.crop((x, y, x + TILE_WIDTH, y + TILE_HEIGHT))
            tile.save(f"{IMAGE_OUTPUT_DIR}/tile_{tile_id:03d}.png")
            tile_id += 1
            # print(f"'{RANKS[x // TILE_WIDTH]}-{SUITS[y // TILE_HEIGHT]}': ['x': {x}, 'y': {y}],")
            print(f"{tile_id}: ['x': {x}, 'y': {y}]")



def crop_tile(img_path: str, x: int, y: int) -> Image.Image:
    """Crop a tile from an image and return it as a PIL Image object."""
    image = Image.open(img_path).convert("RGBA")
    return image.crop((x, y, x + TILE_WIDTH, y + TILE_HEIGHT))





def build_card(card_type: str, background: str) -> None:
    """Build a card image from a card type and background."""
    if not os.path.exists(IMAGE_OUTPUT_DIR):
        os.makedirs(IMAGE_OUTPUT_DIR, exist_ok=True)

    card_positions = load_json(join_path(CURR_DIR, "../json/card_positions.json"))
    background_positions = load_json(join_path(CURR_DIR, "../json/enhancements.json"))

    background_position = background_positions[background]
    back_x, back_y = background_position["x"], background_position["y"]
    card_position = card_positions[card_type]
    card_x, card_y = card_position["x"], card_position["y"]

    background_tile = crop_tile(join_path(CURR_DIR, f"../{ENHANCERS}"), back_x, back_y)
    card_tile = crop_tile(join_path(CURR_DIR, f"../{PLAYING_CARDS}"), card_x, card_y)

    final_image = Image.alpha_composite(background_tile, card_tile)
    final_image.save(f"{IMAGE_OUTPUT_DIR}/card_{card_type}_{background}.png")



cards = ['2-hearts', '3-hearts', '4-hearts', '5-hearts', '6-hearts', '7-hearts', '8-hearts', '9-hearts', '10-hearts', 'J-hearts', 'Q-hearts', 'K-hearts', 'A-hearts', '2-diamonds', '3-diamonds', '4-diamonds', '5-diamonds', '6-diamonds', '7-diamonds', '8-diamonds', '9-diamonds', '10-diamonds', 'J-diamonds', 'Q-diamonds', 'K-diamonds', 'A-diamonds', '2-clubs', '3-clubs', '4-clubs', '5-clubs', '6-clubs', '7-clubs', '8-clubs', '9-clubs', '10-clubs', 'J-clubs', 'Q-clubs', 'K-clubs', 'A-clubs', '2-spades', '3-spades', '4-spades', '5-spades', '6-spades', '7-spades', '8-spades', '9-spades', '10-spades', 'J-spades', 'Q-spades', 'K-spades', 'A-spades']


for i in range(6):
    card_type = random.choice(cards)
    background = random.choice(VALID_CARD_BACKGROUNDS)
    build_card(card_type, background)




# crop_tile(ENHANCERS)
