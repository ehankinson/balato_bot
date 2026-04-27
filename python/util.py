import json
import yaml
import random

def load_yaml(filepath: str) -> dict:
    with open(filepath, "r", encoding="utf-8") as y:
        return yaml.safe_load(y)



def load_json(filepath: str) -> dict:
    with open(filepath, "r", encoding="utf-8") as j:
        return json.load(j)



def random_card_amount() -> int:
    return random.randint(2, 15)



def card_crop(width: int, heigh: int, crop_values: list[float], is_list: bool = True) -> list[int] | tuple[int]:
    val = [
        crop_values[0], crop_values[1],
        int(width * crop_values[2]),
        int(heigh * crop_values[3])
    ]
    return val if is_list else tuple(val)
