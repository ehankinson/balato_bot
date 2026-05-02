import random
import threading

# 1445x393 THIS IS THE DIMENSIONS OF the SS WE ARE TAKING
from PIL import Image, ImageDraw, ImageFilter

from render_card import render_card
from card_models import Card, Hand, RenderedHand, CardAnnotation
from const import CARD_ID, HAND_WIDTH, HAND_HEIGHT, X_RATIO_GAP, Y_RATIO_GAP


MAX_Y_LIFT = 18
Y_JITTER = 5
ANGLE = 5.6
ANGLE_JITTER = 0.5
X_START_GAP = int(X_RATIO_GAP * HAND_WIDTH)
Y_START_GAP = int(Y_RATIO_GAP * HAND_HEIGHT)
TOTAL_HAND_WIDTH_RATIO: float = 1372 / 1445
TOTAL_HAND_WIDTH = TOTAL_HAND_WIDTH_RATIO * HAND_WIDTH
BACKGROUND_PALETTES = [
    ((31, 122, 77), (68, 164, 95), (18, 78, 62)),      # green
    ((142, 120, 28), (205, 175, 54), (86, 72, 24)),     # yellow/gold
    ((121, 41, 55), (183, 62, 79), (73, 29, 43)),       # red
    ((43, 75, 137), (77, 126, 193), (26, 47, 92)),      # blue
    ((84, 52, 135), (135, 82, 186), (48, 34, 86)),      # purple
]
BACKGROUND_POOL_SIZE = 32
BACKGROUND_POOL: list[Image.Image] | None = None
BACKGROUND_POOL_LOCK = threading.Lock()


def lerp_color(
    color_a: tuple[int, int, int],
    color_b: tuple[int, int, int],
    amount: float
) -> tuple[int, ...]:
    return tuple(
        round(a + (b - a) * amount)
        for a, b in zip(color_a, color_b)
    )


def render_background_texture(width: int, height: int) -> Image.Image:
    base_color, glow_color, shadow_color = random.choice(BACKGROUND_PALETTES)
    scale = 4
    small_width = max(1, width // scale)
    small_height = max(1, height // scale)
    img = Image.new("RGBA", (small_width, small_height), base_color + (255,))
    pixels = img.load()

    for y in range(small_height):
        vertical_amount = y / max(1, small_height - 1)
        for x in range(small_width):
            horizontal_amount = x / max(1, small_width - 1)
            glow = 1 - min(1, ((horizontal_amount - 0.52) ** 2 * 4.0) + ((vertical_amount - 0.42) ** 2 * 2.4))
            shade = min(1, vertical_amount * 0.55 + abs(horizontal_amount - 0.5) * 0.35)
            color = lerp_color(base_color, glow_color, max(0, glow) * 0.32)
            color = lerp_color(color, shadow_color, shade * 0.38)
            pixels[x, y] = color + (255,)

    texture = Image.new("RGBA", (small_width, small_height), (0, 0, 0, 0))
    draw = ImageDraw.Draw(texture, "RGBA")

    for _ in range(random.randint(10, 18)):
        start_x = random.randint(-small_width // 3, small_width)
        start_y = random.randint(-small_height // 2, small_height)
        end_x = start_x + random.randint(small_width // 4, small_width)
        end_y = start_y + random.randint(-small_height // 2, small_height // 2)
        color = random.choice([(255, 255, 220, 18), (255, 255, 255, 12), (0, 0, 0, 18)])
        draw.line((start_x, start_y, end_x, end_y), fill=color, width=random.randint(1, 3))

    for _ in range(random.randint(6, 10)):
        x = random.randint(-small_width // 4, small_width)
        y = random.randint(-small_height // 2, small_height)
        box_w = random.randint(small_width // 4, small_width // 2)
        box_h = random.randint(small_height // 2, small_height)
        color = random.choice([(255, 255, 230, 14), (0, 0, 0, 12)])
        draw.arc((x, y, x + box_w, y + box_h), random.randint(0, 180), random.randint(180, 360), fill=color, width=random.randint(1, 4))

    texture = texture.filter(ImageFilter.GaussianBlur(random.uniform(0.5, 1.2)))
    img = Image.alpha_composite(img, texture)

    fog = Image.new("RGBA", (small_width, small_height), (0, 0, 0, 0))
    draw = ImageDraw.Draw(fog, "RGBA")
    for _ in range(random.randint(5, 9)):
        x = random.randint(-small_width // 3, small_width)
        y = random.randint(-small_height // 4, small_height)
        fog_w = random.randint(small_width // 4, small_width // 2)
        fog_h = random.randint(small_height // 5, small_height // 2)
        draw.ellipse((x, y, x + fog_w, y + fog_h), fill=(255, 255, 235, random.randint(10, 24)))

    fog = fog.filter(ImageFilter.GaussianBlur(random.uniform(4.5, 9.0)))
    img = Image.alpha_composite(img, fog)
    return img.resize((width, height), Image.Resampling.BICUBIC)


def get_background_pool() -> list[Image.Image]:
    global BACKGROUND_POOL

    if BACKGROUND_POOL is None:
        with BACKGROUND_POOL_LOCK:
            if BACKGROUND_POOL is None:
                BACKGROUND_POOL = [
                    render_background_texture(HAND_WIDTH, HAND_HEIGHT)
                    for _ in range(BACKGROUND_POOL_SIZE)
                ]

    return BACKGROUND_POOL


def render_background(width: int, height: int) -> Image.Image:
    if width == HAND_WIDTH and height == HAND_HEIGHT:
        return random.choice(get_background_pool()).copy()

    return render_background_texture(width, height)

def calculate_card_gap(card_amount: int, card_width: int) -> float:
    if card_amount <= 1:
        return 0.0

    total_card_space = card_amount * card_width
    shift_space = TOTAL_HAND_WIDTH - total_card_space
    shift_per_card = shift_space / (card_amount - 1)
    return shift_per_card



def calculate_card_angle(card_index: int, card_amount: int) -> float:
    mid = (card_amount - 1) / 2
    if mid == 0:
        return 0.0

    offset_from_center = card_index - mid
    normalized_offset = offset_from_center / mid
    return -normalized_offset * ANGLE



def calculate_card_y_lift(card_index: int, card_amount: int) -> float:
    mid = (card_amount - 1) / 2
    if mid == 0:
        return 0.0

    distance_from_center = abs(card_index - mid)
    normalized_center_lift = 1 - (distance_from_center / mid)
    return normalized_center_lift * MAX_Y_LIFT



def calculate_box_dimensions(card: Card, card_image: Image.Image, x_pos: int, y_pos: int) -> CardAnnotation:
    card_w, card_h = card_image.width, card_image.height
    center_x = round((x_pos + card_w / 2) / HAND_WIDTH, 6)
    center_y = round((y_pos + card_h / 2) / HAND_HEIGHT, 6)
    norm_width = round(card_w / HAND_WIDTH, 6)
    norm_height = round(card_h / HAND_HEIGHT, 6)
    return CardAnnotation(
        card=card,
        box=[CARD_ID, center_x, center_y, norm_width, norm_height]
    )



def render_hand(hand: Hand) -> RenderedHand:
    img = render_background(HAND_WIDTH, HAND_HEIGHT)

    card_amount = len(hand.cards)
    card_gap: float = 0.0
    annotations: list[CardAnnotation] = []

    for i, card in enumerate(hand.cards):
        card_image = render_card(card)

        angle = calculate_card_angle(i, card_amount) + random.uniform(-ANGLE_JITTER, ANGLE_JITTER)

        if i == 0:
            card_gap = calculate_card_gap(card_amount, card_image.width)

        x_pos = int(X_START_GAP + i * (card_image.width + card_gap))
        y_jitter = random.uniform(-Y_JITTER, Y_JITTER)
        y_pos = round(Y_START_GAP - calculate_card_y_lift(i, card_amount) + y_jitter)

        card_image = card_image.rotate(angle, expand=True)
        img.paste(card_image, (x_pos, y_pos), card_image)

        annotations.append(calculate_box_dimensions(
            card, card_image, x_pos, y_pos
        ))

    return RenderedHand(
        image=img,
        annotations=annotations
    )



if __name__ == '__main__':
    hand = Hand.random_hand(11)
    render_hand(hand)
