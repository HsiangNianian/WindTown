"""Draw the workshop's original 16px icons: uv run --with Pillow tools/draw_editor_icons.py."""

from pathlib import Path
from PIL import Image, ImageDraw

# Order matches Icon in src/icons.rs. Draw at native resolution: no
# antialiasing, font glyphs, or external icon artwork. UI displays these at 1x/2x.
NAMES = (
    "brush", "eraser", "pick", "fill", "undo", "redo", "eye", "eye_closed",
    "zoom_in", "zoom_out", "grid", "guides", "flip_h", "flip_v", "swap", "save",
    "reload", "original", "done", "folder", "town", "shop", "left", "right",
    "layers", "tiles", "discard", "keep_editing", "settings", "language",
)
INK = "#304c43"
GREEN = "#719569"
LIGHT = "#afc69b"
CREAM = "#f7eace"
GOLD = "#ebbd60"
WOOD = "#b18d66"
BLUE = "#668d96"
PALE = "#b0c8be"
ROSE = "#cf8f79"


def draw_icon(name):
    im = Image.new("RGBA", (16, 16))
    d = ImageDraw.Draw(im)

    if name == "brush":
        d.polygon([(6, 8), (11, 2), (13, 2), (14, 3), (8, 10)], fill=INK)
        d.line((8, 8, 12, 3), fill=WOOD, width=2)
        d.line((8, 7, 11, 3), fill=GOLD)
        d.polygon([(5, 7), (9, 10), (8, 13), (5, 14), (1, 14), (3, 12), (3, 9)], fill=INK)
        d.polygon([(5, 8), (8, 10), (7, 12), (3, 13), (4, 10)], fill=BLUE)
        d.point((5, 10), fill=PALE)
        d.line((6, 7, 9, 10), fill=CREAM)
    elif name == "eraser":
        d.polygon([(2, 8), (8, 2), (10, 2), (14, 6), (14, 8), (8, 14), (6, 14), (2, 10)], fill=INK)
        d.polygon([(3, 8), (8, 3), (10, 3), (13, 6), (8, 11), (6, 11)], fill=ROSE)
        d.line((4, 8, 8, 4, 10, 4), fill=CREAM)
        d.polygon([(3, 9), (6, 12), (8, 12), (10, 10), (12, 10), (8, 13), (6, 13)], fill=PALE)
        d.line((11, 14, 14, 14), fill=WOOD)
    elif name == "pick":
        d.polygon([(8, 3), (10, 1), (12, 1), (14, 3), (14, 5), (12, 7)], fill=INK)
        d.line((10, 3, 12, 5), fill=GREEN, width=2)
        d.line((7, 4, 12, 9), fill=INK, width=2)
        d.polygon([(7, 6), (9, 8), (4, 13), (1, 14), (2, 11)], fill=INK)
        d.line((7, 7, 3, 11), fill=PALE)
        d.line((7, 9, 4, 12), fill=BLUE)
        d.point((2, 13), fill=GOLD)
    elif name == "fill":
        d.line((4, 5, 4, 2, 7, 1, 10, 4), fill=INK)
        d.polygon([(1, 8), (7, 3), (12, 8), (7, 13), (5, 13)], fill=INK)
        d.polygon([(3, 8), (7, 5), (10, 8), (7, 11), (5, 11)], fill=BLUE)
        d.line((3, 8, 7, 5, 10, 8), fill=PALE)
        d.line((7, 3, 12, 8), fill=CREAM)
        d.polygon([(13, 9), (15, 12), (15, 14), (12, 14), (12, 12)], fill=INK)
        d.line((13, 11, 13, 13), fill=BLUE)
    elif name in ("undo", "redo", "reload"):
        d.line((4, 5, 10, 5, 13, 8, 13, 11, 10, 13, 7, 13), fill=INK, width=3)
        d.line((5, 5, 10, 5, 12, 8, 12, 11, 10, 12, 7, 12), fill=GREEN)
        d.polygon([(1, 5), (6, 1), (6, 9)], fill=INK)
        d.polygon([(3, 5), (5, 3), (5, 7)], fill=GOLD)
        if name == "redo":
            im = im.transpose(Image.Transpose.FLIP_LEFT_RIGHT)
        elif name == "reload":
            d.polygon([(15, 10), (10, 6), (10, 14)], fill=INK)
            d.polygon([(13, 10), (11, 8), (11, 12)], fill=GOLD)
    elif name in ("eye", "eye_closed"):
        d.polygon([(0, 8), (4, 4), (11, 4), (15, 8), (11, 12), (4, 12)], fill=INK)
        d.polygon([(2, 8), (5, 5), (10, 5), (13, 8), (10, 11), (5, 11)], fill=CREAM)
        d.rectangle((6, 5, 9, 11), fill=GREEN)
        d.rectangle((7, 6, 8, 10), fill=INK)
        d.point((7, 6), fill=CREAM)
        if name == "eye_closed":
            d.line((3, 1, 14, 12), fill=CREAM, width=3)
            d.line((2, 2, 13, 13), fill=INK, width=2)
    elif name in ("zoom_in", "zoom_out"):
        d.rectangle((3, 1, 8, 11), fill=INK)
        d.rectangle((1, 3, 10, 9), fill=INK)
        d.rectangle((3, 3, 8, 9), fill=PALE)
        d.rectangle((2, 4, 9, 8), fill=PALE)
        d.line((9, 10, 13, 14), fill=INK, width=4)
        d.line((10, 10, 14, 14), fill=WOOD, width=2)
        d.line((3, 6, 8, 6), fill=INK)
        if name == "zoom_in":
            d.line((5, 4, 5, 8), fill=INK)
    elif name in ("grid", "tiles"):
        d.rectangle((1, 1, 14, 14), fill=INK)
        for y in (2, 6, 10):
            for x in (2, 6, 10):
                d.rectangle((x, y, x + 2, y + 2), fill=CREAM if name == "grid" else GREEN)
        if name == "tiles":
            d.rectangle((6, 6, 8, 8), fill=GOLD)
            d.rectangle((10, 2, 12, 4), fill=BLUE)
    elif name == "guides":
        d.rectangle((1, 1, 14, 5), fill=INK)
        d.rectangle((1, 1, 5, 14), fill=INK)
        d.rectangle((2, 2, 13, 4), fill=GOLD)
        d.rectangle((2, 2, 4, 13), fill=GOLD)
        for n in (4, 7, 10, 13):
            d.point((n, 4), fill=INK)
            d.point((4, n), fill=INK)
        d.line((8, 11, 14, 11), fill=GREEN)
        d.line((11, 8, 11, 14), fill=GREEN)
    elif name in ("flip_h", "flip_v", "swap"):
        if name == "swap":
            d.line((2, 13, 13, 2), fill=WOOD)
            d.polygon([(2, 2), (8, 2), (2, 8)], fill=INK)
            d.polygon([(3, 3), (6, 3), (3, 6)], fill=GREEN)
            d.polygon([(13, 13), (7, 13), (13, 7)], fill=INK)
            d.polygon([(12, 12), (9, 12), (12, 9)], fill=GOLD)
        else:
            d.line((7, 1, 7, 14), fill=WOOD)
            d.polygon([(1, 4), (5, 8), (1, 12)], fill=INK)
            d.polygon([(2, 6), (4, 8), (2, 10)], fill=GREEN)
            d.polygon([(14, 4), (10, 8), (14, 12)], fill=INK)
            d.polygon([(13, 6), (11, 8), (13, 10)], fill=GOLD)
            if name == "flip_v":
                im = im.transpose(Image.Transpose.ROTATE_90)
    elif name == "save":
        d.polygon([(2, 1), (11, 1), (14, 4), (14, 14), (1, 14), (1, 2)], fill=INK)
        d.rectangle((2, 2, 13, 13), fill=GREEN)
        d.rectangle((4, 2, 10, 6), fill=CREAM)
        d.rectangle((8, 2, 9, 5), fill=INK)
        d.rectangle((4, 9, 11, 13), fill=CREAM)
        d.line((5, 10, 10, 10), fill=WOOD)
        d.line((5, 12, 10, 12), fill=WOOD)
    elif name == "original":
        d.polygon([(2, 2), (7, 3), (11, 1), (14, 2), (14, 13), (10, 12), (6, 14), (1, 13)], fill=INK)
        d.polygon([(3, 3), (7, 4), (11, 2), (13, 3), (13, 11), (10, 11), (6, 13), (2, 12)], fill=CREAM)
        d.line((6, 5, 6, 12), fill=WOOD)
        d.line((10, 3, 10, 10), fill=WOOD)
        d.rectangle((3, 5, 4, 8), fill=GREEN)
        d.line((8, 9, 11, 6, 12, 6), fill=BLUE)
    elif name in ("done", "left", "right"):
        d.polygon([(1, 6), (8, 6), (8, 2), (14, 8), (8, 14), (8, 10), (1, 10)], fill=INK)
        d.polygon([(2, 7), (9, 7), (9, 5), (12, 8), (9, 11), (9, 9), (2, 9)], fill=GREEN)
        if name == "left":
            im = im.transpose(Image.Transpose.FLIP_LEFT_RIGHT)
        elif name == "done":
            d.line((2, 3, 2, 1, 13, 1, 13, 3), fill=WOOD)
    elif name == "folder":
        d.polygon([(1, 3), (6, 3), (8, 5), (14, 5), (14, 14), (1, 14)], fill=INK)
        d.rectangle((2, 4, 5, 6), fill=GOLD)
        d.rectangle((2, 6, 13, 13), fill=WOOD)
        d.polygon([(4, 7), (15, 7), (13, 13), (2, 13)], fill=INK)
        d.polygon([(5, 8), (14, 8), (12, 12), (4, 12)], fill=GOLD)
    elif name in ("town", "shop"):
        d.rectangle((3, 6, 12, 14), fill=INK)
        d.rectangle((4, 7, 11, 13), fill=CREAM)
        d.rectangle((8, 10, 10, 14), fill=INK)
        d.rectangle((5, 9, 6, 11), fill=BLUE)
        if name == "town":
            d.polygon([(1, 7), (7, 1), (14, 7)], fill=INK)
            d.polygon([(4, 6), (7, 3), (11, 6)], fill=GREEN)
            d.rectangle((11, 2, 12, 4), fill=INK)
        else:
            d.rectangle((3, 2, 12, 3), fill=INK)
            d.polygon([(3, 4), (12, 4), (14, 7), (1, 7)], fill=INK)
            for x in (3, 7, 11):
                d.rectangle((x, 4, x + 1, 6), fill=GREEN)
                d.rectangle((x + 2, 4, x + 3, 6), fill=GOLD)
    elif name == "layers":
        for y, color in ((9, WOOD), (6, GREEN), (3, PALE)):
            d.polygon([(1, y + 2), (7, y - 1), (14, y + 2), (8, y + 5)], fill=INK)
            d.polygon([(3, y + 2), (7, y), (12, y + 2), (8, y + 4)], fill=color)
    elif name == "discard":
        d.rectangle((4, 4, 12, 14), fill=INK)
        d.rectangle((5, 5, 11, 13), fill=ROSE)
        d.line((7, 6, 7, 12), fill=INK)
        d.line((9, 6, 9, 12), fill=INK)
        d.rectangle((3, 2, 13, 3), fill=INK)
        d.line((6, 1, 10, 1), fill=INK)
    elif name == "settings":
        d.rectangle((5, 1, 10, 14), fill=INK)
        d.rectangle((1, 5, 14, 10), fill=INK)
        d.rectangle((3, 3, 12, 12), fill=INK)
        d.rectangle((6, 2, 9, 13), fill=GREEN)
        d.rectangle((2, 6, 13, 9), fill=GREEN)
        d.rectangle((4, 4, 11, 11), fill=LIGHT)
        d.rectangle((5, 5, 10, 10), fill=INK)
        d.rectangle((6, 6, 9, 9), fill=CREAM)
        d.line((6, 6, 8, 6), fill=GOLD)
    elif name == "language":
        d.ellipse((1, 1, 14, 14), fill=INK)
        d.ellipse((2, 2, 13, 13), fill=PALE)
        d.ellipse((5, 2, 10, 13), outline=INK)
        d.line((2, 5, 13, 5), fill=INK)
        d.line((2, 10, 13, 10), fill=INK)
        d.point((11, 4), fill=CREAM)
    elif name == "keep_editing":
        d.line((2, 8, 6, 12, 14, 3), fill=INK, width=4)
        d.line((2, 7, 6, 11, 13, 3), fill=GREEN, width=2)
    else:
        raise ValueError(name)
    return im


def main():
    sheet = Image.new("RGBA", (128, 64))
    for index, name in enumerate(NAMES):
        icon = draw_icon(name)
        assert icon.getbbox(), name
        sheet.paste(icon, ((index % 8) * 16, (index // 8) * 16))
    path = Path(__file__).resolve().parents[1] / "assets/ui/editor-icons.png"
    path.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(path)
    print(path)


if __name__ == "__main__":
    main()
