import torch
from torch import nn
from torchvision import models, transforms
from PIL import Image
from config.settings import ROOT_DIR

MODEL_QUEUE = {}

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


def run_model(name: str, img: Image.Image) -> str:
    if name not in MODEL_QUEUE:
        model, checkpoint = load_model(name)
        MODEL_QUEUE[name] = { "model": model, "checkpoint": checkpoint }

    model, checkpoint = MODEL_QUEUE[name].values()
    
    width, height = checkpoint["img_size"]

    transform = transforms.Compose([
        transforms.Resize((height, width)),
        transforms.ToTensor(),
    ])

    x = transform(img).unsqueeze(0)

    with torch.no_grad():
        outputs = model(x)
        prediction = outputs.argmax(1).item()

    normalaized_prediction = checkpoint["class_names"][prediction]

    return normalaized_prediction
    