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
    JOKER_EDITION_CROP,
    JOKER_TYPE_CROP,
    RANK_CROP,
    ROOT_DIR,
    SEAL_CROP,
    SUIT_CROP,
)
from core.class_indices import NEGATIVE_JOKER_EDITION_ID
from core.enums import (
    Edition,
    Enhancement,
    JokerFeatureTrainingType,
    JokersName,
    Planet,
    Rank,
    Seal,
    Spectral,
    Suit,
    Tarot,
)
from core.models import RANDOM_JOKERS, Hand, Joker, JokerReq, RenderedHand
from rendering.consumable import (
    Consumable,
    generate_consumables,
    render_consumables,
)
from rendering.hand import generate_hand, render_hand
from rendering.joker import render_jokers
from utils.files import build_folder, rebuild_folder
from utils.images import card_crop, yolo_box_to_crop

CUTOFF = 0.9  # split between training and val
CPU_COUNT = int(os.cpu_count() if os.cpu_count() is not None else 1)
CropBox = tuple[int | float, int | float, int | float, int | float]

FEATURES = {
    "rank": Rank,
    "suit": Suit,
    "enhancement": Enhancement,
    "edition": Edition,
    "seal": Seal,
    "tarot": Tarot,
    "planet": Planet,
    "spectral": Spectral,
}

JOKER_FEATURE_ENUMS = {
    JokerFeatureTrainingType.JOKER_TYPE: RANDOM_JOKERS,
    JokerFeatureTrainingType.JOKER_EDITION: list(Edition) + [NEGATIVE_JOKER_EDITION_ID],
}

type Feature = Rank | Suit | Enhancement | Edition | Seal | Tarot | Planet | Spectral


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
    hand_index: int, cutoff: float, jokers: list[Joker] | None = None
) -> tuple[str, str, RenderedHand]:
    if not jokers:
        joker_amount = random.randint(1, 9)
        jokers = [generate_random_joker() for _ in range(joker_amount)]

    jokers_render = render_jokers(jokers, True)

    name = f"{hand_index}_{len(jokers)}"
    split = "train" if hand_index < cutoff else "val"
    return name, split, jokers_render


def generate_rendered_consumables(
    hand_index: int, cutoff: float
) -> tuple[str, str, RenderedHand]:
    consumables_amount = random.randint(1, 4)
    consumables = [generate_random_consumable() for _ in range(consumables_amount)]
    consumable_render = render_consumables(consumables, training=True)

    name = f"{hand_index}_{consumables_amount}"
    split = "train" if hand_index < cutoff else "val"
    return name, split, consumable_render


def generate_random_joker(joker_name: JokersName | None = None) -> Joker:
    """Build a visually randomized Joker, optionally with a required identity."""
    return Joker(
        background_image=(
            joker_name if joker_name is not None else random.choice(RANDOM_JOKERS)
        ),
        face_image=None,
        negative=random.choice([True, False]),
        edition=random.choice(list(Edition)),
        req=JokerReq(),
        copyable=False,
    )


def generate_random_consumable() -> Consumable:
    rng = random.randint(1, 3)
    val = None
    if rng == 1:
        val = random.choice(list(Tarot))
    elif rng == 2:
        val = random.choice(list(Planet))
    else:
        val = random.choice(list(Spectral))

    return val


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
    training_amount: int,
    item_amounts: tuple[int, int],
    render_function: Callable[[int, str], RenderedHand],
    command: str,
) -> list[tuple[str, RenderedHand]]:
    """
    training_amount: Is the number of times we will call the render function
    per_image_amount: Is the number of items within those images, like cards/jokers...
    render_function: A function that takes in a list of items, and then generates the image
    """
    worker_amount = 4
    chunks = split_work(training_amount, worker_amount)

    schedule: list[tuple[str, RenderedHand]] = [
        ("", RenderedHand(None, []))
    ] * training_amount
    val_cutoff = training_amount * CUTOFF

    progress_lock = threading.Lock()

    def render_images(chunk: range, progress: tqdm):
        for index in chunk:
            rng = random.randint(item_amounts[0], item_amounts[1])
            rendered_data = render_function(rng, command)
            split = "train" if index < val_cutoff else "val"

            schedule[index] = (split, rendered_data)

            with progress_lock:
                progress.update(1)

    with (
        tqdm(total=training_amount) as progress,
        ThreadPoolExecutor(max_workers=worker_amount) as executor,
    ):
        list(executor.map(lambda chunk: render_images(chunk, progress), chunks))

    random.shuffle(schedule)
    return schedule


def generate_targeted_rendered_jokers(
    sample_index: int,
    _cutoff: float,
    sample: tuple[JokersName, int, str],
) -> tuple[str, str, RenderedHand]:
    target_joker, joker_amount, split = sample
    jokers = [generate_random_joker(target_joker)]
    random_joker_pool = [joker for joker in RANDOM_JOKERS if joker != target_joker]
    jokers.extend(
        generate_random_joker(random.choice(random_joker_pool))
        for _ in range(joker_amount - 1)
    )
    random.shuffle(jokers)

    jokers_render = render_jokers(jokers, True)
    name = f"{sample_index}_{int(target_joker)}_{joker_amount}"
    return name, split, jokers_render


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

    return card_crop(w, h, crop_values)


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
    schedule: list[tuple[str, RenderedHand]],
    features: list[Feature],
):
    build_folders(start_path, features)
    worker_amount = CPU_COUNT - 1
    chunks = split_work(amount, worker_amount)

    progress_lock = threading.Lock()

    def process_items(item_indices: range, progress: tqdm, feature: Feature) -> None:
        for index in item_indices:
            file_location, render_data = schedule[index]
            image = render_data.image
            for count, card_annotation in enumerate(render_data.annotations):
                item, box = card_annotation.card, card_annotation.box
                item_image = image.crop(yolo_box_to_crop(box, image))
                feature_image = item_image.crop(feature_crop(feature, item_image))
                final_path = os.path.join(
                    start_path, file_location, str(item.value), f"{index}_{count}.png"
                )
                feature_image.save(final_path)

            with progress_lock:
                progress.update(1)

    feature = features[-1]

    with (
        tqdm(total=amount) as progress,
        ThreadPoolExecutor(max_workers=worker_amount) as executor,
    ):
        list(
            executor.map(lambda chunk: process_items(chunk, progress, feature), chunks)
        )


def setup(
    command: str,
    start_path: str,
    render_amount: tuple[int, int],
    render_function: Callable[[int, str], RenderedHand],
):
    training_amount = 5_000
    start_path = os.path.join(start_path, f"{command}_data")
    features = list(FEATURES[command])
    schedule = build_schedule(training_amount, render_amount, render_function, command)
    generate_feature_data(start_path, training_amount, schedule, features)


if __name__ == "__main__":
    available_commands = [
        "all_card_features",
        "enhancement",
        "edition",
        "rank",
        "suit",
        "seal",
        "card_locations",
        "joker_locations",
        "consumable_locations",
        "all_joker_features",
        "joker_type",
        "joker_edition",
        "all_consumables",
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

    start_path = os.path.join(ROOT_DIR, "training_data")
    render_amount = (0, 0)
    render_function = None

    if command == "all_card_features":
        render_amount = (6, 16)
        for feature in ["enhancement", "edition", "rank", "suit", "seal"]:
            setup(feature, start_path, render_amount, generate_hand)

    elif command == "all_consumables":
        render_amount = (1, 4)
        for feature in ["tarot", "planet", "spectral"]:
            setup(feature, start_path, render_amount, generate_consumables)

    else:
        if command in ["enhancement", "edition", "rank", "suit", "seal"]:
            render_amount = (6, 16)
            render_function = generate_hand

        elif command in ["tarot", "planet", "spectral"]:
            render_amount = (1, 4)
            render_function = generate_consumables

        setup(command, start_path, render_amount, render_function)
