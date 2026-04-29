import os
import json
import yaml
import shutil
import random

def load_yaml(filepath: str) -> dict:
    with open(filepath, "r", encoding="utf-8") as y:
        return yaml.safe_load(y)



def load_json(filepath: str) -> dict:
    with open(filepath, "r", encoding="utf-8") as j:
        return json.load(j)



def random_card_amount() -> int:
    return random.choices(
        population=[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18],
        weights=[1, 1, 2, 3, 5, 8, 16, 50, 33, 18, 7, 4, 1, 1, 1, 1, 1, 1],
        k=1
    )[0]



def card_crop(width: int, height: int, crop_values: list[float]) -> tuple[int, int, int, int]:
    left = int(crop_values[0])
    top = int(crop_values[1])
    right = int(width * crop_values[2])
    bottom = int(height * crop_values[3])

    left = max(0, min(left, width - 1))
    top = max(0, min(top, height - 1))
    right = max(left + 1, min(right, width))
    bottom = max(top + 1, min(bottom, height))

    return left, top, right, bottom



def build_folder(folder_path: str) -> None:
    if not os.path.isdir(folder_path):
        os.mkdir(folder_path)



def remove_folder(folder_path: str) -> None:
    if os.path.isdir(folder_path):
        shutil.rmtree(folder_path)



def rebuild_folder(folder_path: str) -> None:
    remove_folder(folder_path)
    build_folder(folder_path)
