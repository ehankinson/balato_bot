from config.model_registry import LOCATION_MODEL
from config.settings import (
    EDITION_CROP,
    ENHANCEMENT_CROP,
    RANK_CROP,
    SEAL_CROP,
    SUIT_CROP,
)
from core.enums import Edition, Enhancement, Rank, Seal, Suit
from core.models import Card, CardData
from model import run_model
from PIL import Image
from utils.images import card_crop


def get_card_locations(img: Image.Image, save_path: str | None = None):
    results = LOCATION_MODEL(img)

    for result in results:
        boxes = result.boxes

        for box in boxes:
            xyxy = box.xyxy[0]  # x1, y1, x2, y2
            confidence = box.conf[0]
            class_id = box.cls[0]

            print("box:", xyxy)
            print("confidence:", confidence)
            print("class:", class_id)

        if save_path is not None:
            plotted = Image.fromarray(result.plot()[..., ::-1])
            plotted.save(save_path)
            print(f"Saved annotations to {save_path}")


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

    for feature, properties in feature_map.items():
        properties["values"] = run_model(feature, properties["values"])

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


def get_selected_cards(img, command, observation):
    """Map a completed Rust-policy card command onto detected screen positions.

    The caller supplies the synchronized full-game observation; a screenshot of
    the hand alone cannot supply the policy's inventory, deck, or run context.
    """
    if command.kind not in (2, 3) or command.targets is None:
        raise ValueError("screen card selection requires a play/discard command")
    locations = []
    for result in LOCATION_MODEL(img, verbose=False):
        for box in result.boxes:
            if float(box.conf) >= 0.9 and int(box.cls) == 0:
                locations.append([float(value) for value in box.xyxy[0]])
    locations.sort(key=lambda location: location[0])
    if len(locations) != len(observation["hand.present"]):
        raise ValueError("detected hand does not match the supplied Rust observation")
    if any(index < 0 or index >= len(locations) for index in command.targets):
        raise ValueError("selected card is outside the detected hand")
    cards = get_card_information([img.crop(location) for location in locations])
    return [
        CardData(card=cards[index], location=locations[index])
        for index in command.targets
    ]
