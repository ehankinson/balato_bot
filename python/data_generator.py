import os
import threading

from PIL import Image
from tqdm import tqdm
from concurrent.futures import ThreadPoolExecutor

from card_models import Card, Hand
from render_hand import render_hand
from vision import get_card_locations_in_hand
from card_enums import Rank, Suit, Seal, Enhancement, CardFeatureTrainingType
from util import random_card_amount, card_crop, build_folder, remove_folder, rebuild_folder
from const import (
    BOX_MODEL,
    RANK_CROP,
    SEAL_CROP,
    SUIT_CROP,
    ENHANCEMENT_CROP,
    FOLDER_TRAINING_NAMES
)

CUTOFF = 0.9 # split between training and val
THREAD_WORKERS = os.cpu_count() - 15
BATCH_SIZE = 100

FEATURE_ENUMS = {
    CardFeatureTrainingType.Rank: Rank,
    CardFeatureTrainingType.Suit: Suit,
    CardFeatureTrainingType.Enhancement: Enhancement,
    CardFeatureTrainingType.Seal: Seal,
}



def feature_info(
    train_type: CardFeatureTrainingType, card_image: Image, card: Card
) -> tuple[Rank | Suit | Enhancement | Seal, tuple[int, int, int, int]]:
    w, h = card_image.size
    match train_type:
        case CardFeatureTrainingType.Rank:
            return card.rank, card_crop(w, h, RANK_CROP)

        case CardFeatureTrainingType.Suit:
            return card.suit, card_crop(w, h, SUIT_CROP)

        case CardFeatureTrainingType.Enhancement:
            return card.enhancement, card_crop(w, h, ENHANCEMENT_CROP)

        case CardFeatureTrainingType.Seal:
            return card.seal, card_crop(w, h, SEAL_CROP)



def generate_hand_training_data(hand_amount: int = 5000) -> None:
    cutoff = hand_amount * CUTOFF
    start_path = "training_data/card_data"
    rebuild_folder(start_path)
    image_root = f"{start_path}/images"
    build_folder(image_root)
    label_root = f"{start_path}/labels"
    build_folder(label_root)

    for i in tqdm(range(hand_amount)):
        card_amount = random_card_amount()
        hand = Hand.random_hand(card_amount)
        hand_render = render_hand(hand)

        name = f"{i}_{card_amount}"
        split = "train" if i < cutoff else "val"

        image_path = f"{image_root}/{split}"
        build_folder(image_path)

        label_path = f"{label_root}/{split}"
        build_folder(label_path)

        img_path = f"{image_path}/{name}.png"
        label_path = f"{label_path}/{name}.txt"

        hand_render.image.save(img_path)

        with open(label_path, "w", encoding="utf-8") as t:
            box_values = hand_render.annotations
            for data in box_values:
                line = " ".join([str(val) for val in data.box])
                t.write(line + "\n")



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

    worker_amount = min(THREAD_WORKERS, hand_amount)
    chunk_size, extra = divmod(hand_amount, worker_amount)
    chunks = []
    start = 0
    for worker_index in range(worker_amount):
        end = start + chunk_size + (1 if worker_index < extra else 0)
        chunks.append(range(start, end))
        start = end

    progress_lock = threading.Lock()

    def process_hands(hand_indices: range, progress: tqdm) -> None:
        thread_id = threading.get_ident()
        count = 0

        hand_indices = list(hand_indices)
        for batch_start in range(0, len(hand_indices), BATCH_SIZE):
            batch_indices = hand_indices[batch_start:batch_start + BATCH_SIZE]
            batch_data = []

            for hand_index in batch_indices:
                split = "train" if hand_index < cutoff else "val"

                card_amount = random_card_amount()
                hand = Hand.random_hand(card_amount)
                hand_render = render_hand(hand)
                batch_data.append((split, hand_render))

            hand_images = [hand_render.image for _, hand_render in batch_data]
            batch_results = BOX_MODEL(hand_images, verbose=False)

            for (split, hand_render), results in zip(batch_data, batch_results):
                hand_image = hand_render.image
                card_locations = get_card_locations_in_hand([results])
                image_path = f"{start_path}/{split}"

                annotations = hand_render.annotations
                for card_index, annotation in enumerate(annotations):
                    card_data = annotation.card

                    is_stone = card_data.enhancement == Enhancement.STONE
                    if is_stone and train_type != CardFeatureTrainingType.Enhancement:
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
    # generate_card_feature_data(1, CardFeatureTrainingType.Rank)
    # generate_hand_training_data()
