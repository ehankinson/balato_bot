import os


def join_path(curr_dir: str, add_path: str) -> str:
    return os.path.join(curr_dir, add_path)