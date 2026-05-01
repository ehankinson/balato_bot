import os

from card_enums import Jokers
from const import ROOT_DIR
from util import crop_image, load_yaml
from PIL import Image, ImageOps, ImageEnhance


JOKERS = Image.open(os.path.join(ROOT_DIR, "game_images/Jokers.png")).convert("RGBA")
JOKER_LOCATIONS = load_yaml(os.path.join(ROOT_DIR, "yaml/joker_locations.yaml"))
JOKER_WITH = 142
JOKER_HEIGHT = 190

a = JOKER_LOCATIONS[Jokers.GOLDEN_TICKET]
blue_print = crop_image(JOKERS, a["x_pos"], a["y_pos"], JOKER_WITH, JOKER_HEIGHT).convert("RGBA")
r, g, b, a = blue_print.split()

# Convert to grayscale brightness
gray = ImageOps.grayscale(blue_print)

# Boost contrast
gray = ImageEnhance.Contrast(gray).enhance(1.8)

# Optional: invert brightness
gray = ImageOps.invert(gray)

# Create a custom "negative joker" palette
# dark -> mid -> bright
colored = ImageOps.colorize(
    gray,
    black="#2b214f",   # shadows: purple/navy
    mid="#4bb7e8",     # mids: cyan/blue
    white="#ffe36e"    # highlights: yellow/gold
)

# Reattach alpha
colored.putalpha(a)

colored.save("balatro_negative_style.png")
blue_print.save("blueprint.png")

