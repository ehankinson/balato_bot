import sys
import torch

from torchvision import models, transforms
from PIL import Image
import cv2

from best_hand import get_best_scoring_hand
from utils.images import card_crop
from core.models import Card, Joker
from core.enums import Edition, Rank, Suit, Enhancement, Seal, Jokers
from core.class_indices import NEGATIVE_JOKER_EDITION_ID
from config.model_registry import CARD_BOX_MODEL, JOKER_BOX_MODEL
from config.settings import EDITION_CROP, JOKER_TYPE_CROP, RANK_CROP, SUIT_CROP, SEAL_CROP, ENHANCEMENT_CROP

TUPLE = False
CONFIDENCE = 0.9

def load_model(model_path: str):
    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    checkpoint = torch.load(model_path, map_location=device)
    width, height = checkpoint["img_size"]

    model = models.mobilenet_v3_small(weights=None)
    model.classifier[3] = torch.nn.Linear(
        model.classifier[3].in_features,
        checkpoint["num_classes"]
    )

    model.load_state_dict(checkpoint["state_dict"])
    model = model.to(device)
    model.eval()

    transform = transforms.Compose([
        transforms.Resize((height, width)),
        transforms.ToTensor(),
    ])

    return model, transform, checkpoint["class_names"], device


# load model once
rank_model, rank_transform, rank_class_names, rank_device = load_model("models/rank_model.pt")
suit_model, suit_transform, suit_class_names, suit_device = load_model("models/suit_model.pt")
enhancement_model, enhancement_transform, enhancement_class_names, enhancement_device = load_model("models/enhancement_model.pt")
seal_model, seal_transform, seal_class_names, seal_device = load_model("models/seal_model.pt")
edition_model, edition_transform, edition_class_names, edition_device = load_model("models/edition_model.pt")
joker_type_model, joker_type_transform, joker_type_class_names, joker_type_device = load_model("models/joker_type_model.pt")
joker_edition_model, joker_edition_transform, joker_edition_class_names, joker_edition_device = load_model("models/joker_edition_model.pt")


def predict_image(img: Image.Image, model, transform, class_names, device):
    x = transform(img).unsqueeze(0).to(device)

    with torch.no_grad():
        output = model(x)
        pred_idx = output.argmax(1).item()

    return class_names[pred_idx]



def get_card_locations_in_hand(results: list) -> list[list[float]]:
    values = []

    for box in results[0].boxes:
        confidence = float(box.conf[0])
        if confidence < CONFIDENCE:
            continue

        x1, y1, x2, y2 = box.xyxy[0]
        x1, y1, x2, y2 = map(float, (x1, y1, x2, y2))
        values.append([x1, y1, x2, y2])

    return sorted(values, key=lambda x: x[0])



def predict_rank(img: Image.Image):
    return predict_image(img, rank_model, rank_transform, rank_class_names, rank_device)



def predict_suit(img: Image.Image):
    return predict_image(img, suit_model, suit_transform, suit_class_names, suit_device)



def predict_enhancement(img: Image.Image):
    return predict_image(img, enhancement_model, enhancement_transform, enhancement_class_names, enhancement_device)



def predict_seal(img: Image.Image):
    return predict_image(img, seal_model, seal_transform, seal_class_names, seal_device)


def predict_edition(img: Image.Image):
    return predict_image(img, edition_model, edition_transform, edition_class_names, edition_device)


def predict_joker_type(img: Image.Image):
    return predict_image(img, joker_type_model, joker_type_transform, joker_type_class_names, joker_type_device)


def predict_joker_edition(img: Image.Image):
    return predict_image(img, joker_edition_model, joker_edition_transform, joker_edition_class_names, joker_edition_device)



def get_cards(image: Image.Image) -> list[Card | Joker]:
    results = JOKER_BOX_MODEL(image, verbose=False)
    annotated = results[0].plot()
    cv2.imwrite("output.png", annotated)
    
    joker_positions = get_card_locations_in_hand(results)

    detected_cards = []

    for i, (x1, y1, x2, y2) in enumerate(joker_positions):
        card = image.crop((x1, y1, x2, y2))
        # card.save(f"{i}_card.png")
        w, h = card.size
    #     if w < 100 or h < 150:
    #         continue
    # 
        joker_crop = card.crop(card_crop(w, h, JOKER_TYPE_CROP))
        joker_crop.save(f"{i}.png")
    #     rank_crop = card.crop(card_crop(w, h, RANK_CROP))
    #     suit_crop = card.crop(card_crop(w, h, SUIT_CROP))
    #     enhancement_crop = card.crop(card_crop(w, h, ENHANCEMENT_CROP))
    #     seal_crop = card.crop(card_crop(w, h, SEAL_CROP))
    #     edition_crop = card.crop(card_crop(w, h, EDITION_CROP))

        # joker_crop.save(f"{i}_joker_type.png")
    #     # rank_crop.save(f"{i}_rank.png")
    #     # suit_crop.save(f"{i}_suit.png")
    #     # enhancement_crop.save(f"{i}_enhancement.png")
    #     # seal_crop.save(f"{i}_seal.png")
    # 
        joker_type = predict_joker_type(joker_crop)
        joker_edition = predict_joker_edition(joker_crop)
    #     rank = predict_rank(rank_crop)
    #     suit = predict_suit(suit_crop)
    #     seal = predict_seal(seal_crop)
    #     enhancement = predict_enhancement(enhancement_crop)
    #     edition = predict_edition(edition_crop)

    #     detected_cards.append(Card(
    #         rank=Rank(int(rank)),
    #         suit=Suit(int(suit)),
    #         enhancement=Enhancement(int(enhancement)),
    #         seal=Seal(int(seal)),
    #         edition=Edition(int(edition))
    #     ))
    # 
        int_e = int(joker_edition)
        is_negative = int_e == NEGATIVE_JOKER_EDITION_ID
        edition = 0 if is_negative else int_e
        detected_cards.append(Joker(
            background_image=Jokers(int(joker_type)),
            negative=is_negative,
            edition=Edition(edition)
        ))

    return detected_cards


if __name__ == '__main__':
    args = sys.argv
    image_count = args[1]
    # image = Image.open(f"training_data/real_data/hand_{image_count}.png").convert("RGB")
    image = Image.open("training_data/3_joker.png").convert("RGB")
    cards = get_cards(image)
    for c in cards:
        print(c)
        
    # best_score, best_hand = get_best_scoring_hand(cards)
    # print(f"\nThe best score is {best_score}")
    # print(f"The Cards played are {best_hand[0]}")
    # print(f"The Cards held in hand are {best_hand[1]}")
