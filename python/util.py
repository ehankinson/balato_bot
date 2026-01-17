"Utility functions for the balatro project"

import os
import json


def join_path(curr_dir: str, add_path: str) -> str:
    """Join a current directory path with an additional path."""
    return os.path.join(curr_dir, add_path)



def load_json(file_path: str) -> dict:
    """Load a JSON file and return its contents as a dictionary."""
    with open(file_path, "r", encoding="utf-8") as f:
        return json.load(f)
