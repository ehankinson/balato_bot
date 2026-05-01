import os

from card_enums import Jokers
from const import ROOT_DIR
from PIL import Image
from util import crop_image, load_yaml

JOKERS = Image.open(os.path.join(ROOT_DIR, "game_images/Jokers.png")).convert("RGBA")
JOKER_LOCATIONS = load_yaml(os.path.join(ROOT_DIR, "yaml/joker_locations.yaml"))
JOKER_WITH = 142
JOKER_HEIGHT = 190

a = JOKER_LOCATIONS[Jokers.BARON]
blue_print = crop_image(JOKERS, a["x_pos"], a["y_pos"], JOKER_WITH, JOKER_HEIGHT).convert("RGBA")
