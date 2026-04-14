import yaml

def load_yaml(filepath: str) -> dict:
    with open(filepath, "r", encoding="utf-8") as y:
        return yaml.safe_load(y)
