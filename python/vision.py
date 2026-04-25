import os
import time
import torch

from torchvision import models, transforms
from PIL import Image

from util import card_crop
from card_models import Card
from card_enums import Rank, Suit, Enhancement, Seal
from const import BOX_MODEL, RANK_CROP, SUIT_CROP
from render_card import render_card, WIDTH_MULT, HEIGHT_MULT

TUPLE = True

def load_model(model_path: str):
    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    checkpoint = torch.load(model_path, map_location=device)

    model = models.mobilenet_v3_small(weights=None)
    model.classifier[3] = torch.nn.Linear(
        model.classifier[3].in_features,
        checkpoint["num_classes"]
    )

    model.load_state_dict(checkpoint["state_dict"])
    model = model.to(device)
    model.eval()

    transform = transforms.Compose([
        transforms.Resize((checkpoint["img_size"], checkpoint["img_size"])),
        transforms.ToTensor(),
    ])

    return model, transform, checkpoint["class_names"], device


# load model once
rank_model, rank_transform, rank_class_names, rank_device = load_model("models/rank_model.pt")
suit_model, suit_transform, suit_class_names, suit_device = load_model("models/suit_model.pt")



def predict_image(img: Image, model, transform, class_names, device):
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



def predict_rank(img: Image):
    return predict_image(img, rank_model, rank_transform, rank_class_names, rank_device)



def predict_suit(img: Image):
    return predict_image(img, suit_model, suit_transform, suit_class_names, suit_device)



def get_cards(image: Image):
    for _ in range(5):
        results = BOX_MODEL(image)
        card_positions = get_card_locations_in_hand(results)

        detected_cards = []

        for (x1, y1, x2, y2) in card_positions:
            card = image.crop((x1, y1, x2, y2))
            w, h = card.size

            rank_crop = card.crop(card_crop(w, h, RANK_CROP, TUPLE))

            suit_crop = card.crop(card_crop(w, h, SUIT_CROP, TUPLE))

            rank = predict_rank(rank_crop)
            suit = predict_suit(suit_crop)

            detected_cards.append(Card(
                rank=Rank(int(rank)),
                suit=Suit(int(suit)),
                enhancement=Enhancement.NONE,
                seal=Seal.NONE
            ))

    for card in detected_cards:
        print(card)


if __name__ == '__main__':
    image = Image.open("/home/hank/projects/balato_bot/training_data/real_data_1.png").convert("RGB")
    get_cards(image)
