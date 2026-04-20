import os
import torch

from torch import nn
from torch.utils.data import DataLoader
from torchvision import datasets, transforms, models
from ultralytics import YOLO
from const import CURR_DIR

def train_card_box():
    trainer_config = os.path.join(CURR_DIR, "../yaml/card_trainer.yaml")

    model = YOLO("yolo11n.pt")

    model.train(
        data=trainer_config,
        epochs=20,
        imgsz=640, # scales input to be this square
        batch=16,
        device=0, # 0 GPU / 1 CPU
        workers=4, # how many threads to use to load the data
        patience=20 # after x epchos if no change quite
    )



def train_card_rank():
    data_dir = "training_data/rank_data"
    batch_size = 64
    epochs = 10
    img_size = 64

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")

    # ===== TRANSFORMS (image prep) =====
    transform = transforms.Compose([
        transforms.Resize((img_size, img_size)),        # make all images same size
        transforms.ToTensor(),                          # convert to numbers
    ])

    # ===== LOAD DATA =====
    train_dataset = datasets.ImageFolder(f"{data_dir}/train", transform=transform)
    val_dataset   = datasets.ImageFolder(f"{data_dir}/val", transform=transform)

    train_loader = DataLoader(train_dataset, batch_size=batch_size, shuffle=True)
    val_loader   = DataLoader(val_dataset, batch_size=batch_size)

    # ===== MODEL =====
    model = models.mobilenet_v3_small(pretrained=True)

    # Replace final layer (VERY IMPORTANT)
    model.classifier[3] = nn.Linear(model.classifier[3].in_features, 13)

    model = model.to(device)

    # ===== TRAINING SETUP =====
    criterion = nn.CrossEntropyLoss()
    optimizer = torch.optim.Adam(model.parameters(), lr=0.001)

    # ===== TRAIN LOOP =====
    for epoch in range(epochs):
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
        print(f"Epoch {epoch+1} | Loss: {total_loss:.3f} | Train Acc: {acc:.3f}")

    # ===== SAVE MODEL =====
    torch.save(model.state_dict(), "rank_model.pt")

if __name__ == "__main__":
    train_card_rank()