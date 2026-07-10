from PIL import Image

from best_hand import get_best_scoring_hand
from config.model_registry import CARD_BOX_MODEL
from config.settings import (
    EDITION_CROP,
    ENHANCEMENT_CROP,
    RANK_CROP,
    SEAL_CROP,
    SUIT_CROP,
)
from core.enums import Edition, Enhancement, Rank, Seal, Suit
from core.models import Card, CardData, FinalScoringResults, GameState
from model import preload_models, run_model
from utils.images import card_crop


def get_card_locations(img: Image.Image):
    results = CARD_BOX_MODEL(img)
    for result in results:
        boxes = result.boxes

        for box in boxes:
            xyxy = box.xyxy[0]  # x1, y1, x2, y2
            confidence = box.conf[0]
            class_id = box.cls[0]

            print("box:", xyxy)
            print("confidence:", confidence)
            print("class:", class_id)


def get_card_information(card_images: list[Image.Image]) -> list[Card]:
    cards: list[Card] = []

    feature_map = {
        "rank": {"values": [], "crop": RANK_CROP},
        "seal": {"values": [], "crop": SEAL_CROP},
        "suit": {"values": [], "crop": SUIT_CROP},
        "edition": {"values": [], "crop": EDITION_CROP},
        "enhancement": {"values": [], "crop": ENHANCEMENT_CROP},
    }

    for card_image in card_images:
        w, h = card_image.size
        for feature in feature_map:
            feature_map[feature]["values"].append(
                card_image.crop(card_crop(w, h, feature_map[feature]["crop"]))
            )

    for feature in feature_map:
        outputs = run_model(feature, feature_map[feature]["values"])
        feature_map[feature]["values"] = outputs

    for i in range(len(card_images)):
        cards.append(
            Card(
                rank=Rank(feature_map["rank"]["values"][i]),
                suit=Suit(feature_map["suit"]["values"][i]),
                enhancement=Enhancement(feature_map["enhancement"]["values"][i]),
                seal=Seal(feature_map["seal"]["values"][i]),
                edition=Edition(feature_map["edition"]["values"][i]),
            )
        )

    return cards


def get_played_hand(
    img: Image.Image,
) -> tuple[list[CardData], list[CardData], list[CardData]]:
    preload_models()
    game_state = GameState()

    card_locations = []
    results = CARD_BOX_MODEL(img, verbose=False)

    for res in results:
        for i, box in enumerate(res.boxes):
            if float(box.conf) < 0.9:
                continue

            location = [float(val) for val in box.xyxy[0]]
            card_locations.append(location)

    card_images = []
    for i, location in enumerate(card_locations):
        card_image = img.crop(location)
        card_images.append(card_image)

    cards = get_card_information(card_images)

    best_hand = get_best_scoring_hand(cards, [], game_state)

    scored_played = []
    for card in best_hand.hand_scoring.scored_played:
        index = cards.index(card)
        card_location = card_locations[index]
        scored_played.append(CardData(card=card, location=card_location))

    unscored_played = []
    for card in best_hand.hand_scoring.unscored_played:
        index = cards.index(card)
        card_location = card_locations[index]
        unscored_played.append(CardData(card=card, location=card_location))

    scored_held = []
    for card in best_hand.hand_scoring.scored_held:
        index = cards.index(card)
        card_location = card_locations[index]
        scored_held.append(CardData(card=card, location=card_location))

    return scored_played, unscored_played, scored_held


if __name__ == "__main__":
    img = Image.open("/home/hank/projects/balatro_bot/hand_0.png").convert("RGB")
    get_played_hand(img)
