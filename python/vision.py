import os
import torch

from torchvision import models, transforms
from PIL import Image
from ultralytics import YOLO
from const import CURR_DIR, Card, Rank, Suit, Enhancement, Seal
from create_card import create_card
from create_hand import WIDTH_MULT, HEIGHT_MULT

CARD_BOX_MODEL = YOLO(os.path.join(CURR_DIR, "../models/card_selector.pt"))
# load model once
rank_model = models.mobilenet_v3_small(weights=None)
rank_model.classifier[3] = torch.nn.Linear(rank_model.classifier[3].in_features, 13)

rank_model.load_state_dict(torch.load("models/rank_model.pt"))
rank_model.eval()

# transform
rank_transform = transforms.Compose([
    transforms.Resize((64, 64)),
    transforms.ToTensor(),
])

# class mapping (IMPORTANT — match training!)
class_names = ['1', '10', '11', '12', '13', '2', '3', '4', '5', '6', '7', '8', '9']


def create_rank_cache() -> list[Image]:
    values = []
    for rank in list(Rank):
        card = create_card(Card(rank, Suit.SPADES, Enhancement.NONE, Seal.NONE))

        card_image = card.image
        card_image = card_image.resize(
            (int(card_image.width * WIDTH_MULT),int(card_image.height * HEIGHT_MULT)),
            Image.Resampling.LANCZOS
        )
        crop_image = card_image.crop((17, 15, 59, 55))
        crop_image.save(f"{rank}.png")


def get_card_locations_in_hand(results: list) -> list[Card]:
    values = []

    for box in results[0].boxes:
        x1, y1, x2, y2 = box.xyxy[0]
        x1, y1, x2, y2 = map(float, (x1, y1, x2, y2))
        values.append([x1, y1, x2, y2])

    return sorted(values, key=lambda x: x[0])



def predict_rank(img: Image):
    x = rank_transform(img).unsqueeze(0)

    with torch.no_grad():
        output = rank_model(x)
        pred_idx = output.argmax(1).item()

    rank = class_names[pred_idx]

    return rank



def get_cards(image: Image):
    results = CARD_BOX_MODEL(image)
    card_positions = get_card_locations_in_hand(results)

    detected_cards = []

    for (x1, y1, x2, y2) in card_positions:
        card = image.crop((x1, y1, x2, y2))
        w, h = card.size

        rank_crop = card.crop((
            0, 0, int(w * 0.28), int(h * 0.25)
        ))

        rank = predict_rank(rank_crop)

        detected_cards.append(Rank(int(rank)))

    print(detected_cards)


if __name__ == '__main__':
    image = Image.open("/home/hank/projects/balato_bot/training_data/real_data_2.png").convert("RGB")
    get_cards(image)
