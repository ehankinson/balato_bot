import os
import sys
import tempfile

import torch
from torch import nn
from torch.utils.data import DataLoader
from torchvision import datasets, models, transforms
from torchvision.utils import save_image
from tqdm import tqdm
from ultralytics import YOLO

from config.settings import ROOT_DIR, TRAINING_CONFIG
from utils.files import load_json, load_yaml, write_yaml

EPOCHS = 6
PATIENCE = 2
BATCH_SIZE = 256
BOX_BATCH_SIZE = 64


def resolve_dataset_config(config: str) -> str:
    trainer_config = os.path.join(ROOT_DIR, config)
    dataset_config = load_yaml(trainer_config)
    dataset_config["path"] = os.path.normpath(
        os.path.join(ROOT_DIR, dataset_config["path"])
    )

    resolved_path = os.path.join(
        tempfile.gettempdir(), f"balatro_{os.path.basename(config)}"
    )
    write_yaml(resolved_path, dataset_config)
    return resolved_path


def train_card_box(config: str):
    trainer_config = resolve_dataset_config(config)

    model = YOLO("yolo11n.pt")
    device = (
        "cuda"
        if torch.cuda.is_available()
        else "mps"
        if torch.backends.mps.is_available()
        else "cpu"
    )
    threads = 1 if os.cpu_count() is None else os.cpu_count()
    assert isinstance(threads, int)
    workers = min(8, threads - 1)

    print(f"Using device: {device}")

    model.train(
        data=trainer_config,
        epochs=EPOCHS,
        imgsz=640,  # scales input to be this square
        batch=BOX_BATCH_SIZE,
        device=device,
        workers=workers,  # how many threads to use to load the data
        patience=PATIENCE,  # after x epchos if no change quite
    )


def load_config(key: str) -> dict:
    return load_json(TRAINING_CONFIG)[key]


def train_model(model_type: str, debug: bool = False):
    print(f"Started Training '{model_type}'")
    model_config = load_config(model_type)

    fails_dir = os.path.join(
        ROOT_DIR, "training_data", "training", f"{model_type}_fails"
    )
    if debug:
        os.makedirs(fails_dir, exist_ok=True)
        print(f"Saving final-epoch misclassified images to {fails_dir}")

    data_dir = model_config["data_dir"]
    width, height = model_config["img_size"]

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    print(f"Using device: {device}")

    # ===== TRANSFORMS (image prep) =====
    transform = transforms.Compose(
        [
            transforms.Resize((height, width)),  # torchvision uses (height, width)
            transforms.ToTensor(),  # convert to numbers
        ]
    )

    # ===== LOAD DATA =====
    train_dataset = datasets.ImageFolder(
        f"{ROOT_DIR}/{data_dir}/train", transform=transform
    )
    val_dataset = datasets.ImageFolder(
        f"{ROOT_DIR}/{data_dir}/val", transform=transform
    )

    train_loader = DataLoader(
        train_dataset,
        batch_size=BATCH_SIZE,
        shuffle=True,
        num_workers=12,
        pin_memory=True,
        persistent_workers=True,
    )
    val_loader = DataLoader(
        val_dataset,
        batch_size=BATCH_SIZE,
        num_workers=12,
        pin_memory=True,
        persistent_workers=True,
    )

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

        for images, labels in tqdm(
            train_loader, desc=f"Epoch {epoch + 1}/{EPOCHS} [Train]", unit="batch"
        ):
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
        fails_saved = 0
        val_index = 0

        with torch.no_grad():
            for images, labels in tqdm(
                val_loader, desc=f"Epoch {epoch + 1}/{EPOCHS} [Val]", unit="batch"
            ):
                images, labels = (
                    images.to(device, non_blocking=True),
                    labels.to(device, non_blocking=True),
                )

                outputs = model(images)
                loss = criterion(outputs, labels)

                val_loss += loss.item()

                preds = outputs.argmax(1)
                val_correct += (preds == labels).sum().item()
                val_total += labels.size(0)

                if debug and epoch == EPOCHS - 1:
                    wrong = (preds != labels).nonzero(as_tuple=True)[0]
                    for i in wrong:
                        idx = val_index + i.item()
                        true_name = val_dataset.classes[labels[i].item()]
                        pred_name = val_dataset.classes[preds[i].item()]
                        original = os.path.basename(val_dataset.samples[idx][0])
                        save_image(
                            images[i].cpu(),
                            os.path.join(
                                fails_dir,
                                f"ep{epoch + 1}_true-{true_name}_pred-{pred_name}_{original}",
                            ),
                        )
                        fails_saved += 1
                val_index += labels.size(0)

        val_acc = val_correct / val_total

        if debug and epoch == EPOCHS - 1:
            print(f"Saved {fails_saved} misclassified images to {fails_dir}")

        print(f"Val Loss: {val_loss:.3f} | Val Acc: {val_acc:.3f}\n")

    # ===== SAVE MODEL =====
    output_path = f"{ROOT_DIR}/{model_config['output_path']}"

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


def train_joker_features(debug: bool = False) -> None:
    for arg in ["joker_edition", "joker_type"]:
        train_model(arg, debug=debug)


def train_card_features(debug: bool = False) -> None:
    for arg in ["rank", "suit", "seal", "edition", "enhancement"]:
        train_model(arg, debug=debug)


if __name__ == "__main__":
    available_commands = {
        "all_card_features": {"function": train_card_features},
        "all_joker_features": {"function": train_joker_features},
        "enhancement": {"function": train_model, "args": ["enhancement"]},
        "edition": {"function": train_model, "args": ["edition"]},
        "rank": {"function": train_model, "args": ["rank"]},
        "suit": {"function": train_model, "args": ["suit"]},
        "seal": {"function": train_model, "args": ["seal"]},
        "joker_edition": {"function": train_model, "args": ["joker_edition"]},
        "joker_type": {"function": train_model, "args": ["joker_type"]},
        "locations": {
            "function": train_card_box,
            "args": ["yaml/location_trainer.yaml"],
        },
        "tarot": {"function": train_model, "args": ["tarot"]},
    }

    if len(sys.argv) < 2 or sys.argv[1] not in available_commands:
        print("Sorry that command is invalid please add 1 of the following:")
        for key in available_commands.keys():
            print(key)
        print("Optionally append --debug to save misclassified val images")

        exit()

    debug = "--debug" in sys.argv[2:]
    command = sys.argv[1]
    function = available_commands[command]["function"]
    args = available_commands[command].get("args", [])

    if function in (train_model, train_card_features, train_joker_features):
        function(*args, debug=debug)
    else:
        function(*args)
