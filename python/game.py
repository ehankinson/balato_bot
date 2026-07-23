import time

import mss
import pyautogui
import pyscreenshot
from PIL import Image

from config.settings import (
    CASH_OUT_X,
    CASH_OUT_Y,
    HAND_CROP_LEFT,
    HAND_CROP_TOP,
    HAND_HEIGHT,
    HAND_WIDTH,
    NEXT_ROUND_X,
    NEXT_ROUND_Y,
    PLAY_HAND_X,
    PLAY_HAND_Y,
    SELECT_BLIND_1_X,
    SELECT_BLIND_1_Y,
    SELECT_BLIND_2_X,
    SELECT_BLIND_2_Y,
)
from core.enums import HandAction
from core.models import Card, CardData, Deck, FinalScoringResults, GameState
from model import preload_models
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


def play_blind(deck: Deck, game_state: GameState) -> None:
    hand = []
    while game_state.hands > 0:
        hand_img = get_hand()

        selected_data, mode, hand = get_played_hand(hand_img, deck, game_state)
        selected_cards = [data.card for data in selected_data]
        play_hand(selected_data, mode)
        has_won = game_state.execute_hand_action(
            mode, selected_cards, hand, deck, draw=False
        )

        if mode == HandAction.PLAY_HAND:
            card_time = 0.5
            time.sleep(1.85 + card_time * len(selected_data))
        else:
            time.sleep(1)

        if has_won:
            break

    deck.add_to_discard_pile(hand)


if __name__ == "__main__":
    deck = Deck()
    things_to_do = 100
    game_state = GameState(score_to_beat=0)
    scores_to_beat = [300, 450]
    score_index = 0
    preload_models()

    for _ in range(things_to_do):
        if score_index == 2:
            pyautogui.keyDown("r")
            time.sleep(3)
            pyautogui.keyUp("r")
            score_index = 0

        if score_index == 0:
            pyautogui.moveTo(SELECT_BLIND_1_X, SELECT_BLIND_1_Y)
        else:
            pyautogui.moveTo(SELECT_BLIND_2_X, SELECT_BLIND_2_Y)

        print("Moved to the select blind_button")
        time.sleep(0.05)
        pyautogui.click()
        time.sleep(2)

        game_state.score_to_beat = scores_to_beat[score_index]
        print("Playing Blind")
        play_blind(deck, game_state)

        score_index += 1

        card_to_deck = 0.15
        time.sleep(card_to_deck * len(deck.discards))
        time.sleep(1.25)
        print("Moving to CashOUT")
        pyautogui.moveTo(CASH_OUT_X, CASH_OUT_Y)
        pyautogui.click()

        deck.reset()
        game_state.reset()

        time.sleep(1)
        print("Moving to Next Round in the SHOP")
        pyautogui.moveTo(NEXT_ROUND_X, NEXT_ROUND_Y)
        pyautogui.click()
        time.sleep(0.75)
