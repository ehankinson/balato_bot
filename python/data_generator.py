import os
import random
import sys
import threading
from collections.abc import Callable
from concurrent.futures import ThreadPoolExecutor

from PIL import Image
from tqdm import tqdm

from config.settings import (
    CONSUMABLE_CROP,
    EDITION_CROP,
    ENHANCEMENT_CROP,
    JOKER_NAME_CROP,
    RANK_CROP,
    ROOT_DIR,
    SEAL_CROP,
    SUIT_CROP,
)
from core.class_indices import NEGATIVE_JOKER_EDITION_ID
from core.enums import (
    Edition,
    Enhancement,
    JokerEdition,
    JokerFeatureTrainingType,
    JokersName,
    Planet,
    Rank,
    Seal,
    Spectral,
    Suit,
    Tarot,
)
from core.models import RANDOM_JOKERS, RenderedHand
from core.type_aliases import Feature
from rendering.consumable import (
    generate_consumables,
)
from rendering.hand import generate_hand
from rendering.joker import generate_jokers
from utils.files import build_folder, rebuild_folder
from utils.images import card_crop, yolo_box_to_crop

CUTOFF = 0.9  # split between training and val
CPU_COUNT = int(os.cpu_count() if os.cpu_count() is not None else 1)
CropBox = tuple[int | float, int | float, int | float, int | float]


FEATURES: dict[str, type[Feature]] = {
    "rank": Rank,
    "suit": Suit,
    "enhancement": Enhancement,
    "edition": Edition,
    "seal": Seal,
    "tarot": Tarot,
    "planet": Planet,
    "spectral": Spectral,
    "joker_name": JokersName,
    "joker_edition": JokerEdition,
}

JOKER_FEATURE_ENUMS = {
    JokerFeatureTrainingType.JOKER_TYPE: RANDOM_JOKERS,
    JokerFeatureTrainingType.JOKER_EDITION: list(Edition) + [NEGATIVE_JOKER_EDITION_ID],
}


def split_work(total_amount: int, worker_amount: int) -> list[range]:
    chunk_size, extra = divmod(total_amount, worker_amount)
    chunks: list[range] = []
    start = 0
    for worker_index in range(worker_amount):
        end = start + chunk_size + (1 if worker_index < extra else 0)
        chunks.append(range(start, end))
        start = end

    return chunks


def build_folders(
    start_path: str,
    features: list[Feature],
) -> None:
    rebuild_folder(start_path)

    for split in ("train", "val"):
        image_path = f"{start_path}/{split}"
        build_folder(image_path)

        for feature in features:
            build_folder(f"{image_path}/{int(feature)}")


def build_schedule(
    training_amount: int, features: list[Feature], render_amount: tuple[int, int]
) -> list[tuple[Feature, int, str]]:
    schedule: list[tuple[Feature, int, str]] = []
    chunk_size, left_over = divmod(training_amount, len(features))

    for feature in features:
        iter_amount = chunk_size + 1 if left_over > 0 else chunk_size
        left_over -= 1

        for count in range(iter_amount):
            amount = random.randint(render_amount[0], render_amount[1])
            location = "train" if count < iter_amount * CUTOFF else "val"
            schedule.append((feature, amount, location))

    return schedule


def feature_crop(feature: Feature, img: Image.Image) -> tuple[int, int, int, int]:
    w, h = img.size
    crop_values = []
    match feature:
        case Rank():
            crop_values = RANK_CROP

        case Suit():
            crop_values = SUIT_CROP

        case Enhancement():
            crop_values = ENHANCEMENT_CROP

        case Seal():
            crop_values = SEAL_CROP

        case Edition():
            crop_values = EDITION_CROP

        case Tarot():
            crop_values = CONSUMABLE_CROP

        case Spectral():
            crop_values = CONSUMABLE_CROP

        case Planet():
            crop_values = CONSUMABLE_CROP

        case JokersName():
            crop_values = JOKER_NAME_CROP

        case JokerEdition():
            crop_values = JOKER_NAME_CROP

    return card_crop(w, h, crop_values)


def generate_location_training_data(
    size: int,
    data_type: str,
    function: Callable[..., tuple[str, str, RenderedHand]],
    is_feature: bool = False,
) -> None:
    cutoff = size * CUTOFF

    start_path = os.path.join(ROOT_DIR, "training_data", data_type)
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

    if size <= 0:
        return

    cpu_threads = CPU_COUNT if CPU_COUNT is not None else 2
    worker_amount = min(cpu_threads - 1, size)
    chunks = split_work(size, worker_amount)
    progress_lock = threading.Lock()
    args = [0, cutoff, is_feature] if is_feature else [0, cutoff]

    def process_hands(hand_indices: range, progress: tqdm) -> None:
        for hand_index in hand_indices:
            args[0] = hand_index
            name, split, hand_render = function(*args)

            img_path = f"{image_root}/{split}/{name}.png"
            label_path = f"{label_root}/{split}/{name}.txt"

            hand_render.image.save(img_path)

            with open(label_path, "w", encoding="utf-8") as t:
                for data in hand_render.annotations:
                    line = " ".join([str(val) for val in data.box])
                    t.write(line + "\n")

            with progress_lock:
                progress.update(1)

    with (
        tqdm(total=size) as progress,
        ThreadPoolExecutor(max_workers=worker_amount) as executor,
    ):
        list(executor.map(lambda chunk: process_hands(chunk, progress), chunks))


def generate_feature_data(
    start_path: str,
    amount: int,
    render_function: Callable[[int, Feature], RenderedHand],
    features: list[Feature],
    schedule: list[tuple[Feature, int, str]],
):
    build_folders(start_path, features)
    # worker_amount = max(1, CPU_COUNT - 1)
    worker_amount = 4
    chunks = split_work(amount, worker_amount)
    progress_lock = threading.Lock()

    def process_items(item_indices: range, progress: tqdm) -> None:
        for index in item_indices:
            feature, amount, item_location = schedule[index]
            render_data = render_function(amount, feature)
            image = render_data.image
            for count, card_annotation in enumerate(render_data.annotations):
                item, box = card_annotation.card, card_annotation.box
                item_image = image.crop(yolo_box_to_crop(box, image))
                feature_image = item_image.crop(feature_crop(feature, item_image))
                final_path = os.path.join(
                    start_path, item_location, str(item.value), f"{count}_{index}.png"
                )
                feature_image.save(final_path)

            with progress_lock:
                progress.update(1)

    with (
        tqdm(total=amount) as progress,
        ThreadPoolExecutor(max_workers=worker_amount) as executor,
    ):
        list(executor.map(lambda chunk: process_items(chunk, progress), chunks))


def setup(
    training_amount: int,
    command: str,
    start_path: str,
    render_amount: tuple[int, int],
    render_function: Callable[[int, Feature], RenderedHand],
):
    start_path = os.path.join(start_path, f"{command}_data")
    features = list(FEATURES[command])
    schedule = build_schedule(training_amount, features, render_amount)
    generate_feature_data(
        start_path, training_amount, render_function, features, schedule
    )


if __name__ == "__main__":
    available_commands = [
        "all_card_features",
        "all_consumables",
        "all_jokers",
        "enhancement",
        "edition",
        "rank",
        "suit",
        "seal",
        "card_locations",
        "joker_locations",
        "consumable_locations",
        "joker_name",
        "joker_edition",
        "tarot",
        "planet",
        "spectral",
    ]

    if len(sys.argv) < 2:
        print("Please pass in 1 of these arguemnts")
        for val in available_commands:
            print(val)
        sys.exit()

    command = sys.argv[1]
    if command not in available_commands:
        print("Sorry that command is invalid please use 1 of the following:")
        for val in available_commands:
            print(val)
        sys.exit()

    training_amount = 5_000
    start_path = os.path.join(ROOT_DIR, "training_data")
    render_amount = (0, 0)
    render_function = None

    if command == "all_card_features":
        render_amount = (6, 16)
        for feature in ["enhancement", "edition", "rank", "suit", "seal"]:
            setup(training_amount, feature, start_path, render_amount, generate_hand)

    elif command == "all_consumables":
        render_amount = (1, 4)
        for feature in ["tarot", "planet", "spectral"]:
            setup(
                training_amount,
                feature,
                start_path,
                render_amount,
                generate_consumables,
            )

    elif command == "all_jokers":
        render_amount = (1, 9)
        for feature in ["joker_name", "joker_edition"]:
            setup(training_amount, feature, start_path, render_amount, generate_jokers)

    else:
        if command in ["enhancement", "edition", "rank", "suit", "seal"]:
            render_amount = (6, 16)
            render_function = generate_hand

        elif command in ["tarot", "planet", "spectral"]:
            render_amount = (1, 4)
            render_function = generate_consumables

        elif command in ["joker_name", "joker_edition"]:
            render_amount = (1, 9)
            render_function = generate_jokers

        assert render_function is not None
        setup(training_amount, command, start_path, render_amount, render_function)
