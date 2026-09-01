import os
import random
import sys
import threading
from collections.abc import Callable
from concurrent.futures import ThreadPoolExecutor

from PIL import Image
from tqdm import tqdm

from config.settings import (
    CONSUMABLE_CANVAS_HEIGHT,
    CONSUMABLE_CANVAS_WIDTH,
    CONSUMABLE_CROP,
    EDITION_CROP,
    ENHANCEMENT_CROP,
    HAND_HEIGHT,
    HAND_WIDTH,
    JOKER_CANVAS_HEIGHT,
    JOKER_CANVAS_WIDTH,
    JOKER_NAME_CROP,
    RANK_CROP,
    ROOT_DIR,
    SEAL_CROP,
    SUIT_CROP,
)
from core.enums import (
    Edition,
    Enhancement,
    JokerEdition,
    JokerName,
    Planet,
    Rank,
    Seal,
    Spectral,
    Suit,
    Tarot,
)
from core.models import Card, CardAnnotation, Hand, Joker, RenderedHand
from core.type_aliases import Feature
from rendering.backgrounds import render_background
from rendering.consumable import (
    generate_consumables,
    render_consumables,
)
from rendering.hand import generate_hand, render_hand
from rendering.joker import generate_jokers, render_jokers
from utils.files import build_folder, rebuild_folder
from utils.images import card_crop, yolo_box_to_crop

CUTOFF = 0.9  # split between training and val
WORKER_AMOUNT = 8
CropBox = tuple[int | float, int | float, int | float, int | float]


def build_random_consumables(amount: int) -> list[Feature]:
    feature_type = random.choice([Tarot, Planet, Spectral])
    return [random.choice(list(feature_type)) for _ in range(amount)]


LOCATION_RENDERERS: dict[
    str,
    tuple[
        int,
        tuple[int, int],
        tuple[int, int],
        Callable[[int], object],
        Callable[..., RenderedHand],
    ],
] = {
    "card": (
        0,
        (6, 16),
        (HAND_WIDTH, HAND_HEIGHT),
        lambda amount: Hand([Card.random() for _ in range(amount)]),
        render_hand,
    ),
    "joker": (
        1,
        (1, 9),
        (JOKER_CANVAS_WIDTH, JOKER_CANVAS_HEIGHT),
        lambda amount: [Joker.random() for _ in range(amount)],
        render_jokers,
    ),
    "consumable": (
        2,
        (1, 4),
        (CONSUMABLE_CANVAS_WIDTH, CONSUMABLE_CANVAS_HEIGHT),
        build_random_consumables,
        render_consumables,
    ),
}


FEATURES: dict[str, type[Feature]] = {
    "rank": Rank,
    "suit": Suit,
    "enhancement": Enhancement,
    "edition": Edition,
    "seal": Seal,
    "tarot": Tarot,
    "planet": Planet,
    "spectral": Spectral,
    "joker_name": JokerName,
    "joker_edition": JokerEdition,
}

FEATURE_GROUPS: dict[
    str, tuple[tuple[int, int], Callable[[int, Feature], RenderedHand], list[str]]
] = {
    "card_features": (
        (6, 16),
        generate_hand,
        ["enhancement", "edition", "rank", "suit", "seal"],
    ),
    "consumables": ((1, 4), generate_consumables, ["tarot", "planet", "spectral"]),
    "jokers": ((1, 9), generate_jokers, ["joker_name", "joker_edition"]),
}

FEATURE_COMMANDS: dict[
    str, tuple[tuple[int, int], Callable[[int, Feature], RenderedHand], list[str]]
] = {
    feature: (render_amount, render_function, [feature])
    for render_amount, render_function, group_features in FEATURE_GROUPS.values()
    for feature in group_features
} | {
    f"all_{group}": (render_amount, render_function, group_features)
    for group, (
        render_amount,
        render_function,
        group_features,
    ) in FEATURE_GROUPS.items()
}

FEATURE_CROPS = {
    Rank: RANK_CROP,
    Suit: SUIT_CROP,
    Enhancement: ENHANCEMENT_CROP,
    Seal: SEAL_CROP,
    Edition: EDITION_CROP,
    Tarot: CONSUMABLE_CROP,
    Spectral: CONSUMABLE_CROP,
    Planet: CONSUMABLE_CROP,
    JokerName: JOKER_NAME_CROP,
    JokerEdition: JOKER_NAME_CROP,
}


def split_work(total_amount: int) -> list[range]:
    chunk_size, extra = divmod(total_amount, WORKER_AMOUNT)
    chunks: list[range] = []
    start = 0
    for worker_index in range(WORKER_AMOUNT):
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
    crop_values = FEATURE_CROPS[type(feature)]
    return card_crop(w, h, crop_values)


def generate_location_training_data(size: int) -> None:
    start_path = os.path.join(ROOT_DIR, "training_data", "location_data")
    rebuild_folder(start_path)
    image_root = f"{start_path}/images"
    label_root = f"{start_path}/labels"
    build_folder(image_root)
    build_folder(label_root)

    for split in ("train", "val"):
        build_folder(f"{image_root}/{split}")
        build_folder(f"{label_root}/{split}")

    cutoff = size * CUTOFF

    work: list[tuple[str, int]] = [
        (render_type, index)
        for render_type in LOCATION_RENDERERS
        for index in range(size)
    ]
    random.shuffle(work)

    def remap_box(
        box: list[float],
        class_id: int,
        offset_x: int,
        offset_y: int,
        small_w: int,
        small_h: int,
    ) -> list[float]:
        _, center_x, center_y, box_w, box_h = box
        return [
            class_id,
            round((offset_x + center_x * small_w) / HAND_WIDTH, 6),
            round((offset_y + center_y * small_h) / HAND_HEIGHT, 6),
            round(box_w * small_w / HAND_WIDTH, 6),
            round(box_h * small_h / HAND_HEIGHT, 6),
        ]

    def render_on_main_canvas(render_type: str) -> RenderedHand:
        class_id, (low, high), (small_w, small_h), build_items, render_function = (
            LOCATION_RENDERERS[render_type]
        )
        items = build_items(random.randint(low, high))

        background = render_background(HAND_WIDTH, HAND_HEIGHT, True)
        offset_x = (HAND_WIDTH - small_w) // 2
        offset_y = (HAND_HEIGHT - small_h) // 2
        small_background = background.crop(
            (offset_x, offset_y, offset_x + small_w, offset_y + small_h)
        )

        rendered = render_function(items, True, background=small_background)
        background.paste(rendered.image, (offset_x, offset_y))

        annotations = [
            CardAnnotation(
                card=data.card,
                box=remap_box(data.box, class_id, offset_x, offset_y, small_w, small_h),
            )
            for data in rendered.annotations
        ]
        return RenderedHand(image=background, annotations=annotations)

    progress_lock = threading.Lock()

    def process_locations(work_chunk: range, progress: tqdm) -> None:
        for work_index in work_chunk:
            render_type, index = work[work_index]
            split = "train" if index < cutoff else "val"
            name = f"{render_type}_{index}"

            hand_render = render_on_main_canvas(render_type)

            hand_render.image.save(f"{image_root}/{split}/{name}.png")

            label_path = f"{label_root}/{split}/{name}.txt"
            with open(label_path, "w", encoding="utf-8") as t:
                for data in hand_render.annotations:
                    line = " ".join(str(val) for val in data.box)
                    t.write(line + "\n")

            with progress_lock:
                progress.update(1)

    with (
        tqdm(total=len(work)) as progress,
        ThreadPoolExecutor(max_workers=WORKER_AMOUNT) as executor,
    ):
        chunks = split_work(len(work))
        list(executor.map(lambda chunk: process_locations(chunk, progress), chunks))


def generate_feature_data(
    start_path: str,
    amount: int,
    render_function: Callable[[int, Feature], RenderedHand],
    features: list[Feature],
    schedule: list[tuple[Feature, int, str]],
):
    build_folders(start_path, features)
    chunks = split_work(amount)
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
        ThreadPoolExecutor(max_workers=WORKER_AMOUNT) as executor,
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
    available_commands = list(FEATURE_COMMANDS) + ["locations"]

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

    if command == "locations":
        generate_location_training_data(training_amount)
        sys.exit()

    render_amount, render_function, features = FEATURE_COMMANDS[command]
    for feature in features:
        setup(training_amount, feature, start_path, render_amount, render_function)
