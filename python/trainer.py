import os
import sys

import torch
from config.settings import FOLDER_TRAINING_NAMES, ROOT_DIR, TRAINING_CONFIG
from core.enums import CardFeatureTrainingType
from torch import nn
from torch.utils.data import DataLoader
from torchvision import datasets, models, transforms
from ultralytics import YOLO
from utils.files import load_json

EPOCHS = 5
PATIENCE = 2
BATCH_SIZE = 64


def train_card_box(config: str):
    trainer_config = os.path.join(ROOT_DIR, config)

    model = YOLO("yolo11n.pt")

    model.train(
        data=trainer_config,
        epochs=EPOCHS,
        imgsz=640,  # scales input to be this square
        batch=BATCH_SIZE,
        device="cpu" if sys.platform == "darwin" else 0,  # 0 GPU / 1 CPU
        workers=4,  # how many threads to use to load the data
        patience=PATIENCE,  # after x epchos if no change quite
    )


def load_config(key: str) -> dict:
    return load_json(TRAINING_CONFIG)[key]


def train_model(model_type: str):
    print("started training")
    model_config = load_config(model_type)

    data_dir = model_config["data_dir"]
    width, height = model_config["img_size"]

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")

    # ===== TRANSFORMS (image prep) =====
    transform = transforms.Compose(
        [
            transforms.Resize((height, width)),  # torchvision uses (height, width)
            transforms.ToTensor(),  # convert to numbers
        ]
    )

    # ===== LOAD DATA =====
    train_dataset = datasets.ImageFolder(f"{data_dir}/train", transform=transform)
    val_dataset = datasets.ImageFolder(f"{data_dir}/val", transform=transform)

    train_loader = DataLoader(train_dataset, batch_size=BATCH_SIZE, shuffle=True)
    val_loader = DataLoader(val_dataset, batch_size=BATCH_SIZE)

    # ===== MODEL =====
    model = models.mobilenet_v3_small(
        weights=models.MobileNet_V3_Small_Weights.IMAGENET1K_V1
    )

    # Replace final layer (VERY IMPORTANT)
    final_features = model_config["features"]
    model.classifier[3] = nn.Linear(model.classifier[3].in_features, final_features)

    model = model.to(device)

    # ===== TRAINING SETUP =====
    criterion = nn.CrossEntropyLoss()
    optimizer = torch.optim.Adam(model.parameters(), lr=0.001)

    # ===== TRAIN LOOP =====
    for epoch in range(EPOCHS):
        print(f"Started Epoch {epoch + 1} |")
        model.train()

        total_loss = 0
        correct = 0
        total = 0

        for images, labels in train_loader:
            images, labels = images.to(device), labels.to(device)

            outputs = model(images)
            loss = criterion(outputs, labels)

            optimizer.zero_grad()
            loss.backward()
            optimizer.step()

            total_loss += loss.item()

            preds = outputs.argmax(1)
            correct += (preds == labels).sum().item()
            total += labels.size(0)

        acc = correct / total

        print("Finished Training | ")
        print(f"Train Loss: {total_loss:.3f} | Train Acc: {acc:.3f} | ")

        model.eval()
        val_loss = 0
        val_correct = 0
        val_total = 0

        with torch.no_grad():
            for images, labels in val_loader:
                images, labels = images.to(device), labels.to(device)

                outputs = model(images)
                loss = criterion(outputs, labels)

                val_loss += loss.item()

                preds = outputs.argmax(1)
                val_correct += (preds == labels).sum().item()
                val_total += labels.size(0)

        val_acc = val_correct / val_total

        print(f"Val Loss: {val_loss:.3f} | Val Acc: {val_acc:.3f}")

    # ===== SAVE MODEL =====
    output_path = model_config["output_path"]

    torch.save(
        {
            "model_type": model_type,
            "arch": "mobilenet_v3_small",
            "img_size": [width, height],
            "num_classes": len(train_dataset.classes),
            "class_names": train_dataset.classes,
            "state_dict": model.state_dict(),
        },
        output_path,
    )


def train_joker_features() -> None:
    for arg in ["joker_edition", "joker_type"]:
        train_model(arg)


if __name__ == "__main__":
    available_commands = {
        # "all_card_features": {"function": generate_all_feature_data},
        # "card_enhancement": {"function": generate_card_feature_data, "args": [CardFeatureTrainingType.ENHANCEMENT]},
        # "card_edition": {"function": generate_card_feature_data, "args": [CardFeatureTrainingType.EDITION]},
        # "card_rank": {"function": generate_card_feature_data, "args": [CardFeatureTrainingType.RANK]},
        # "card_suit": {"function": generate_card_feature_data, "args": [CardFeatureTrainingType.SUIT]},
        # "card_seal": {"function": generate_card_feature_data, "args": [CardFeatureTrainingType.SEAL]},
        "playing_hands": {
            "function": train_card_box,
            "args": ["yaml/card_trainer.yaml"],
        },
        "jokers": {"function": train_card_box, "args": ["yaml/joker_trainer.yaml"]},
        "joker_edition": {"function": train_model, "args": ["joker_edition"]},
        "joker_type": {"function": train_model, "args": ["joker_type"]},
        "all_joker_features": {"function": train_joker_features},
    }

    if len(sys.argv) < 2 or sys.argv[1] not in available_commands:
        print("Sorry that command is invalid please add 1 of the following:")
        for key in available_commands.keys():
            print(key)

        exit()

    command = sys.argv[1]
    function = available_commands[command]["function"]
    args = available_commands[command].get("args", [])
    function(*args)
