import time

import mss
import pyautogui
import pyscreenshot
from PIL import Image

from config.settings import HAND_CROP_LEFT, HAND_CROP_TOP, HAND_HEIGHT, HAND_WIDTH, PLAY_HAND_X, PLAY_HAND_Y
from core.enums import HandAction
from core.models import Card, CardData, Deck, FinalScoringResults, GameState
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
        pyautogui.click()

    if mode == HandAction.PLAY_HAND:
        pyautogui.moveTo(PLAY_HAND_X, PLAY_HAND_Y)
    else:
        pyautogui.moveTo(PLAY_HAND_X + 500, PLAY_HAND_Y)
    pyautogui.click()


def play_blind(deck: Deck, game_state: GameState) -> None:
    i = 0
    while game_state.hands > 0:
        hand_img = get_hand()
        hand_img.save(f"../hand_{i}.png")

        selected_data, mode, hand = get_played_hand(hand_img, deck, game_state)
        selected_cards = [data.card for data in selected_data]
        print(f"The Selected mode was {mode}")

        play_hand(selected_data, mode)
        has_won = game_state.execute_hand_action(mode, selected_cards, hand, deck)
        print(f"GameState has {game_state.hands} hands and {game_state.discards} discards left")
        print(f"GameState has current score as {game_state.current_score}")
        
        print()
        card_time = 0.5
        time.sleep(1.85 + card_time * len(selected_data))
        if has_won:
            return

        i += 1


if __name__ == "__main__":
    deck = Deck()
    game_state = GameState(score_to_beat=0)
    for _ in range(3):
        score_to_beat = int(input("input score to beat: "))
        game_state.score_to_beat = score_to_beat
        play_blind(deck, game_state)
        deck.reset()
        game_state.reset()
