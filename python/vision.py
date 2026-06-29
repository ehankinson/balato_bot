import time

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
from model import load_model, run_model
from utils.images import card_crop


def pretty_time(seconds: float) -> str:
    if seconds < 0.001:
        return f"{seconds * 1_000_000:.0f}us"

    if seconds < 1:
        return f"{seconds * 1_000:.0f}ms"

    return f"{seconds:.2f}s"


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
    game_state = GameState()

    card_locations = []
    start_time = time.perf_counter()
    results = CARD_BOX_MODEL(img, verbose=False)

    for res in results:
        for i, box in enumerate(res.boxes):
            if float(box.conf) < 0.9:
                continue

            location = [float(val) for val in box.xyxy[0]]
            card_locations.append(location)
    location_end_time = time.perf_counter()

    card_images = []
    for i, location in enumerate(card_locations):
        card_image = img.crop(location)
        card_images.append(card_image)

    cards = get_card_information(card_images)

    for card in cards:
        print(card)
    print()

    feature_end_time = time.perf_counter()

    best_hand_start = time.perf_counter()
    best_hand = get_best_scoring_hand(cards, [], game_state)
    best_hand_end = time.perf_counter()

    card_location_time = location_end_time - start_time
    card_feature_time = feature_end_time - location_end_time
    card_best_hand_time = best_hand_end - best_hand_start

    total_time = card_location_time + card_feature_time + card_best_hand_time

    location_pct = card_location_time / total_time
    feature_pct = card_feature_time / total_time
    best_hand_pct = card_best_hand_time / total_time

    print(
        f"The location took {pretty_time(card_location_time)} which was {round(location_pct * 100, 3)}%"
    )
    print(
        f"The location took {pretty_time(card_feature_time)} which was {round(feature_pct * 100, 3)}%"
    )
    print(
        f"The location took {pretty_time(card_best_hand_time)} which was {round(best_hand_pct * 100, 3)}%"
    )

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
