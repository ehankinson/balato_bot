import os
import random
import sys
import threading
from concurrent.futures import ThreadPoolExecutor
from typing import Any, Callable

from config.settings import (
    EDITION_CROP,
    ENHANCEMENT_CROP,
    FOLDER_TRAINING_NAMES,
    JOKER_EDITION_CROP,
    JOKER_TYPE_CROP,
    RANK_CROP,
    SEAL_CROP,
    SUIT_CROP,
)
from core.class_indices import NEGATIVE_JOKER_EDITION_ID
from core.enums import (
    CardFeatureTrainingType,
    Edition,
    Enhancement,
    JokerFeatureTrainingType,
    JokersName,
    Rank,
    Seal,
    Suit,
)
from core.models import RANDOM_JOKERS, Card, Hand, Joker, RenderedHand
from PIL import Image
from rendering.hand import render_hand
from rendering.joker import render_jokers
from tqdm import tqdm
from utils.files import build_folder, rebuild_folder
from utils.images import card_crop

CUTOFF = 0.9  # split between training and val
CPU_COUNT = os.cpu_count()
BATCH_SIZE = 10
CropBox = tuple[int | float, int | float, int | float, int | float]

CARD_FEATURE_ENUMS = {
    CardFeatureTrainingType.RANK: Rank,
    CardFeatureTrainingType.SUIT: Suit,
    CardFeatureTrainingType.ENHANCEMENT: Enhancement,
    CardFeatureTrainingType.SEAL: Seal,
    CardFeatureTrainingType.EDITION: Edition,
}

JOKER_FEATURE_ENUMS = {
    JokerFeatureTrainingType.JOKER_TYPE: RANDOM_JOKERS,
    JokerFeatureTrainingType.JOKER_EDITION: list(Edition) + [NEGATIVE_JOKER_EDITION_ID],
}


def random_full_card_amount() -> int:
    return random.choices(
        population=[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18],
        weights=[1, 1, 2, 3, 5, 8, 16, 50, 33, 18, 7, 4, 1, 1, 1, 1, 1, 1],
        k=1,
    )[0]


def random_feature_card_amount() -> int:
    return random.choices(
        population=[6, 7, 8, 9, 10, 11], weights=[8, 16, 50, 33, 18, 7], k=1
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


def generate_rendered_hand(
    hand_index: int, cutoff: float, is_feature: bool = False
) -> tuple[str, str, RenderedHand]:
    card_amount = (
        random_full_card_amount() if not is_feature else random_feature_card_amount()
    )
    hand = Hand.random_hand(card_amount)
    hand_render = render_hand(hand, True)

    name = f"{hand_index}_{card_amount}"
    split = "train" if hand_index < cutoff else "val"
    return name, split, hand_render


def generate_rendered_jokers(
    hand_index: int, cutoff: float, jokers: list[Joker] = []
) -> tuple[str, str, RenderedHand]:
    if len(jokers) == 0:
        joker_amount = random.randint(1, 9)
        jokers = [Joker.random() for _ in range(joker_amount)]

    jokers_render = render_jokers(jokers, True)

    name = f"{hand_index}_{len(jokers)}"
    split = "train" if hand_index < cutoff else "val"
    return name, split, jokers_render


def generate_joker_type_list(amount: int) -> list[list[Joker]]:
    training_data: list[list[Joker]] = []
    for joker in RANDOM_JOKERS:
        for i in range(amount):
            joker_count = (i % 9) + 1
            jokers = [Joker.random() for _ in range(joker_count)]

            random_joker = random.choice(jokers)
            random_joker.background_image = joker

            training_data.append(jokers)

    random.shuffle(training_data)
    return training_data


def build_folders(start_path: str, features: list[Any]) -> None:
    rebuild_folder(start_path)

    for split in ("train", "val"):
        image_path = f"{start_path}/{split}"
        build_folder(image_path)

        for feature in features:
            build_folder(f"{image_path}/{int(feature)}")


def build_balanced_joker_type_schedule(
    samples_per_joker: int,
) -> list[tuple[JokersName, int, str]]:
    schedule: list[tuple[JokersName, int, str]] = []
    train_amount = round(samples_per_joker * CUTOFF)

    for joker in RANDOM_JOKERS:
        for sample_index in range(samples_per_joker):
            joker_amount = sample_index % 9 + 1
            split = "train" if sample_index < train_amount else "val"
            schedule.append((joker, joker_amount, split))

    random.shuffle(schedule)
    return schedule


def generate_targeted_rendered_jokers(
    sample_index: int,
    cutoff: float,
    schedule: list[tuple[JokersName, int, str]],
) -> tuple[str, str, RenderedHand]:
    target_joker, joker_amount, split = schedule[sample_index]
    jokers = [Joker(target_joker)]
    random_joker_pool = [joker for joker in RANDOM_JOKERS if joker != target_joker]
    jokers.extend(
        Joker(random.choice(random_joker_pool)) for _ in range(joker_amount - 1)
    )
    random.shuffle(jokers)

    jokers_render = render_jokers(jokers, True)
    name = f"{sample_index}_{int(target_joker)}_{joker_amount}"
    return name, split, jokers_render


def card_feature_info(
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


def joker_feature_info(
    train_type: JokerFeatureTrainingType, joker_image: Image.Image, joker: Joker
) -> tuple[JokersName | Edition | int, CropBox]:
    w, h = joker_image.size

    match train_type:
        case JokerFeatureTrainingType.JOKER_TYPE:
            return joker.background_image, card_crop(w, h, JOKER_TYPE_CROP)

        case JokerFeatureTrainingType.JOKER_EDITION:
            return (
                NEGATIVE_JOKER_EDITION_ID if joker.negative else joker.edition,
                card_crop(w, h, JOKER_EDITION_CROP),
            )


def yolo_box_to_crop(box: list[float], image: Image.Image) -> tuple[int, int, int, int]:
    _, center_x, center_y, width, height = box
    image_width, image_height = image.size

    box_width = width * image_width
    box_height = height * image_height
    left = round(center_x * image_width - box_width / 2)
    top = round(center_y * image_height - box_height / 2)
    right = round(center_x * image_width + box_width / 2)
    bottom = round(center_y * image_height + box_height / 2)

    left = max(0, min(left, image_width - 1))
    top = max(0, min(top, image_height - 1))
    right = max(left + 1, min(right, image_width))
    bottom = max(top + 1, min(bottom, image_height))

    return left, top, right, bottom


def generate_box_training_data(
    size: int,
    data_type: str,
    function: Callable[..., tuple[str, str, RenderedHand]],
    is_feature: bool = False,
) -> None:
    cutoff = size * CUTOFF

    start_path = f"training_data/{data_type}"
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

    with tqdm(total=size) as progress:
        with ThreadPoolExecutor(max_workers=worker_amount) as executor:
            list(executor.map(lambda chunk: process_hands(chunk, progress), chunks))


def generate_hand_training_data(hand_amount: int = 5000) -> None:
    generate_box_training_data(hand_amount, "hand_data", generate_rendered_hand)


def generate_joker_training_data(joker_amount: int = 5000) -> None:
    generate_box_training_data(joker_amount, "joker_data", generate_rendered_jokers)


def generate_feature_data(
    train_type: Any,
    amount: int,
    start_path: str,
    features: list[Any],
    render_function: Callable[..., tuple[str, str, RenderedHand]],
    feature_function: Callable[[Any, Image.Image, Any], tuple[Any, CropBox]],
    skip_function: Callable[[Any], bool] | None = None,
    special_data: list[Any] | None = None,
) -> None:
    amount = amount if special_data is None else len(special_data)
    cutoff = amount * CUTOFF
    build_folders(start_path, features)

    if amount <= 0:
        return

    cpu_count = CPU_COUNT if CPU_COUNT is not None else 1
    worker_amount = min(max(1, cpu_count - 1), amount)
    chunks = split_work(amount, worker_amount)

    progress_lock = threading.Lock()

    def process_items(item_indices: range, progress: tqdm) -> None:
        thread_id = threading.get_ident()
        count = 0

        for sample_index in item_indices:
            if special_data is not None:
                _, split, rendered_data = render_function(
                    sample_index, cutoff, special_data[sample_index]
                )
            else:
                _, split, rendered_data = render_function(sample_index, cutoff)

            image = rendered_data.image
            item_locations = [
                yolo_box_to_crop(annotation.box, image)
                for annotation in rendered_data.annotations
            ]
            image_path = f"{start_path}/{split}"

            annotations = rendered_data.annotations
            if len(item_locations) != len(annotations):
                with progress_lock:
                    progress.update(1)
                continue

            for item_index, annotation in enumerate(annotations):
                item_data = annotation.card

                if skip_function is not None and skip_function(item_data):
                    continue

                item_image = image.crop(
                    (
                        item_locations[item_index][0],
                        item_locations[item_index][1],
                        item_locations[item_index][2],
                        item_locations[item_index][3],
                    )
                )

                feature, crop_values = feature_function(
                    train_type, item_image, item_data
                )
                feature_label = str(int(feature))
                feature_image_path = f"{image_path}/{feature_label}"

                feature_image = (
                    f"{feature_image_path}/{thread_id}_{count}_{feature_label}.png"
                )

                crop_feature = item_image.crop((crop_values))
                crop_feature.save(feature_image)
                count += 1

            with progress_lock:
                progress.update(1)

    with tqdm(total=amount) as progress:
        with ThreadPoolExecutor(max_workers=worker_amount) as executor:
            list(executor.map(lambda chunk: process_items(chunk, progress), chunks))


def generate_card_feature_data(
    train_type: CardFeatureTrainingType, hand_amount: int = 5000
) -> None:
    def should_skip_card(card: Card) -> bool:
        return (
            card.enhancement == Enhancement.STONE
            and train_type != CardFeatureTrainingType.ENHANCEMENT
        )

    generate_feature_data(
        train_type=train_type,
        amount=hand_amount,
        start_path=f"training_data/{FOLDER_TRAINING_NAMES[train_type]}_data",
        features=list(CARD_FEATURE_ENUMS[train_type]),
        render_function=lambda item_index, cutoff: generate_rendered_hand(
            item_index, cutoff, True
        ),
        feature_function=card_feature_info,
        skip_function=should_skip_card,
    )


def generate_joker_feature_data(
    train_type: JokerFeatureTrainingType, jokers_amount: int = 5000
) -> None:
    jokers = (
        generate_joker_type_list(jokers_amount)
        if train_type == JokerFeatureTrainingType.JOKER_TYPE
        else None
    )

    generate_feature_data(
        train_type=train_type,
        amount=jokers_amount,
        start_path=f"training_data/{train_type.name.lower()}_data",
        features=list(JOKER_FEATURE_ENUMS[train_type]),
        render_function=generate_rendered_jokers,
        feature_function=joker_feature_info,
        special_data=jokers,
    )


def generate_all_feature_data() -> None:
    for training_type in CardFeatureTrainingType:
        generate_card_feature_data(training_type)


def generate_all_joker_feature_data() -> None:
    for training_type in JokerFeatureTrainingType:
        joker_amount = (
            5000 if training_type == JokerFeatureTrainingType.JOKER_EDITION else 150
        )
        generate_joker_feature_data(training_type, joker_amount)


if __name__ == "__main__":
    available_commands = {
        "all_card_features": {"function": generate_all_feature_data},
        "card_enhancement": {
            "function": generate_card_feature_data,
            "args": [CardFeatureTrainingType.ENHANCEMENT],
        },
        "card_edition": {
            "function": generate_card_feature_data,
            "args": [CardFeatureTrainingType.EDITION],
        },
        "card_rank": {
            "function": generate_card_feature_data,
            "args": [CardFeatureTrainingType.RANK],
        },
        "card_suit": {
            "function": generate_card_feature_data,
            "args": [CardFeatureTrainingType.SUIT],
        },
        "card_seal": {
            "function": generate_card_feature_data,
            "args": [CardFeatureTrainingType.SEAL],
        },
        "playing_hands": {"function": generate_hand_training_data},
        "jokers": {"function": generate_joker_training_data},
        "all_joker_features": {"function": generate_all_joker_feature_data},
        "joker_type": {
            "function": generate_joker_feature_data,
            "args": [JokerFeatureTrainingType.JOKER_TYPE, 150],
        },
        "joker_edition": {
            "function": generate_joker_feature_data,
            "args": [JokerFeatureTrainingType.JOKER_EDITION],
        },
    }

    if len(sys.argv) < 2 or sys.argv[1] not in available_commands:
        print("Sorry that command is invalid please add 1 of the following:")
        for key in available_commands.keys():
            print(key)

        exit()

    command = sys.argv[1]
    function = available_commands[command]["function"]
    args = available_commands[command].get("args", [])
    function(*args)
