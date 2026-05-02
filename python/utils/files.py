import json
import os
import shutil

import yaml


def load_yaml(filepath: str) -> dict:
    with open(filepath, "r", encoding="utf-8") as y:
        return yaml.safe_load(y)


def write_yaml(filepath: str, object: dict) -> None:
    with open(filepath, "w", encoding="utf-8") as y:
        yaml.dump(object, y)


def load_json(filepath: str) -> dict:
    with open(filepath, "r", encoding="utf-8") as j:
        return json.load(j)


def build_folder(folder_path: str) -> None:
    if not os.path.isdir(folder_path):
        os.mkdir(folder_path)


def remove_folder(folder_path: str) -> None:
    if os.path.isdir(folder_path):
        shutil.rmtree(folder_path)


def rebuild_folder(folder_path: str) -> None:
    remove_folder(folder_path)
    build_folder(folder_path)

