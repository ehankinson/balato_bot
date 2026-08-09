import os
import random
import sys
import threading
from concurrent.futures import ThreadPoolExecutor
from typing import Any, Callable

from PIL import Image
from tqdm import tqdm

from config.settings import (
    EDITION_CROP,
    ENHANCEMENT_CROP,
    FOLDER_TRAINING_NAMES,
    JOKER_EDITION_CROP,
    JOKER_TYPE_CROP,
    RANK_CROP,
    ROOT_DIR,
    SEAL_CROP,
    SUIT_CROP,
)
from core.class_indices import NEGATIVE_JOKER_EDITION_ID
from core.enums import (
    CardFeatureTrainingType,
    Consumables,
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
from core.models import RANDOM_JOKERS, Card, Hand, Joker, JokerReq, RenderedHand
from rendering.consumable import Consumable, render_consumables
from rendering.hand import render_hand
from rendering.joker import render_jokers
from utils.files import build_folder, rebuild_folder
from utils.images import card_crop

CUTOFF = 0.9  # split between training and val
CPU_COUNT = os.cpu_count()
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

CONSUMABLE_FEATURE_ENUMS = {
    Consumables.TAROT: Tarot,
    Consumables.PLANET: Planet,
    Consumables.SPECTRAL: Spectral,
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
    hand_index: int, cutoff: float, jokers: list[Joker] | None = None
) -> tuple[str, str, RenderedHand]:
    if not jokers:
        joker_amount = random.randint(1, 9)
        jokers = [generate_random_joker() for _ in range(joker_amount)]

    jokers_render = render_jokers(jokers, True)

    name = f"{hand_index}_{len(jokers)}"
    split = "train" if hand_index < cutoff else "val"
    return name, split, jokers_render


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


def generate_rendered_consumables(
    sample_index: int,
    cutoff: float,
    sample: Consumable | tuple[Consumable, str],
) -> tuple[str, str, RenderedHand]:
    """Render one consumable sample on a training background."""
    if isinstance(sample, tuple):
        consumable, split = sample
    else:
        consumable = sample
        split = "train" if sample_index < cutoff else "val"

    rendered_consumables = render_consumables([consumable], True)
    name = f"{sample_index}_{type(consumable).__name__.lower()}_{int(consumable)}"
    return name, split, rendered_consumables


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


def build_balanced_consumable_schedule(
    train_type: Consumables, sample_amount: int
) -> list[tuple[Consumable, str]]:
    """Distribute a total sample count evenly across consumable classes."""
    if sample_amount < 0:
        raise ValueError("sample_amount cannot be negative")

    consumables = list(CONSUMABLE_FEATURE_ENUMS[train_type])
    samples_per_class, extra_samples = divmod(sample_amount, len(consumables))
    schedule: list[tuple[Consumable, str]] = []

    for class_index, consumable in enumerate(consumables):
        class_amount = samples_per_class + (class_index < extra_samples)
        train_amount = round(class_amount * CUTOFF)
        if class_amount >= 2:
            train_amount = min(class_amount - 1, max(1, train_amount))

        for sample_index in range(class_amount):
            split = "train" if sample_index < train_amount else "val"
            schedule.append((consumable, split))

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


def render_feature_sample(
    sample_index: int,
    cutoff: float,
    render_function: Callable[..., tuple[str, str, RenderedHand]],
    special_data: list[Any] | None,
) -> tuple[str, str, RenderedHand]:
    """Render one scene, optionally using prebuilt data for that sample."""
    if special_data is None:
        return render_function(sample_index, cutoff)

    return render_function(sample_index, cutoff, special_data[sample_index])


def crop_annotated_item(image: Image.Image, box: list[float]) -> Image.Image:
    """Crop one complete card, Joker, or consumable from a rendered scene."""
    return image.crop(yolo_box_to_crop(box, image))


def extract_feature_image(
    train_type: Any,
    item_image: Image.Image,
    item_data: Any,
    feature_function: Callable[[Any, Image.Image, Any], tuple[Any, CropBox]] | None,
) -> tuple[Any, Image.Image]:
    """Return the class label and optional smaller feature crop for one item."""
    if feature_function is None:
        return item_data, item_image

    feature, crop_box = feature_function(train_type, item_image, item_data)
    return feature, item_image.crop(crop_box)


def save_feature_image(
    image: Image.Image,
    start_path: str,
    split: str,
    feature: Any,
    sample_name: str,
    item_index: int,
) -> None:
    """Save a prepared feature image into its train/val class folder."""
    feature_label = str(int(feature))
    filename = (
        f"{threading.get_ident()}_{sample_name}_{item_index}_{feature_label}.png"
    )
    image.save(os.path.join(start_path, split, feature_label, filename))


def process_rendered_features(
    train_type: Any,
    start_path: str,
    sample_name: str,
    split: str,
    rendered_data: RenderedHand,
    feature_function: Callable[[Any, Image.Image, Any], tuple[Any, CropBox]] | None,
    skip_function: Callable[[Any], bool] | None,
) -> None:
    """Crop, optionally refine, and save every annotated item in one scene."""
    for item_index, annotation in enumerate(rendered_data.annotations):
        item_data = annotation.card
        if skip_function is not None and skip_function(item_data):
            continue

        item_image = crop_annotated_item(rendered_data.image, annotation.box)
        feature, feature_image = extract_feature_image(
            train_type, item_image, item_data, feature_function
        )
        save_feature_image(
            feature_image,
            start_path,
            split,
            feature,
            sample_name,
            item_index,
        )


def _generate_feature_dataset(
    train_type: Any,
    amount: int,
    start_path: str,
    features: list[Any],
    render_function: Callable[..., tuple[str, str, RenderedHand]],
    feature_function: Callable[[Any, Image.Image, Any], tuple[Any, CropBox]]
    | None = None,
    skip_function: Callable[[Any], bool] | None = None,
    special_data: list[Any] | None = None,
) -> None:
    """Build folders, then render, crop, optionally refine, and save each scene."""
    amount = amount if special_data is None else len(special_data)
    cutoff = amount * CUTOFF
    build_folders(start_path, features)

    if amount <= 0:
        return

    cpu_count = CPU_COUNT if CPU_COUNT is not None else 1
    worker_amount = min(max(1, cpu_count // 2), amount)
    # worker_amount = 1
    chunks = split_work(amount, worker_amount)

    progress_lock = threading.Lock()

    def process_items(item_indices: range, progress: tqdm) -> None:
        for sample_index in item_indices:
            name, split, rendered_data = render_feature_sample(
                sample_index, cutoff, render_function, special_data
            )
            process_rendered_features(
                train_type,
                start_path,
                name,
                split,
                rendered_data,
                feature_function,
                skip_function,
            )

            with progress_lock:
                progress.update(1)

    with tqdm(total=amount) as progress:
        with ThreadPoolExecutor(max_workers=worker_amount) as executor:
            list(executor.map(lambda chunk: process_items(chunk, progress), chunks))


def generate_feature_data(
    train_type: CardFeatureTrainingType | JokerFeatureTrainingType | Consumables,
    amount: int = 5000,
) -> None:
    """Generate the requested card, Joker, or consumable feature dataset."""
    if isinstance(train_type, CardFeatureTrainingType):

        def should_skip_card(card: Card) -> bool:
            return (
                card.enhancement == Enhancement.STONE
                and train_type != CardFeatureTrainingType.ENHANCEMENT
            )

        _generate_feature_dataset(
            train_type=train_type,
            amount=amount,
            start_path=(
                f"{ROOT_DIR}/training_data/{FOLDER_TRAINING_NAMES[train_type]}_data"
            ),
            features=list(CARD_FEATURE_ENUMS[train_type]),
            render_function=lambda item_index, cutoff: generate_rendered_hand(
                item_index, cutoff, True
            ),
            feature_function=card_feature_info,
            skip_function=should_skip_card,
        )
        return

    if isinstance(train_type, JokerFeatureTrainingType):
        schedule = (
            build_balanced_joker_type_schedule(amount)
            if train_type == JokerFeatureTrainingType.JOKER_TYPE
            else None
        )
        render_function = (
            generate_targeted_rendered_jokers
            if schedule is not None
            else generate_rendered_jokers
        )

        _generate_feature_dataset(
            train_type=train_type,
            amount=amount,
            start_path=f"{ROOT_DIR}/training_data/{train_type.name.lower()}_data",
            features=list(JOKER_FEATURE_ENUMS[train_type]),
            render_function=render_function,
            feature_function=joker_feature_info,
            special_data=schedule,
        )
        return

    if isinstance(train_type, Consumables):
        schedule = build_balanced_consumable_schedule(train_type, amount)
        _generate_feature_dataset(
            train_type=train_type,
            amount=amount,
            start_path=os.path.join(
                ROOT_DIR, "training_data", f"{train_type.name.lower()}_data"
            ),
            features=list(CONSUMABLE_FEATURE_ENUMS[train_type]),
            render_function=generate_rendered_consumables,
            special_data=schedule,
        )
        return


def generate_all_feature_data() -> None:
    for training_type in CardFeatureTrainingType:
        generate_feature_data(training_type)


def generate_all_joker_feature_data() -> None:
    for training_type in JokerFeatureTrainingType:
        joker_amount = (
            5000 if training_type == JokerFeatureTrainingType.JOKER_EDITION else 150
        )
        generate_feature_data(training_type, joker_amount)


def generate_all_consumable_feature_data(consumable_amount: int = 5000) -> None:
    for training_type in Consumables:
        generate_feature_data(training_type, consumable_amount)


if __name__ == "__main__":
    available_commands = {
        "all_card_features": {"function": generate_all_feature_data},
        "card_enhancement": {
            "function": generate_feature_data,
            "args": [CardFeatureTrainingType.ENHANCEMENT],
        },
        "card_edition": {
            "function": generate_feature_data,
            "args": [CardFeatureTrainingType.EDITION],
        },
        "card_rank": {
            "function": generate_feature_data,
            "args": [CardFeatureTrainingType.RANK],
        },
        "card_suit": {
            "function": generate_feature_data,
            "args": [CardFeatureTrainingType.SUIT],
        },
        "card_seal": {
            "function": generate_feature_data,
            "args": [CardFeatureTrainingType.SEAL],
        },
        "playing_hands": {"function": generate_hand_training_data},
        "jokers": {"function": generate_joker_training_data},
        "all_joker_features": {"function": generate_all_joker_feature_data},
        "joker_type": {
            "function": generate_feature_data,
            "args": [JokerFeatureTrainingType.JOKER_TYPE, 150],
        },
        "joker_edition": {
            "function": generate_feature_data,
            "args": [JokerFeatureTrainingType.JOKER_EDITION],
        },
        "all_consumables": {"function": generate_all_consumable_feature_data},
        "tarot": {
            "function": generate_feature_data,
            "args": [Consumables.TAROT],
        },
        "planet": {
            "function": generate_feature_data,
            "args": [Consumables.PLANET],
        },
        "spectral": {
            "function": generate_feature_data,
            "args": [Consumables.SPECTRAL],
        },
    }

    if len(sys.argv) < 2 or sys.argv[1] not in available_commands:
        print("Sorry that command is invalid please add 1 of the following:")
        for key in available_commands.keys():
            print(key)

        exit()

    command = sys.argv[1]
    command_config = available_commands[command]
    function = command_config["function"]
    args = command_config.get("args", [])
    function(*args)
