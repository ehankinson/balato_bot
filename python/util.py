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
    return random.randint(2, 15)



def card_crop(width: int, heigh: int, crop_values: list[float]) -> tuple[int, int, int, int]:
    return (
        int(crop_values[0]), int(crop_values[1]),
        int(width * crop_values[2]),
        int(heigh * crop_values[3])
    )



def build_folder(folder_path: str) -> None:
    if not os.path.isdir(folder_path):
        os.mkdir(folder_path)



def remove_folder(folder_path: str) -> None:
    if os.path.isdir(folder_path):
        shutil.rmtree(folder_path)



def rebuild_folder(folder_path: str) -> None:
    remove_folder(folder_path)
    build_folder(folder_path)
