import os
import csv
from dataclasses import dataclass
from typing import Dict

from tqdm import tqdm
import torch
import torch.nn as nn
from torch.utils.data import Dataset, DataLoader, random_split
from torchvision import transforms
from torchvision.models import resnet18, ResNet18_Weights
from PIL import Image


# ---------------- CONFIG ----------------
@dataclass
class Config:
    """Configuration for the trainer."""
    images_dir = "training_data/images"
    labels_csv = "training_data/labels.csv"
    img_size = 160
    batch_size = 128
    lr = 3e-4
    epochs = 10
    val_split = 0.1
    num_workers = 4
    device = "cuda" if torch.cuda.is_available() else "cpu"

cfg = Config()


# ---------------- LABELS ----------------
RANKS = ["A","2","3","4","5","6","7","8","9","10","J","Q","K"]
SUITS = ["hearts","diamonds","clubs","spades"]
ENHANCEMENTS = ["blank_card","glass","steel","wild","stone","gold","lucky"]
SEALS = ["None","red","blue","gold","purple"]
EDITIONS = ["None","foil","holo","polychrome","negative"]

label_maps = {
    "rank": {v: i for i, v in enumerate(RANKS)},
    "suit": {v: i for i, v in enumerate(SUITS)},
    "enhancement": {v: i for i, v in enumerate(ENHANCEMENTS)},
    "seal": {v: i for i, v in enumerate(SEALS)},
    "edition": {v: i for i, v in enumerate(EDITIONS)},
}


# ---------------- DATASET ----------------
class BalatroDataset(Dataset):
    """Dataset for the Balatro cards."""

    def __init__(self, images_dir, labels_csv, transform=None):
        """Initialize the Balatro dataset."""
        self.images_dir = images_dir
        self.transform = transform
        with open(labels_csv, encoding="utf-8") as f:
            self.rows = list(csv.DictReader(f))

    def __len__(self):
        return len(self.rows)



    def __getitem__(self, idx):
        r = self.rows[idx]
        img = Image.open(os.path.join(self.images_dir, r["filename"])).convert("RGB")
        if self.transform:
            img = self.transform(img)

        labels = {k: torch.tensor(m[r[k]], dtype=torch.long) for k, m in label_maps.items()}
        return img, labels


# ---------------- TRANSFORMS ----------------
train_tf = transforms.Compose([
    transforms.Resize((cfg.img_size, cfg.img_size)),
    transforms.ColorJitter(0.2, 0.2, 0.2, 0.02),
    transforms.RandomAffine(degrees=2, translate=(0.02, 0.02), scale=(0.95, 1.05)),
    transforms.GaussianBlur(3, sigma=(0.1, 1.0)),
    transforms.ToTensor(),
    transforms.Normalize([0.485,0.456,0.406], [0.229,0.224,0.225]),
])

val_tf = transforms.Compose([
    transforms.Resize((cfg.img_size, cfg.img_size)),
    transforms.ToTensor(),
    transforms.Normalize([0.485,0.456,0.406], [0.229,0.224,0.225]),
])


# ---------------- MODEL ----------------
class MultiHeadResNet18(nn.Module):
    """Multi-head ResNet18 model."""
    def __init__(self, head_sizes: Dict[str, int]):
        """Initialize the MultiHeadResNet18 model."""
        super().__init__()
        base = resnet18(weights=ResNet18_Weights.DEFAULT)
        in_features = base.fc.in_features
        base.fc = nn.Identity()
        self.backbone = base
        self.heads = nn.ModuleDict({
            name: nn.Linear(in_features, size)
            for name, size in head_sizes.items()
        })

    def forward(self, x):
        """Forward pass through the model."""
        f = self.backbone(x)
        return {k: h(f) for k, h in self.heads.items()}


head_sizes = {
    "rank": len(RANKS),
    "suit": len(SUITS),
    "enhancement": len(ENHANCEMENTS),
    "seal": len(SEALS),
    "edition": len(EDITIONS),
}


if __name__ == '__main__':
    # ---------------- TRAINING ----------------
    model = MultiHeadResNet18(head_sizes).to(cfg.device)

    dataset = BalatroDataset(cfg.images_dir, cfg.labels_csv, train_tf)
    val_size = int(len(dataset) * cfg.val_split)
    train_ds, val_ds = random_split(dataset, [len(dataset)-val_size, val_size])
    val_ds.dataset.transform = val_tf

    train_loader = DataLoader(train_ds, cfg.batch_size, shuffle=True, num_workers=cfg.num_workers)
    val_loader = DataLoader(val_ds, cfg.batch_size, shuffle=False, num_workers=cfg.num_workers)

    criterion = nn.CrossEntropyLoss()
    optimizer = torch.optim.AdamW(model.parameters(), lr=cfg.lr)

    for epoch in range(cfg.epochs):
        model.train()
        total_loss = 0

        # Training loop with progress bar
        train_pbar = tqdm(train_loader, desc=f"Epoch {epoch+1}/{cfg.epochs} [Train]", leave=False)
        for x, y in train_pbar:
            x = x.to(cfg.device)
            y = {k: v.to(cfg.device) for k, v in y.items()}

            out = model(x)
            loss = sum(criterion(out[k], y[k]) for k in out)

            optimizer.zero_grad()
            loss.backward()
            optimizer.step()

            total_loss += loss.item()
            train_pbar.set_postfix(loss=f"{loss.item():.4f}")

        model.eval()
        correct = {k: 0 for k in head_sizes}
        total = 0

        # Validation loop with progress bar
        val_pbar = tqdm(val_loader, desc=f"Epoch {epoch+1}/{cfg.epochs} [Val]", leave=False)
        with torch.no_grad():
            for x, y in val_pbar:
                x = x.to(cfg.device)
                y = {k: v.to(cfg.device) for k, v in y.items()}
                out = model(x)

                for k in out:
                    pred = out[k].argmax(1)
                    correct[k] += (pred == y[k]).sum().item()
                total += x.size(0)

        # Print epoch summary
        avg_loss = total_loss / len(train_loader)
        acc_str = " | ".join(f"{k}: {correct[k]/total:.2%}" for k in correct)
        tqdm.write(f"Epoch {epoch+1}/{cfg.epochs} — Loss: {avg_loss:.4f} — Val Acc: {acc_str}")

    # ---------------- SAVE ----------------
    torch.save({
        "model": model.state_dict(),
        "label_maps": label_maps,
        "img_size": cfg.img_size
    }, "balatro_resnet18.pt")

    print("\nSaved model to balatro_resnet18.pt")
