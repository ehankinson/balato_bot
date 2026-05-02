import os
import random
import threading
from concurrent.futures import ThreadPoolExecutor

from core.enums import CardFeatureTrainingType, Edition, Enhancement, Rank, Seal, Suit
from core.models import Card, Hand, RenderedHand
from config.model_registry import BOX_MODEL
from config.settings import (
    ENHANCEMENT_CROP,
    FOLDER_TRAINING_NAMES,
    RANK_CROP,
    SEAL_CROP,
    SUIT_CROP,
    EDITION_CROP
)
from PIL import Image
from rendering.hand import render_hand
from tqdm import tqdm
from utils.files import build_folder, rebuild_folder
from utils.images import card_crop
from vision import get_card_locations_in_hand

CUTOFF = 0.9 # split between training and val
CPU_COUNT = os.cpu_count() if os.cpu_count() is not None else 1
BATCH_SIZE = 50

FEATURE_ENUMS = {
    CardFeatureTrainingType.RANK: Rank,
    CardFeatureTrainingType.SUIT: Suit,
    CardFeatureTrainingType.ENHANCEMENT: Enhancement,
    CardFeatureTrainingType.SEAL: Seal,
    CardFeatureTrainingType.EDITION: Edition
}


def random_full_card_amount() -> int:
    return random.choices(
        population=[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18],
        weights=[1, 1, 2, 3, 5, 8, 16, 50, 33, 18, 7, 4, 1, 1, 1, 1, 1, 1],
        k=1
    )[0]


def random_feature_card_amount() -> int:
    return random.choices(
        population=[6, 7, 8, 9, 10, 11],
        weights=[8, 16, 50, 33, 18, 7],
        k=1
    )[0]



def split_work(total_amount: int, worker_amount: int) -> list[range]:
    chunk_size, extra = divmod(total_amount, worker_amount)
    chunks = []
    start = 0
    for worker_index in range(worker_amount):
        end = start + chunk_size + (1 if worker_index < extra else 0)
        chunks.append(range(start, end))
        start = end

    return chunks



def generate_rendered_hand(hand_index: int, cutoff: float, is_feature: bool = False) -> tuple[str, str, RenderedHand]:
    card_amount = random_full_card_amount() if not is_feature else random_feature_card_amount()
    hand = Hand.random_hand(card_amount)
    hand_render = render_hand(hand)

    name = f"{hand_index}_{card_amount}"
    split = "train" if hand_index < cutoff else "val"
    return name, split, hand_render



def feature_info(
    train_type: CardFeatureTrainingType, card_image: Image.Image, card: Card
) -> tuple[Rank | Suit | Enhancement | Seal | Edition, tuple[int, int, int, int]]:
    w, h = card_image.size
    match train_type:
        case CardFeatureTrainingType.RANK:
            return card.rank, card_crop(w, h, RANK_CROP)

        case CardFeatureTrainingType.SUIT:
            return card.suit, card_crop(w, h, SUIT_CROP)

        case CardFeatureTrainingType.ENHANCEMENT:
            return card.enhancement, card_crop(w, h, ENHANCEMENT_CROP)

        case CardFeatureTrainingType.SEAL:
            return card.seal, card_crop(w, h, SEAL_CROP)

        case CardFeatureTrainingType.EDITION:
            return card.edition, card_crop(w, h, EDITION_CROP)



def generate_hand_training_data(hand_amount: int = 5000) -> None:
    cutoff = hand_amount * CUTOFF
    start_path = "training_data/card_data"
    rebuild_folder(start_path)
    image_root = f"{start_path}/images"
    build_folder(image_root)
    label_root = f"{start_path}/labels"
    build_folder(label_root)

    for split in ("train", "val"):
        image_path = f"{image_root}/{split}"
        build_folder(image_path)

        label_path = f"{label_root}/{split}"
        build_folder(label_path)

    if hand_amount <= 0:
        return

    worker_amount = min(CPU_COUNT - 1, hand_amount)
    chunks = split_work(hand_amount, worker_amount)
    progress_lock = threading.Lock()

    def process_hands(hand_indices: range, progress: tqdm) -> None:
        for hand_index in hand_indices:
            name, split, hand_render = generate_rendered_hand(hand_index, cutoff)

            img_path = f"{image_root}/{split}/{name}.png"
            label_path = f"{label_root}/{split}/{name}.txt"

            hand_render.image.save(img_path)

            with open(label_path, "w", encoding="utf-8") as t:
                for data in hand_render.annotations:
                    line = " ".join([str(val) for val in data.box])
                    t.write(line + "\n")

            with progress_lock:
                progress.update(1)

    with tqdm(total=hand_amount) as progress:
        with ThreadPoolExecutor(max_workers=worker_amount) as executor:
            list(executor.map(lambda chunk: process_hands(chunk, progress), chunks))



def generate_card_feature_data(train_type: CardFeatureTrainingType, hand_amount: int = 5000) -> None:
    cutoff = hand_amount * CUTOFF
    start_path = f"training_data/{FOLDER_TRAINING_NAMES[train_type]}_data"
    rebuild_folder(start_path)

    for split in ("train", "val"):
        image_path = f"{start_path}/{split}"
        build_folder(image_path)

        for feature in FEATURE_ENUMS[train_type]:
            build_folder(f"{image_path}/{int(feature)}")

    if hand_amount <= 0:
        return

    worker_amount = min(max(1, CPU_COUNT // 2), hand_amount)
    chunks = split_work(hand_amount, worker_amount)

    progress_lock = threading.Lock()

    def process_hands(hand_indices: range, progress: tqdm) -> None:
        thread_id = threading.get_ident()
        count = 0

        hand_indices = list(hand_indices)
        for batch_start in range(0, len(hand_indices), BATCH_SIZE):
            batch_indices = hand_indices[batch_start:batch_start + BATCH_SIZE]
            batch_data = []

            for hand_index in batch_indices:
                _, split, hand_render = generate_rendered_hand(hand_index, cutoff, True)
                batch_data.append((split, hand_render))

            hand_images = [hand_render.image for _, hand_render in batch_data]
            batch_results = BOX_MODEL(hand_images, verbose=False)

            for (split, hand_render), results in zip(batch_data, batch_results):
                hand_image = hand_render.image
                card_locations = get_card_locations_in_hand([results])
                image_path = f"{start_path}/{split}"

                annotations = hand_render.annotations
                if len(card_locations) != len(annotations):
                    continue

                for card_index, annotation in enumerate(annotations):
                    card_data = annotation.card

                    is_stone = card_data.enhancement == Enhancement.STONE
                    if is_stone and train_type != CardFeatureTrainingType.ENHANCEMENT:
                        continue # skipping stones for not enhamcement cards

                    card_image = hand_image.crop((
                        card_locations[card_index][0],
                        card_locations[card_index][1],
                        card_locations[card_index][2],
                        card_locations[card_index][3]
                    ))

                    feature, crop_values = feature_info(train_type, card_image, card_data)
                    feature_label = str(int(feature))
                    feature_image_path = f"{image_path}/{feature_label}"

                    feature_image = f"{feature_image_path}/{thread_id}_{count}_{feature_label}.png"
                    crop_feature = card_image.crop((crop_values))
                    crop_feature.save(feature_image)
                    count += 1

            with progress_lock:
                progress.update(len(batch_data))

    with tqdm(total=hand_amount) as progress:
        with ThreadPoolExecutor(max_workers=worker_amount) as executor:
            list(executor.map(lambda chunk: process_hands(chunk, progress), chunks))



def generate_all_feature_data() -> None:
    for training_type in CardFeatureTrainingType:
        generate_card_feature_data(training_type)



if __name__ == '__main__':
    generate_all_feature_data()
    # generate_card_feature_data(1, CardFeatureTrainingType.RANK)
    # generate_hand_training_data()
