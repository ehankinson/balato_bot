import os
import torch

from torch import nn
from torch.utils.data import DataLoader
from torchvision import datasets, transforms, models
from ultralytics import YOLO

from util import load_json
from const import CURR_DIR, TRAINING_CONFIG


EPOCHS = 10
BATCH_SIZE = 64



def train_card_box():
    trainer_config = os.path.join(CURR_DIR, "../yaml/card_trainer.yaml")

    model = YOLO("yolo11n.pt")

    model.train(
        data     =trainer_config,
        epochs   =20,
        imgsz    =640, # scales input to be this square
        batch    =16,
        device   =0, # 0 GPU / 1 CPU
        workers  =4, # how many threads to use to load the data
        patience =20 # after x epchos if no change quite
    )



def load_config(key: str) -> dict:
    return load_json(TRAINING_CONFIG)[key]



def train_model(model_type: str):
    model_config = load_config(model_type)

    data_dir = model_config["data_dir"]
    img_size = model_config["img_size"]

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")

    # ===== TRANSFORMS (image prep) =====
    transform = transforms.Compose([
        transforms.Resize((img_size, img_size)),    # make all images same size
        transforms.ToTensor(),                      # convert to numbers
    ])

    # ===== LOAD DATA =====
    train_dataset = datasets.ImageFolder(f"{data_dir}/train", transform=transform)
    val_dataset   = datasets.ImageFolder(f"{data_dir}/val", transform=transform)

    train_loader = DataLoader(train_dataset, batch_size=BATCH_SIZE, shuffle=True)
    val_loader   = DataLoader(val_dataset, batch_size=BATCH_SIZE)

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

        print(
            f"Epoch {epoch+1} | "
            f"Train Loss: {total_loss:.3f} | Train Acc: {acc:.3f} | "
            f"Val Loss: {val_loss:.3f} | Val Acc: {val_acc:.3f}"
        )

    # ===== SAVE MODEL =====
    output_path = model_config["output_path"]

    torch.save({
        "model_type": model_type,
        "arch": "mobilenet_v3_small",
        "img_size": img_size,
        "num_classes": len(train_dataset.classes),
        "class_names": train_dataset.classes,
        "state_dict": model.state_dict(),
    }, output_path)


if __name__ == "__main__":
    train_model("suit")
