import time

import mss
import pyautogui
import pyscreenshot
from PIL import Image

from config.settings import HAND_CROP_LEFT, HAND_CROP_TOP, HAND_HEIGHT, HAND_WIDTH, PLAY_HAND_X, PLAY_HAND_Y
from core.models import Card, CardData, FinalScoringResults
from vision import get_card_locations, get_played_hand

count = 0


def primary_monitor_bbox() -> tuple[int, int, int, int]:
    with mss.MSS() as screen_capture:
        primary_monitor = screen_capture.monitors[1]

    left = primary_monitor["left"]
    top = primary_monitor["top"]
    right = left + primary_monitor["width"]
    bottom = top + primary_monitor["height"]
    return left, top, right, bottom


def screenshot_primary(filename: str | None = None) -> Image.Image:
    image = pyscreenshot.grab(bbox=primary_monitor_bbox()).convert("RGB")
    if filename is not None:
        image.save(filename)

    return image


def crop_play_hand(img: Image.Image, left: int, top: int) -> Image.Image:
    return img.crop((left, top, left + HAND_WIDTH, top + HAND_HEIGHT))


def get_hand() -> Image.Image:
    main_screen = screenshot_primary()
    return crop_play_hand(main_screen, HAND_CROP_LEFT, HAND_CROP_TOP)


def play_hand(played_hand: list[CardData]) -> None:
    monitor_left, monitor_top, _, _ = primary_monitor_bbox()
    
    for card_data in played_hand:
        location = card_data.location
        screen_x = HAND_CROP_LEFT + location[0] + monitor_left + 50
        screen_y = HAND_CROP_TOP + location[1] + monitor_top + 75

        pyautogui.moveTo(screen_x, screen_y)
        pyautogui.click()

    pyautogui.moveTo(PLAY_HAND_X, PLAY_HAND_Y)
    pyautogui.click()


def save_hand() -> None:
    for _ in range(4):
        hand_img = get_hand()
        hand_img.save("../hand_0.png")
        scored_played, _, _ = get_played_hand(hand_img)
        play_hand(scored_played)

        card_time = 0.5
        time.sleep(1.85 + card_time * len(scored_played))


if __name__ == "__main__":
    save_hand()
