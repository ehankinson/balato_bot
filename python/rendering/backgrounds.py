import random
import threading

from PIL import Image, ImageDraw, ImageFilter

from config.settings import BACKGROUND_PALETTES, BACKGROUND_POOL_SIZE


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


def get_background_pool(width: int, height: int, size: int) -> list[Image.Image]:
    global BACKGROUND_POOL

    if BACKGROUND_POOL is None:
        with BACKGROUND_POOL_LOCK:
            if BACKGROUND_POOL is None:
                BACKGROUND_POOL = [
                    render_background_texture(width, height)
                    for _ in range(size)
                ]

    return BACKGROUND_POOL


def render_background(width: int, height: int, training: bool = False) -> Image.Image:
    if training:
        return random.choice(get_background_pool(width, height, BACKGROUND_POOL_SIZE)).copy()

    return render_background_texture(width, height)

