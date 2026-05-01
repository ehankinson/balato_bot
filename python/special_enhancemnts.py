import math
import random

import numpy as np
from PIL import Image


def _random_phase():
    return random.uniform(0.0, 10.0)


def _rgb_to_hsl(rgb):
    r, g, b = rgb[..., 0], rgb[..., 1], rgb[..., 2]
    low = np.minimum(r, np.minimum(g, b))
    high = np.maximum(r, np.maximum(g, b))
    delta = high - low
    summ = high + low

    h = np.zeros_like(r)
    l = 0.5 * summ
    s = np.zeros_like(r)

    mask = delta > 1e-8
    s[mask] = np.where(
        l[mask] < 0.5,
        delta[mask] / np.maximum(summ[mask], 1e-8),
        delta[mask] / np.maximum(2.0 - summ[mask], 1e-8),
    )

    rmax = mask & (high == r)
    gmax = mask & (high == g)
    bmax = mask & (high == b)

    h[rmax] = (g[rmax] - b[rmax]) / delta[rmax]
    h[gmax] = (b[gmax] - r[gmax]) / delta[gmax] + 2.0
    h[bmax] = (r[bmax] - g[bmax]) / delta[bmax] + 4.0
    h = (h / 6.0) % 1.0

    return np.stack([h, s, l], axis=-1)


def _hue_channel(s, t, h):
    hs = (h % 1.0) * 6.0
    out = np.empty_like(h)

    lt_1 = hs < 1.0
    between_1_3 = (hs >= 1.0) & (hs < 3.0)
    between_3_4 = (hs >= 3.0) & (hs < 4.0)

    out[lt_1] = (t[lt_1] - s[lt_1]) * hs[lt_1] + s[lt_1]
    out[between_1_3] = t[between_1_3]
    out[between_3_4] = (
        (t[between_3_4] - s[between_3_4]) * (4.0 - hs[between_3_4])
        + s[between_3_4]
    )
    out[hs >= 4.0] = s[hs >= 4.0]
    return out


def _hsl_to_rgb(hsl):
    h, sat, lum = hsl[..., 0], hsl[..., 1], hsl[..., 2]

    gray = sat < 1e-8
    t = np.where(lum < 0.5, sat * lum + lum, -sat * lum + (sat + lum))
    s = 2.0 * lum - t

    r = _hue_channel(s, t, h + 1.0 / 3.0)
    g = _hue_channel(s, t, h)
    b = _hue_channel(s, t, h - 1.0 / 3.0)

    rgb = np.stack([r, g, b], axis=-1)
    rgb[gray] = lum[gray, None]
    return rgb


def negative_effect(
    img: Image.Image
) -> Image.Image:
    negative_y = _random_phase()
    negative_shine_r = _random_phase()
    invert_lightness = negative_y != 0.0

    arr = np.asarray(img).astype(np.float32) / 255.0

    rgb = arr[..., :3]
    alpha = arr[..., 3:4]

    hsl = _rgb_to_hsl(rgb)

    if invert_lightness:
        hsl[..., 2] = 1.0 - hsl[..., 2]

    hsl[..., 0] = -hsl[..., 0] + 0.2

    out_rgb = _hsl_to_rgb(hsl)
    out_rgb += 0.8 * np.array([79 / 255, 99 / 255, 103 / 255], dtype=np.float32)
    out_alpha = alpha.copy()
    out_alpha[out_alpha < 0.7] /= 3.0

    out = np.dstack([out_rgb, out_alpha])

    shine = _apply_negative_shine(out, negative_shine_r)
    shine_alpha = shine[..., 3:4]
    out[..., :3] = out[..., :3] * (1.0 - shine_alpha) + shine[..., :3] * shine_alpha

    out = np.clip(out, 0.0, 1.0)
    return Image.fromarray((out * 255).astype(np.uint8), "RGBA")


def _apply_negative_shine(tex, negative_shine_r):
    h, w = tex.shape[:2]
    rgb = tex[..., :3]
    alpha = tex[..., 3]

    yy, xx = np.mgrid[0:h, 0:w]
    uvx = xx / max(w - 1, 1)
    uvy = yy / max(h - 1, 1)

    low = np.min(rgb, axis=-1)
    high = np.max(rgb, axis=-1)
    delta = high - low - 0.1

    fac = 0.8 + 0.9 * np.sin(
        11.0 * uvx
        + 4.32 * uvy
        + negative_shine_r * 12.0
        + np.cos(negative_shine_r * 5.3 + uvy * 4.2 - uvx * 4.0)
    )
    fac2 = 0.5 + 0.5 * np.sin(
        8.0 * uvx
        + 2.32 * uvy
        + negative_shine_r * 5.0
        - np.cos(negative_shine_r * 2.3 + uvx * 8.2)
    )
    fac3 = 0.5 + 0.5 * np.sin(
        10.0 * uvx
        + 5.32 * uvy
        + negative_shine_r * 6.111
        + np.sin(negative_shine_r * 5.3 + uvy * 3.2)
    )
    fac4 = 0.5 + 0.5 * np.sin(
        3.0 * uvx
        + 2.32 * uvy
        + negative_shine_r * 8.111
        + np.sin(negative_shine_r * 1.3 + uvy * 11.2)
    )
    fac5 = np.sin(
        0.9 * 16.0 * uvx
        + 5.32 * uvy
        + negative_shine_r * 12.0
        + np.cos(negative_shine_r * 5.3 + uvy * 4.2 - uvx * 4.0)
    )

    maxfac = 0.7 * np.maximum(
        np.maximum(fac, np.maximum(fac2, np.maximum(fac3, 0.0)))
        + (fac + fac2 + fac3 * fac4),
        0.0,
    )

    shine_rgb = rgb * 0.5 + np.array([0.4, 0.4, 0.8], dtype=np.float32)
    shine_rgb[..., 0] = (
        shine_rgb[..., 0] - delta + delta * maxfac * (0.7 + fac5 * 0.27) - 0.1
    )
    shine_rgb[..., 1] = (
        shine_rgb[..., 1] - delta + delta * maxfac * (0.7 - fac5 * 0.27) - 0.1
    )
    shine_rgb[..., 2] = shine_rgb[..., 2] - delta + delta * maxfac * 0.7 - 0.1

    alpha_factor = (
        0.5
        * np.maximum(
            np.minimum(
                1.0,
                np.maximum(0.0, 0.3 * np.maximum(low * 0.2, delta))
                + np.minimum(np.maximum(maxfac * 0.1, 0.0), 0.4),
            ),
            0.0,
        )
        + 0.15 * maxfac * (0.1 + delta)
    )
    shine_alpha = alpha * alpha_factor

    return np.dstack([shine_rgb, shine_alpha])


def hologram_effect(img: Image.Image) -> Image.Image:
    holo_x = _random_phase()
    holo_y = _random_phase()
    time = _random_phase()

    tex = np.asarray(img).astype(np.float32) / 255.0

    h, w = tex.shape[:2]
    rgb = tex[..., :3]
    alpha = tex[..., 3:4]

    yy, xx = np.mgrid[0:h, 0:w]

    uvx = xx / max(w - 1, 1)
    uvy = yy / max(h - 1, 1)

    blue_mix = 0.5 * rgb + 0.5 * np.array([0.0, 0.0, 1.0], dtype=np.float32)
    hsl = _rgb_to_hsl(blue_mix)

    t = holo_y * 7.221 + time

    floored_uvx = np.floor(uvx * w) / w
    floored_uvy = np.floor(uvy * h) / h

    uxc = (floored_uvx - 0.5) * 250.0
    uyc = (floored_uvy - 0.5) * 250.0

    p1x = uxc + 50.0 * math.sin(-t / 143.6340)
    p1y = uyc + 50.0 * math.cos(-t / 99.4324)

    p2x = uxc + 50.0 * math.cos(t / 53.1532)
    p2y = uyc + 50.0 * math.cos(t / 61.4532)

    p3x = uxc + 50.0 * math.sin(-t / 87.53218)
    p3y = uyc + 50.0 * math.sin(-t / 49.0000)

    field = (
        1.0
        + (
            np.cos(np.sqrt(p1x * p1x + p1y * p1y) / 19.483)
            + np.sin(np.sqrt(p2x * p2x + p2y * p2y) / 33.155) * np.cos(p2y / 15.73)
            + np.cos(np.sqrt(p3x * p3x + p3y * p3y) / 27.193) * np.sin(p3x / 21.92)
        )
    ) / 2.0

    res = 0.5 + 0.5 * np.cos(holo_x * 2.612 + (field - 0.5) * 3.14)

    low = np.min(rgb, axis=-1)
    high = np.max(rgb, axis=-1)
    delta = 0.2 + 0.3 * (high - low) + 0.1 * high

    gridsize = 0.79
    fac_a = np.maximum(0.0, 7.0 * np.abs(np.cos(uvx * gridsize * 20.0)) - 6.0)
    fac_b = np.maximum(
        0.0, 7.0 * np.cos(uvy * gridsize * 45.0 + uvx * gridsize * 20.0) - 6.0
    )
    fac_c = np.maximum(
        0.0, 7.0 * np.cos(uvy * gridsize * 45.0 - uvx * gridsize * 20.0) - 6.0
    )
    fac = 0.5 * np.maximum(np.maximum(fac_a, fac_b), fac_c)

    hsl[..., 0] = hsl[..., 0] + res + fac
    hsl[..., 1] = hsl[..., 1] * 1.3
    hsl[..., 2] = hsl[..., 2] * 0.6 + 0.4

    holo_rgb = _hsl_to_rgb(hsl) * np.array([0.9, 0.8, 1.2], dtype=np.float32)

    out_rgb = (1.0 - delta[..., None]) * rgb + delta[..., None] * holo_rgb
    out_alpha = alpha.copy()
    out_alpha[out_alpha < 0.7] /= 3.0

    out = np.dstack([out_rgb, out_alpha])
    out = np.clip(out, 0.0, 1.0)
    return Image.fromarray((out * 255).astype(np.uint8), "RGBA")


def foil_effect(
    img: Image.Image,
) -> Image.Image:
    alpha_floor = 0.78
    brightness_boost = 0.16
    foil_r = _random_phase()
    foil_g = _random_phase()

    tex = np.asarray(img).astype(np.float32) / 255.0

    h, w = tex.shape[:2]
    rgb = tex[..., :3]
    alpha = tex[..., 3]

    yy, xx = np.mgrid[0:h, 0:w]

    uvx = xx / max(w - 1, 1)
    uvy = yy / max(h - 1, 1)

    adjusted_x = uvx - 0.5
    adjusted_y = uvy - 0.5
    adjusted_x = adjusted_x * (h / w)

    length_90 = np.sqrt((90.0 * adjusted_x) ** 2 + (90.0 * adjusted_y) ** 2)
    length_113 = np.sqrt((113.1121 * adjusted_x) ** 2 + (113.1121 * adjusted_y) ** 2)
    length_20 = np.sqrt((20.0 * adjusted_x) ** 2 + (20.0 * adjusted_y) ** 2)

    low = np.min(rgb, axis=-1)
    high = np.max(rgb, axis=-1)
    delta = np.minimum(high, np.maximum(0.5, 1.0 - low))

    fac = (
        2.0 * np.sin(
            (length_90 + foil_r * 2.0)
            + 3.0 * (1.0 + 0.8 * np.cos(length_113 - foil_r * 3.121))
        )
        - 1.0
        - np.maximum(5.0 - length_90, 0.0)
    )
    fac = np.maximum(np.minimum(fac, 1.0), 0.0)

    rot_x = math.cos(foil_r * 0.1221)
    rot_y = math.sin(foil_r * 0.3512)

    dot = rot_x * adjusted_x + rot_y * adjusted_y
    rot_len = math.sqrt(rot_x * rot_x + rot_y * rot_y)
    uv_len = np.sqrt(adjusted_x * adjusted_x + adjusted_y * adjusted_y)

    angle = dot / np.maximum(rot_len * uv_len, 1e-8)

    fac2 = (
        5.0 * np.cos(
            foil_g * 0.3
            + angle * 3.14 * (2.2 + 0.9 * np.sin(foil_r * 1.65 + 0.2 * foil_g))
        )
        - 4.0
        - np.maximum(2.0 - length_20, 0.0)
    )
    fac2 = np.maximum(np.minimum(fac2, 1.0), 0.0)

    fac3 = 0.3 * (
        2.0 * np.sin(
            foil_r * 5.0
            + uvx * 3.0
            + 3.0 * (1.0 + 0.5 * np.cos(foil_r * 7.0))
        )
        - 1.0
    )
    fac3 = np.maximum(np.minimum(fac3, 1.0), -1.0)

    fac4 = 0.3 * (
        2.0 * np.sin(
            foil_r * 6.66
            + uvy * 3.8
            + 3.0 * (1.0 + 0.5 * np.cos(foil_r * 3.414))
        )
        - 1.0
    )
    fac4 = np.maximum(np.minimum(fac4, 1.0), -1.0)

    maxfac = np.maximum(
        np.maximum(fac, np.maximum(fac2, np.maximum(fac3, np.maximum(fac4, 0.0))))
        + 2.2 * (fac + fac2 + fac3 + fac4),
        0.0,
    )

    out = tex.copy()

    out[..., 0] = tex[..., 0] - delta * 0.7 + delta * maxfac * 0.45
    out[..., 1] = tex[..., 1] - delta * 0.7 + delta * maxfac * 0.45
    out[..., 2] = tex[..., 2] + delta * maxfac * 2.2
    out[..., 3] = np.minimum(
        alpha,
        0.3 * alpha + 0.9 * np.minimum(0.5, maxfac * 0.1),
    )

    opaque = alpha[..., None]
    out[..., :3] += brightness_boost * delta[..., None] * opaque
    out[..., 3] = np.maximum(out[..., 3], alpha * alpha_floor)

    out = np.clip(out, 0.0, 1.0)
    return Image.fromarray((out * 255).astype(np.uint8), "RGBA")


def polychrome_effect(
    img: Image.Image
) -> Image.Image:
    polychrome_x = _random_phase()
    polychrome_y = _random_phase()
    time = _random_phase()

    arr = np.asarray(img).astype(np.float32) / 255.0

    rgb = arr[..., :3]
    alpha = arr[..., 3:4]

    low = np.min(rgb, axis=-1)
    high = np.max(rgb, axis=-1)
    delta = high - low
    saturation_fac = 1.0 - np.maximum(0.0, 0.05 * (1.1 - delta))

    hsl_input = rgb.copy()
    hsl_input[..., 0] *= saturation_fac
    hsl_input[..., 1] *= saturation_fac
    hsl = _rgb_to_hsl(hsl_input)

    h, w = arr.shape[:2]
    yy, xx = np.mgrid[0:h, 0:w]

    uvx = xx / max(w - 1, 1)
    uvy = yy / max(h - 1, 1)

    floored_uvx = np.floor(uvx * w) / w
    floored_uvy = np.floor(uvy * h) / h

    x = (floored_uvx - 0.5) * 50.0
    y = (floored_uvy - 0.5) * 50.0

    phase = polychrome_y * 2.221 + time

    p1x = x + 50.0 * math.sin(-phase / 143.6340)
    p1y = y + 50.0 * math.cos(-phase / 99.4324)

    p2x = x + 50.0 * math.cos(phase / 53.1532)
    p2y = y + 50.0 * math.cos(phase / 61.4532)

    p3x = x + 50.0 * math.sin(-phase / 87.53218)
    p3y = y + 50.0 * math.sin(-phase / 49.0000)

    field = (
        1.0
        + (
            np.cos(np.sqrt(p1x * p1x + p1y * p1y) / 19.483)
            + np.sin(np.sqrt(p2x * p2x + p2y * p2y) / 33.155) * np.cos(p2y / 15.73)
            + np.cos(np.sqrt(p3x * p3x + p3y * p3y) / 27.193) * np.sin(p3x / 21.92)
        )
    ) / 2.0

    res = 0.5 + 0.5 * np.cos(polychrome_x * 2.612 + (field - 0.5) * 3.14)

    hsl[..., 0] = hsl[..., 0] + res + polychrome_y * 0.04
    hsl[..., 1] = np.minimum(0.6, hsl[..., 1] + 0.5)

    out_rgb = _hsl_to_rgb(hsl)
    out_alpha = alpha.copy()
    out_alpha[out_alpha < 0.7] /= 3.0

    out_arr = np.dstack([out_rgb, out_alpha])
    return Image.fromarray((out_arr * 255).clip(0, 255).astype(np.uint8), "RGBA")
