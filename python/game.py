from PIL import Image
import mss
import pyscreenshot

from const import HAND_HEIGHT, HAND_WIDTH



def primary_monitor_bbox() -> tuple[int, int, int, int]:
    with mss.mss() as screen_capture:
        primary_monitor = screen_capture.monitors[1]

    left = primary_monitor["left"]
    top = primary_monitor["top"]
    right = left + primary_monitor["width"]
    bottom = top + primary_monitor["height"]
    return left, top, right, bottom



def screenshot_primary(filename: str | None = None) -> Image.Image.Image:
    image = pyscreenshot.grab(bbox=primary_monitor_bbox()).convert("RGB")
    if filename is not None:
        image.save(filename)

    return image



def crop_play_hand(img: Image.Image, left: int, top: int) -> Image.Image.Image:
    return img.crop((left, top, left + HAND_WIDTH, top + HAND_HEIGHT))



if __name__ == "__main__":
    # screenshot = screenshot_primary("image.png")
    img = Image.open("image.png").convert("RGB")
    hand = crop_play_hand(img, left=670, top=800)
    hand.save("hand.png")
