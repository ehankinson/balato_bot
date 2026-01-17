"""Test the trained Balatro card recognition model on real screenshots."""

import os
import sys
import torch
from torchvision import transforms
from PIL import Image

# Add parent directory to path for imports
CURR_DIR = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, CURR_DIR)

from trainer import MultiHeadResNet18, head_sizes, RANKS, SUITS, ENHANCEMENTS, SEALS, EDITIONS


# Reverse lookup maps for predictions
IDX_TO_RANK = {i: v for i, v in enumerate(RANKS)}
IDX_TO_SUIT = {i: v for i, v in enumerate(SUITS)}
IDX_TO_ENHANCEMENT = {i: v for i, v in enumerate(ENHANCEMENTS)}
IDX_TO_SEAL = {i: v for i, v in enumerate(SEALS)}
IDX_TO_EDITION = {i: v for i, v in enumerate(EDITIONS)}


def load_model(model_path: str, device: str):
    """Load the trained model from a checkpoint."""
    checkpoint = torch.load(model_path, map_location=device, weights_only=False)
    img_size = checkpoint.get("img_size", 160)

    model = MultiHeadResNet18(head_sizes)
    model.load_state_dict(checkpoint["model"])
    model.to(device)
    model.eval()

    return model, img_size


def get_transform(img_size: int):
    """Get the transform for inference."""
    return transforms.Compose([
        transforms.Resize((img_size, img_size)),
        transforms.ToTensor(),
        transforms.Normalize([0.485, 0.456, 0.406], [0.229, 0.224, 0.225]),
    ])


def predict_card(model, image_path: str, transform, device: str) -> dict:
    """Run inference on a single card image."""
    img = Image.open(image_path).convert("RGB")
    img_tensor = transform(img).unsqueeze(0).to(device)

    with torch.no_grad():
        outputs = model(img_tensor)

    # Get predictions and confidence scores
    predictions = {}
    for key, logits in outputs.items():
        probs = torch.softmax(logits, dim=1)
        confidence, pred_idx = probs.max(dim=1)
        predictions[key] = {
            "idx": pred_idx.item(),
            "confidence": confidence.item()
        }

    return predictions


def format_prediction(predictions: dict) -> str:
    """Format predictions into a human-readable string."""
    rank = IDX_TO_RANK[predictions["rank"]["idx"]]
    suit = IDX_TO_SUIT[predictions["suit"]["idx"]]
    enhancement = IDX_TO_ENHANCEMENT[predictions["enhancement"]["idx"]]
    seal = IDX_TO_SEAL[predictions["seal"]["idx"]]
    edition = IDX_TO_EDITION[predictions["edition"]["idx"]]

    rank_conf = predictions["rank"]["confidence"]
    suit_conf = predictions["suit"]["confidence"]

    lines = [
        f"  Card: {rank} of {suit}",
        f"    Rank confidence: {rank_conf:.1%}",
        f"    Suit confidence: {suit_conf:.1%}",
    ]

    if enhancement != "blank_card":
        lines.append(f"  Enhancement: {enhancement} ({predictions['enhancement']['confidence']:.1%})")
    if seal != "None":
        lines.append(f"  Seal: {seal} ({predictions['seal']['confidence']:.1%})")
    if edition != "None":
        lines.append(f"  Edition: {edition} ({predictions['edition']['confidence']:.1%})")

    return "\n".join(lines)


def test_directory(test_dir: str, model_path: str):
    """Test all images in a directory."""
    device = "cuda" if torch.cuda.is_available() else "cpu"
    print(f"Using device: {device}\n")

    print(f"Loading model from {model_path}...")
    model, img_size = load_model(model_path, device)
    transform = get_transform(img_size)
    print(f"Model loaded (img_size={img_size})\n")

    # Get all image files
    valid_extensions = {".png", ".jpg", ".jpeg", ".bmp", ".webp"}
    image_files = sorted([
        f for f in os.listdir(test_dir)
        if os.path.splitext(f)[1].lower() in valid_extensions
    ])

    if not image_files:
        print(f"No images found in {test_dir}")
        return

    print(f"Found {len(image_files)} images to test\n")
    print("=" * 60)

    for filename in image_files:
        filepath = os.path.join(test_dir, filename)
        predictions = predict_card(model, filepath, transform, device)

        print(f"\n{filename}:")
        print(format_prediction(predictions))

    print("\n" + "=" * 60)
    print("Testing complete!")


if __name__ == "__main__":
    # Default paths
    default_test_dir = os.path.join(CURR_DIR, "../test_data")
    default_model_path = os.path.join(CURR_DIR, "../balatro_resnet18.pt")

    # Allow overrides via command line
    test_dir = sys.argv[1] if len(sys.argv) > 1 else default_test_dir
    model_path = sys.argv[2] if len(sys.argv) > 2 else default_model_path

    test_directory(test_dir, model_path)
