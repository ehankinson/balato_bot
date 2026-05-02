from PIL import Image
import mss
import pyscreenshot

from config.settings import HAND_HEIGHT, HAND_WIDTH


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
    return crop_play_hand(main_screen, 670, 800)


def save_hand() -> None:
    global count

    img = get_hand()
    filename = f"training_data/real_data/hand_{count}.png"
    img.save(filename)

    print(f"Saved {filename}")
    count += 1


if __name__ == "__main__":
    while True:
        if input() == "p":
            save_hand()
