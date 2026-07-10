import torch
from torch import nn
from torchvision import models, transforms
from PIL import Image
from config.settings import ROOT_DIR

MODEL_QUEUE = {}

FEATURE_MODELS = ["rank", "seal", "suit", "edition", "enhancement"]


def preload_models():
    """Eagerly load all feature models onto GPU and warm up CUDA kernels.

    Safe to call multiple times — already-loaded models are skipped.
    """
    device = "cuda"
    for name in FEATURE_MODELS:
        if name in MODEL_QUEUE:
            continue
        model, checkpoint = load_model(name)
        model.to(device)
        MODEL_QUEUE[name] = {"model": model, "checkpoint": checkpoint}

        width, height = checkpoint["img_size"]
        dummy = torch.zeros(1, 3, height, width, device=device)
        with torch.inference_mode():
            model(dummy)
        torch.cuda.synchronize()


def load_model(name: str):
    checkpoint = torch.load(f"{ROOT_DIR}/models/{name}_model.pt", map_location="cuda")

    model = models.mobilenet_v3_small(weights=None)
    model.classifier[3] = nn.Linear(
        model.classifier[3].in_features,
        checkpoint["num_classes"],
    )

    model.load_state_dict(checkpoint["state_dict"])
    model.eval()

    return model, checkpoint


def run_model(name: str, images: list[Image.Image]) -> list[int]:
    if name not in MODEL_QUEUE:
        model, checkpoint = load_model(name)
        MODEL_QUEUE[name] = {"model": model, "checkpoint": checkpoint}

    model = MODEL_QUEUE[name]["model"]
    checkpoint = MODEL_QUEUE[name]["checkpoint"]

    width, height = checkpoint["img_size"]

    transform = transforms.Compose([
        transforms.Resize((height, width)),
        transforms.ToTensor(),
    ])

    device = "cuda"

    x = torch.stack([transform(img) for img in images]).to(device)

    with torch.inference_mode():
        outputs = model(x)
        predictions = outputs.argmax(1).cpu()

    return [int(checkpoint["class_names"][prediction]) for prediction in predictions]
    