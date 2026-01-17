import os
from PIL import Image

from const import *
from util import join_path

CURR_DIR = os.path.dirname(os.path.abspath(__file__))

SUITS = ["hearts", "diamonds", "clubs", "spades"]
RANKS = [2, 3, 4, 5, 6, 7, 8, 9, 10, "J", "Q", "K", "A"]


def crop_tile(img_path: str):
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


crop_tile(ENHANCERS)
