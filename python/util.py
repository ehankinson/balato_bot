import yaml
import random

def load_yaml(filepath: str) -> dict:
    with open(filepath, "r", encoding="utf-8") as y:
        return yaml.safe_load(y)



def random_card_amount() -> int:
    return random.randint(2, 15)
