import os
import csv
import random
from concurrent.futures import ProcessPoolExecutor, as_completed
from multiprocessing import cpu_count
from PIL import Image, ImageEnhance, ImageFilter

from util import join_path, load_json
from const import (
    MARGIN,
    SPACING,
    ENHANCERS,
    TILE_WIDTH,
    TILE_HEIGHT,
    PLAYING_CARDS,
    IMAGE_OUTPUT_DIR,
    TRAINING_DATA_DIR,
    # VALID_CARD_BACKGROUNDS,
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



def build_card(card_type: str, background: str, save: bool = False) -> Image.Image:
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
    if save:
        final_image.save(f"{IMAGE_OUTPUT_DIR}/card_{card_type}_{background}.png")

    return final_image



def random_affine(img: Image.Image) -> Image.Image:
    """Apply a random affine transformation to an image."""
    w, h = img.size

    angle = random.uniform(-8.0, 8.0)
    scale = random.uniform(0.75, 1.25)
    dx = random.randint(-5, 5)
    dy = random.randint(-5, 5)

    # Pillow >= 10 uses Image.Resampling.*; older versions may still expose Image.BICUBIC.
    resample_bicubic = getattr(getattr(Image, "Resampling", Image), "BICUBIC")

    # scale
    new_w, new_h = int(w * scale), int(h * scale)
    img = img.resize((new_w, new_h), resample_bicubic)

    # rotate
    img = img.rotate(angle, resample=resample_bicubic, expand=True)

    # paste back to original canvas
    canvas = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    x = (w - img.width) // 2 + dx
    y = (h - img.height) // 2 + dy
    canvas.paste(img, (x, y), img)

    return canvas



def random_color_and_blur(img: Image.Image) -> Image.Image:
    """Apply a random color and blur to an image."""
    # Brightness
    if random.random() < 0.8:
        img = ImageEnhance.Brightness(img).enhance(random.uniform(0.75, 1.25))

    # Contrast
    if random.random() < 0.8:
        img = ImageEnhance.Contrast(img).enhance(random.uniform(0.75, 1.25))

    # Blur
    if random.random() < 0.3:
        img = img.filter(ImageFilter.GaussianBlur(radius=random.uniform(0.5, 1.5)))

    return img



def _process_single_card(args: tuple) -> tuple:
    """Worker function to process a single card image.

    Args:
        args: Tuple of (card, index, training_data_dir)

    Returns:
        Tuple of CSV row data (filename, rank, suit, enhancement, seal, edition)
    """
    card, idx, training_data_dir = args

    img = build_card(card, "blank_card")
    img = random_affine(img)
    img = random_color_and_blur(img)

    filename = f"{card}_{idx}.png"
    img.save(f"{training_data_dir}/images/{filename}")

    split_card = card.split("-")
    return (filename, split_card[0], split_card[1], "blank_card", "None", "None")


def build_training_data(amount_per_card: int, workers: int | None = None) -> None:
    """Build training data for a card using multiprocessing.

    Args:
        amount_per_card: Number of augmented images to generate per card type.
        workers: Number of worker processes. Defaults to CPU count.
    """
    if not os.path.exists(TRAINING_DATA_DIR):
        os.makedirs(TRAINING_DATA_DIR, exist_ok=True)

    if not os.path.exists(f"{TRAINING_DATA_DIR}/images"):
        os.makedirs(f"{TRAINING_DATA_DIR}/images", exist_ok=True)

    card_positions: dict[str, dict[str, int]] = load_json(
        join_path(CURR_DIR, "../json/card_positions.json")
    )
    cards: list[str] = list(card_positions.keys())

    # Build list of all tasks: (card_name, index, output_dir)
    tasks = [
        (card, idx, TRAINING_DATA_DIR)
        for card in cards
        for idx in range(amount_per_card)
    ]

    total_tasks = len(tasks)
    num_workers = workers if workers else cpu_count()
    print(f"Processing {total_tasks} images with {num_workers} workers...")

    results = []
    completed = 0

    with ProcessPoolExecutor(max_workers=num_workers) as executor:
        futures = {executor.submit(_process_single_card, task): task for task in tasks}

        for future in as_completed(futures):
            results.append(future.result())
            completed += 1
            if completed % 1000 == 0 or completed == total_tasks:
                print(f"Progress: {completed}/{total_tasks} ({100*completed/total_tasks:.1f}%)")

    # Write CSV after all processing is done
    with open(f"{TRAINING_DATA_DIR}/labels.csv", "w", encoding="utf-8") as csv_file:
        writer = csv.writer(csv_file)
        writer.writerow(["filename", "rank", "suit", "enhancement", "seal", "edition"])
        writer.writerows(results)

    print(f"Done! Generated {total_tasks} images.")


if __name__ == "__main__":
    build_training_data(1000)
    # crop_tile(ENHANCERS)
