import time

import mss
import pyautogui
from config.settings import (
    HAND_CROP_LEFT,
    HAND_CROP_TOP,
    HAND_HEIGHT,
    HAND_WIDTH,
    PLAY_HAND_X,
    PLAY_HAND_Y,
)
from core.enums import HandAction
from core.models import CardData
from PIL import Image, ImageGrab
from vision import get_selected_cards

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
    image = ImageGrab.grab(bbox=primary_monitor_bbox()).convert("RGB")
    if filename is not None:
        image.save(filename)

    return image


def crop_play_hand(img: Image.Image, left: int, top: int) -> Image.Image:
    return img.crop((left, top, left + HAND_WIDTH, top + HAND_HEIGHT))


def get_hand() -> Image.Image:
    main_screen = screenshot_primary()
    return crop_play_hand(main_screen, HAND_CROP_LEFT, HAND_CROP_TOP)


def play_hand(played_hand: list[CardData], mode: HandAction) -> None:
    monitor_left, monitor_top, _, _ = primary_monitor_bbox()

    for card_data in played_hand:
        location = card_data.location
        screen_x = HAND_CROP_LEFT + location[0] + monitor_left + 50
        screen_y = HAND_CROP_TOP + location[1] + monitor_top + 75

        pyautogui.moveTo(screen_x, screen_y)
        time.sleep(0.05)
        pyautogui.click()

    if mode == HandAction.PLAY_HAND:
        pyautogui.moveTo(PLAY_HAND_X, PLAY_HAND_Y)
    else:
        pyautogui.moveTo(PLAY_HAND_X + 500, PLAY_HAND_Y)
    pyautogui.click()


def execute_card_command(command, observation):
    """Execute an already decoded play/discard command against a synchronized UI.

    This function only clicks cards. The owner must synchronize the Rust game
    after successful UI execution; full-game UI synchronization is separate.
    """
    selected = get_selected_cards(get_hand(), command, observation)
    mode = HandAction.PLAY_HAND if command.kind == 2 else HandAction.DISCARD
    play_hand(selected, mode)


if __name__ == "__main__":
    screenshot_primary("tmp.png")
