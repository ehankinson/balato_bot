import sys
import torch

from torchvision import models, transforms
from PIL import Image
import cv2

from best_hand import get_best_scoring_hand
from utils.images import card_crop
from core.models import Card
from core.enums import Edition, Rank, Suit, Enhancement, Seal
from config.model_registry import BOX_MODEL
from config.settings import EDITION_CROP, RANK_CROP, SUIT_CROP, SEAL_CROP, ENHANCEMENT_CROP

TUPLE = False

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


def predict_image(img: Image.Image, model, transform, class_names, device):
    x = transform(img).unsqueeze(0).to(device)

    with torch.no_grad():
        output = model(x)
        pred_idx = output.argmax(1).item()

    return class_names[pred_idx]



def get_card_locations_in_hand(results: list) -> list[list[float]]:
    values = []

    for box in results[0].boxes:
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



def get_cards(image: Image.Image) -> list[Card]:
    results = BOX_MODEL(image, verbose=False)
    annotated = results[0].plot()
    cv2.imwrite("output.png", annotated)
    
    card_positions = get_card_locations_in_hand(results)

    detected_cards = []

    for i, (x1, y1, x2, y2) in enumerate(card_positions):
        card = image.crop((x1, y1, x2, y2))
        # card.save(f"{i}_card.png")
        w, h = card.size
        if w < 100 or h < 150:
            continue

        rank_crop = card.crop(card_crop(w, h, RANK_CROP))
        suit_crop = card.crop(card_crop(w, h, SUIT_CROP))
        enhancement_crop = card.crop(card_crop(w, h, ENHANCEMENT_CROP))
        seal_crop = card.crop(card_crop(w, h, SEAL_CROP))
        edition_crop = card.crop(card_crop(w, h, EDITION_CROP))

        # rank_crop.save(f"{i}_rank.png")
        # suit_crop.save(f"{i}_suit.png")
        # enhancement_crop.save(f"{i}_enhancement.png")
        # seal_crop.save(f"{i}_seal.png")

        rank = predict_rank(rank_crop)
        suit = predict_suit(suit_crop)
        seal = predict_seal(seal_crop)
        enhancement = predict_enhancement(enhancement_crop)
        edition = predict_edition(edition_crop)

        detected_cards.append(Card(
            rank=Rank(int(rank)),
            suit=Suit(int(suit)),
            enhancement=Enhancement(int(enhancement)),
            seal=Seal(int(seal)),
            edition=Edition(int(edition))
        ))

    return detected_cards


if __name__ == '__main__':
    args = sys.argv
    image_count = args[1]
    image = Image.open(f"training_data/real_data/hand_{image_count}.png").convert("RGB")
    # image = Image.open("image.png").convert("RGB")
    cards = get_cards(image)
    for card in cards:
        print(card)
        
    best_score, best_hand = get_best_scoring_hand(cards)
    print(f"\nThe best score is {best_score}")
    print(f"The Cards played are {best_hand[0]}")
    print(f"The Cards held in hand are {best_hand[1]}")
