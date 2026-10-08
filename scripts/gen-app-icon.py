"""Translator Overlay app icon.

Windows 48x48 icon grid (Microsoft app-icon-design):

* Overlay: a caption sitting on a source window, hanging off the
  bottom-right so it reads as a layer covering the original.
* Source / translation: the back plate is a window (title bar + two
  uneven caption lines). The overlay repeats the line pattern as the
  translated caption. Same mark at every size; only stroke weight
  changes below 28px so the lines survive 16px.
* Straight-on, transparent canvas, 2px exterior corners on the grid,
  subtle 120° analogous fills, separate-metaphor shadow clipped to the
  shape below. Windows 11 clips the ICO; this asset is unplated.

Source lines are clipped to the visible client (window minus title bar
minus overlay) so they never paint over the overlay frame.
"""

from __future__ import annotations

import io
import math
import struct
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFilter

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "crates" / "translator-app" / "assets"

# Analogous brand pair (cyan source window, magenta overlay). Dark/mid/light
# so at least half the mark contrasts on both light and dark shells.
CYAN_TL = (6, 182, 212)  # #06B6D4
CYAN_BR = (14, 116, 144)  # #0E7490
TITLE_TL = (8, 92, 112)
TITLE_BR = (6, 58, 74)
MAGENTA_TL = (192, 38, 211)  # #C026D3
MAGENTA_BR = (134, 25, 143)  # #86198F
GLYPH = (255, 255, 255, 255)


def lerp(a, b, t):
    return tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(len(a)))


def gradient_120(size, c0, c1):
    """CSS 120°: lighter top-left, darker bottom-right (one hue ramp)."""
    w, h = size
    img = Image.new("RGBA", (w, h))
    px = img.load()
    dx = math.sin(math.radians(120.0))
    dy = -math.cos(math.radians(120.0))
    projs = [
        0.0 * dx + 0.0 * dy,
        float(w) * dx + 0.0 * dy,
        0.0 * dx + float(h) * dy,
        float(w) * dx + float(h) * dy,
    ]
    pmin, pmax = min(projs), max(projs)
    span = pmax - pmin or 1.0
    for y in range(h):
        row = y * dy
        for x in range(w):
            t = (x * dx + row - pmin) / span
            px[x, y] = lerp(c0, c1, t) + (255,)
    return img


def round_rect_mask(size: tuple[int, int], box, radius: float) -> Image.Image:
    m = Image.new("L", size, 0)
    d = ImageDraw.Draw(m)
    x, y, w, h = box
    r = max(min(radius, w / 2, h / 2), 1)
    d.rounded_rectangle((x, y, x + w, y + h), radius=r, fill=255)
    return m


def fill_mask(mask: Image.Image, c0, c1) -> Image.Image:
    grad = gradient_120(mask.size, c0, c1)
    out = Image.new("RGBA", mask.size, (0, 0, 0, 0))
    out.paste(grad, (0, 0), mask)
    return out


def shadow_on(base_mask: Image.Image, caster: Image.Image, u: float) -> Image.Image:
    """Separate-metaphor drop shadow, clipped to the shape underneath."""
    layer = Image.new("RGBA", caster.size, (0, 0, 0, 0))
    for offset, blur, opacity in (
        (0.5 * u, 0.6 * u, 0.22),
        (1.6 * u, 2.2 * u, 0.16),
    ):
        sh = Image.new("L", caster.size, 0)
        sh.paste(caster, (0, max(int(round(offset)), 1)))
        if blur >= 0.4:
            sh = sh.filter(ImageFilter.GaussianBlur(blur))
        sh = ImageChops.multiply(sh, base_mask)
        sh = sh.point(lambda p, o=opacity: int(p * o))
        zeros = Image.new("L", sh.size, 0)
        layer = Image.alpha_composite(layer, Image.merge("RGBA", (zeros, zeros, zeros, sh)))
    return layer


def title_mask(size: tuple[int, int], window_box, window_mask, u: float, bar_units: float) -> Image.Image:
    x, y, w, _h = window_box
    m = Image.new("L", size, 0)
    ImageDraw.Draw(m).rectangle((x, y, x + w, y + u * bar_units), fill=255)
    return ImageChops.multiply(m, window_mask)


def caption_lines_mask(
    size: tuple[int, int],
    box,
    u: float,
    widths: tuple[float, ...],
    line_units: float,
    gap_units: float,
    pad_x_units: float,
    pad_y_units: float,
) -> Image.Image:
    """Horizontal caption bars (simplified text) packed from the top of `box`."""
    m = Image.new("L", size, 0)
    d = ImageDraw.Draw(m)
    x, y, w, _h = box
    line_h = max(u * line_units, 1.0)
    gap = max(u * gap_units, 1.0)
    pad_x = u * pad_x_units
    yy = y + u * pad_y_units
    inner = max(w - pad_x * 2, 1.0)
    radius = line_h / 2
    for frac in widths:
        lw = max(inner * frac, line_h)
        d.rounded_rectangle((x + pad_x, yy, x + pad_x + lw, yy + line_h), radius=radius, fill=255)
        yy += line_h + gap
    return m


def paint_lines(
    img: Image.Image,
    box,
    clip: Image.Image,
    u: float,
    widths: tuple[float, ...],
    line_units: float,
    gap_units: float,
    pad_x_units: float,
    pad_y_units: float,
) -> Image.Image:
    m = caption_lines_mask(img.size, box, u, widths, line_units, gap_units, pad_x_units, pad_y_units)
    m = ImageChops.multiply(m, clip)
    layer = Image.new("RGBA", img.size, (0, 0, 0, 0))
    layer.paste(Image.new("RGBA", img.size, GLYPH), (0, 0), m)
    return Image.alpha_composite(img, layer)


def render(pixel_size: int, tiny: bool) -> Image.Image:
    """`pixel_size` is the working canvas; 48 grid units span the canvas."""
    s = pixel_size
    u = s / 48.0
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))

    # Source window (square with title bar) + overlay caption hanging off
    # bottom-right. 2px exterior corners on the 48 grid.
    if tiny:
        back_box = (u * 5.0, u * 6.0, u * 29.0, u * 29.0)
        front_box = (u * 14.0, u * 26.5, u * 29.0, u * 14.0)
        title_units = 6.5
    else:
        back_box = (u * 4.5, u * 5.5, u * 30.0, u * 30.0)
        front_box = (u * 13.5, u * 26.0, u * 30.0, u * 14.5)
        title_units = 6.0
    radius = u * 2.0

    back_m = round_rect_mask((s, s), back_box, radius)
    front_m = round_rect_mask((s, s), front_box, radius)
    bar_m = title_mask((s, s), back_box, back_m, u, title_units)
    visible_client = ImageChops.subtract(ImageChops.subtract(back_m, front_m), bar_m)

    img = Image.alpha_composite(img, fill_mask(back_m, CYAN_TL, CYAN_BR))
    img = Image.alpha_composite(img, fill_mask(bar_m, TITLE_TL, TITLE_BR))

    bx, by, bw, bh = back_box
    fy = front_box[1]
    client_top = by + u * title_units
    client_box = (bx, client_top, bw, max(fy - client_top, bh * 0.3))

    img = paint_lines(
        img,
        client_box,
        visible_client,
        u,
        (0.70, 0.48) if tiny else (0.74, 0.50),
        line_units=3.4 if tiny else 2.6,
        gap_units=2.4 if tiny else 2.0,
        pad_x_units=3.4,
        pad_y_units=2.6 if tiny else 2.2,
    )

    if not tiny:
        img = Image.alpha_composite(img, shadow_on(back_m, front_m, u))
    img = Image.alpha_composite(img, fill_mask(front_m, MAGENTA_TL, MAGENTA_BR))

    img = paint_lines(
        img,
        front_box,
        front_m,
        u,
        (0.68, 0.46) if tiny else (0.70, 0.50),
        line_units=3.2 if tiny else 2.5,
        gap_units=2.2 if tiny else 2.0,
        pad_x_units=3.6,
        pad_y_units=3.2 if tiny else 3.4,
    )

    return img


def make(size: int) -> Image.Image:
    tiny = size < 28
    ss = 8 if size <= 64 else 4
    return render(size * ss, tiny).resize((size, size), Image.Resampling.LANCZOS)


def save_ico(path: Path, images: list[Image.Image]) -> None:
    pngs = []
    for im in images:
        buf = io.BytesIO()
        im.save(buf, format="PNG")
        pngs.append(buf.getvalue())
    n = len(pngs)
    offset = 6 + 16 * n
    entries = bytearray()
    payload = bytearray()
    for im, png in zip(images, pngs, strict=True):
        w = 0 if im.width >= 256 else im.width
        h = 0 if im.height >= 256 else im.height
        entries += struct.pack("<BBBBHHII", w, h, 0, 0, 1, 32, len(png), offset)
        offset += len(png)
        payload += png
    path.write_bytes(struct.pack("<HHH", 0, 1, n) + entries + payload)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    sizes = (16, 20, 24, 32, 40, 48, 64, 128, 256)
    images = [make(n) for n in sizes]
    images[-1].save(OUT / "icon.png")
    save_ico(OUT / "icon.ico", images)
    print("wrote", OUT)


if __name__ == "__main__":
    main()
