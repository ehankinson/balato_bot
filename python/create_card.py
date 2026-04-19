import os
from turtle import back

from PIL import Image
from util import load_yaml
from const import Card, Suit, Rank, Seal, Enhancement, CURR_DIR

CARD_WITH = 142
CARD_HEIGHT = 190

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



def create_card(card: Card) -> Card:
    background = get_background(card)
    if card.enhancement == Enhancement.STONE:
        card.image = background
        return card

    card_image = get_card(card)

    background.paste(card_image, (0, 0), card_image)

    seal = get_seal(card)
    if seal is not None:
        background.paste(seal, (0, 0), seal)

    card.image = background

    return card



if __name__ == '__main__':
    test_card = Card(
        Rank.KING,
        Suit.SPADES,
        Enhancement.GLASS,
        Seal.RED
    )
    create_card(test_card)
