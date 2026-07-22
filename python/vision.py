import torch
from PIL import Image

from calculation.poker_discards import generate_discard_table
from calculation.score import get_best_scoring_hand
from config.model_registry import CARD_BOX_MODEL
from config.settings import (
    EDITION_CROP,
    ENHANCEMENT_CROP,
    RANK_CROP,
    SEAL_CROP,
    SUIT_CROP,
)
from core.enums import Edition, Enhancement, HandAction, Rank, Seal, Suit
from core.models import Card, CardData, Deck, GameState
from model import preload_models, run_model
from simulation.blind_trainer import BlindModel
from simulation.decoder import build_mask, model_decoder
from simulation.encoder import encode_game_state
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
    img: Image.Image, deck: Deck, game_state: GameState
) -> tuple[list[CardData], HandAction, list[Card]]:
    preload_models()
    checkpoint = torch.load("/home/hank/projects/balatro_bot/python/ppo_blind.pt")
    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    model = BlindModel(checkpoint["input_size"], checkpoint["hidden_size"]).to(device)
    model.load_state_dict(checkpoint["state_dict"])
    model.eval()

    card_locations = []
    results = CARD_BOX_MODEL(img, verbose=False)

    for res in results:
        for box in res.boxes:
            if float(box.conf) < 0.9:
                continue

            location = [float(val) for val in box.xyxy[0]]
            card_locations.append(location)

    card_locations.sort(key=lambda x: x[0])

    card_images = []
    for location in card_locations:
        card_image = img.crop(location)
        card_images.append(card_image)

    hand = get_card_information(card_images)
    deck.filter(hand)

    best_hand = get_best_scoring_hand(hand, [], game_state)
    discard_table = generate_discard_table(deck, hand)
    encoded_state = encode_game_state(hand, game_state, best_hand, discard_table)

    outputs = model(encoded_state.unsqueeze(0).to(device))
    masks = build_mask(game_state, hand, device)
    mode, _, card_indices, _, _ = model_decoder(
        outputs, masks, device, stochastic=False
    )

    return_data = []
    selected_cards = [hand[index] for index in sorted(card_indices)]

    for card in selected_cards:
        index = hand.index(card)
        card_location = card_locations[index]
        return_data.append(CardData(card=card, location=card_location))

    for card in selected_cards:
        hand.remove(card)

    return return_data, mode, hand


if __name__ == "__main__":
    img = Image.open("/home/hank/projects/balatro_bot/hand_0.png").convert("RGB")
    get_played_hand(img)
