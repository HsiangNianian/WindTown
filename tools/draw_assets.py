"""Rebuild the original pixel art: uv run --with Pillow tools/draw_assets.py."""

from pathlib import Path
import math
import random
from PIL import Image, ImageDraw, ImageFont

OUT = Path(__file__).resolve().parents[1] / "assets"
random.seed(27)


def save(im, name):
    im.save(OUT / name)
    assert im.getbbox(), name


def canvas(w, h, color=(0, 0, 0, 0)):
    im = Image.new("RGBA", (w, h), color)
    return im, ImageDraw.Draw(im)


# All shapes are drawn at their actual in-game resolution, without antialiasing.
sky, d = canvas(480, 270)
for y in range(270):
    t = min(y / 240, 1)
    a, b = (103, 146, 153), (250, 204, 152)
    color = tuple(round(a[i] * (1 - t) + b[i] * t) for i in range(3))
    d.line((0, y, 480, y), fill=color)
d.ellipse((343, 52, 383, 92), fill="#fbe7b2")
d.rectangle((340, 78, 388, 80), fill="#ebc69b")
d.rectangle((352, 86, 393, 88), fill="#edc79c")
save(sky, "sky.png")

hills, d = canvas(1600, 270)
for base, height, color, phase in [
    (155, 27, "#8bada4", 0),
    (183, 33, "#719689", 2),
    (210, 30, "#587c72", 4),
]:
    points = [(x, int(base + math.sin(x / 92 + phase) * height + math.sin(x / 37) * 5))
              for x in range(0, 1601, 4)]
    d.polygon(points + [(1600, 270), (0, 270)], fill=color)
for x in range(0, 1600, 19):
    y = int(196 + math.sin(x / 75) * 9)
    d.rectangle((x + 3, y, x + 4, y + 26), fill="#52746a")
    d.polygon([(x - 3, y + 9), (x + 3, y - 8), (x + 10, y + 9)], fill="#52746a")
save(hills, "hills.png")

cloud, d = canvas(96, 24)
for box in [(10, 12, 89, 18), (19, 7, 71, 19), (27, 3, 46, 18), (45, 6, 61, 19)]:
    d.rectangle(box, fill="#f4dcc1")
d.rectangle((16, 19, 78, 21), fill="#d8c4b0")
save(cloud, "cloud.png")

town, d = canvas(1440, 192)
GROUND = 175
font_path = OUT / "fonts/fusion-pixel.ttf"
font = ImageFont.truetype(str(font_path), 12) if font_path.exists() else ImageFont.load_default()


def rect(box, color):
    d.rectangle(tuple(int(v) for v in box), fill=color)


def tree(x, y=175, size=1.0, autumn=False):
    dark, mid, light, shine = ("#586349", "#8a8a50", "#b2a361", "#d0b773") if autumn else (
        "#355a50", "#4c7960", "#719569", "#92ad73")
    rect((x - 5, y - 65 * size, x + 5, y), "#685847")
    rect((x + 2, y - 66 * size, x + 5, y), "#9b7958")
    d.line((x, y - 27, x - 20 * size, y - 66 * size), fill="#685847", width=4)
    d.line((x, y - 40, x + 22 * size, y - 76 * size), fill="#685847", width=4)
    for ox, oy, r in [(-22, -70, 25), (18, -77, 29), (0, -103, 29), (-31, -91, 19), (32, -94, 18)]:
        cx, cy, r = x + ox * size, y + oy * size, r * size
        d.ellipse((int(cx-r), int(cy-r), int(cx+r), int(cy+r)), fill=dark)
        d.ellipse((int(cx-r), int(cy-r), int(cx+r-4), int(cy+r-7)), fill=mid)
        d.ellipse((int(cx-r+3), int(cy-r+2), int(cx+r-9), int(cy+3)), fill=light)
    for _ in range(int(90 * size)):
        xx = x + random.randint(int(-43*size), int(43*size))
        yy = y + random.randint(int(-120*size), int(-64*size))
        if 0 <= xx < town.width and 0 <= yy < town.height and town.getpixel((xx, yy))[:3] in [
            tuple(bytes.fromhex(c[1:])) for c in [dark, mid, light]
        ]:
            rect((xx, yy, xx+2, yy+1), random.choice([light, shine, mid]))
    rect((x - 9, y - 2, x + 9, y), "#466052")


def bush(x, y, width=30):
    for dx in range(0, width, 8):
        r = random.randint(7, 12)
        d.ellipse((x+dx-r, y-r*2, x+dx+r, y), fill="#426956")
        d.ellipse((x+dx-r, y-r*2, x+dx+r-3, y-4), fill="#739568")
        rect((x+dx-3, y-r*2+3, x+dx+1, y-r*2+4), "#b0b576")


def window(x, y, w=24, h=28):
    rect((x-2, y-2, x+w+2, y+h+2), "#755d50")
    rect((x, y, x+w, y+h), "#eeb97c")
    rect((x+2, y+2, x+w-2, y+h-2), "#ffda91")
    rect((x+3, y+3, x+6, y+h-3), "#ffeac2")
    rect((x+w//2, y, x+w//2+1, y+h), "#987355")
    rect((x, y+h//2, x+w, y+h//2+1), "#987355")
    rect((x-4, y+h+3, x+w+4, y+h+5), "#556c5c")


def house(x, width=128, roof="#ad6953", wall="#ead3a7", label="COFFEE"):
    y, top = 175, 79
    rect((x+6, top+12, x+width-6, y), "#7a7160")
    rect((x+8, top+16, x+width-9, y-5), wall)
    rect((x+width-20, top+15, x+width-9, y-4), "#c2ad87")
    for yy in range(top+29, y-6, 13):
        rect((x+9, yy, x+width-21, yy), "#d7c098")
    rect((x+width-29, top-36, x+width-17, top-4), "#98705b")
    rect((x+width-31, top-36, x+width-15, top-32), "#cfaa81")
    d.polygon([(x-4, top+18), (x+21, top-21), (x+width-28, top-21), (x+width+3, top+18)], fill="#684f48")
    d.polygon([(x-1, top+12), (x+22, top-23), (x+width-28, top-23), (x+width, top+12)], fill=roof)
    for yy in range(top-20, top+13, 6):
        inset = int((top+13-yy)*0.68)
        rect((x+inset, yy, x+width-inset, yy+1), "#d89266")
        for xx in range(x+inset+3, x+width-inset, 13):
            rect((xx, yy+2, xx+1, yy+4), "#945947")
    rect((x-3, top+14, x+width+3, top+18), "#6a524a")
    window(x+19, 119, 27, 28)
    window(x+width-47, 119, 27, 28)
    rect((x+width//2-12, 125, x+width//2+12, y-4), "#5e6558")
    rect((x+width//2-9, 129, x+width//2+9, y-7), "#9a9d79")
    rect((x+width//2-6, 132, x+width//2+6, 149), "#edcb8e")
    rect((x+width//2+6, 157, x+width//2+7, 158), "#ffe2a2")
    rect((x+width//2-16, y-4, x+width//2+16, y), "#c1b697")
    rect((x+20, 98, x+width-20, 113), "#536c5c")
    rect((x+22, 100, x+width-22, 100), "#95a57b")
    d.text((x+width//2, 99), label, font=font, fill="#f8dfac", anchor="mt")
    for wx in [x+17, x+width-47]:
        rect((wx-3, 153, wx+30, 161), "#9d7157")
        for fx in range(wx, wx+29, 5):
            rect((fx, 148, fx+1, 154), "#527557")
            rect((fx-1, 147, fx+2, 149), random.choice(["#d18e79", "#ead1a0", "#f4b783"]))


def lamp(x):
    rect((x-2, 121, x+1, 175), "#455557")
    rect((x-5, 173, x+4, 175), "#485857")
    rect((x-5, 113, x+5, 125), "#465154")
    rect((x-3, 115, x+3, 122), "#ffdfa1")
    rect((x-7, 111, x+7, 113), "#596556")
    rect((x-3, 108, x+3, 110), "#596556")


def bench(x):
    for y in [151, 155, 163]:
        rect((x, y, x+39, y+2), "#92705a")
        rect((x, y, x+39, y), "#c49b70")
    for xx in [x+4, x+34]:
        rect((xx, 150, xx+2, 175), "#536153")


# Fence and planting form a continuous walking route.
for x in range(0, 1440, 12):
    rect((x, 156, x+3, 174), "#a8a183")
    rect((x, 157, x, 172), "#d0bd91")
rect((0, 161, 1440, 163), "#a8a183")
rect((0, 169, 1440, 170), "#8c8f72")
for x in [15, 94, 331, 426, 530, 758, 868, 1074, 1291, 1390]:
    bush(x, 173, random.randint(25, 49))
for args in [(37, 175, 1.15, False), (383, 175, .94, True), (511, 175, .82, False),
             (855, 175, 1.15, False), (1011, 175, .9, True), (1370, 175, 1.3, False)]:
    tree(*args)
house(144, 152, label="COFFEE")
house(602, 132, roof="#647f76", wall="#dcc7a1", label="FLOWERS")
house(1123, 135, roof="#797c76", wall="#d5c8af", label="POST")
for x in [103, 334, 561, 780, 1085, 1291]:
    lamp(x)
for x in [431, 914, 1300]:
    bench(x)
for x in [128, 304, 583, 741, 1105, 1261]:
    rect((x-4, 162, x+5, 174), "#ba8864")
    rect((x-6, 160, x+7, 163), "#d3a178")
    bush(x-3, 161, 6)
# Cafe awning, outdoor table, chalkboard and pennants.
for i in range(12):
    x = 146 + i*12
    d.polygon([(x, 114), (x+11, 114), (x+14, 125), (x-2, 125)], fill="#e4c594" if i%2 else "#6e8a79")
rect((144, 125, 291, 127), "#4a665b")
rect((306, 149, 324, 151), "#b18d66")
rect((314, 152, 316, 175), "#7a7259")
rect((310, 146, 313, 148), "#fff0c8")
rect((119, 151, 135, 174), "#9b7b59")
rect((121, 153, 133, 169), "#3b5953")
for y in [157, 161, 165]:
    rect((124, y, 130, y), "#ded7a8")
d.line((297, 98, 380, 119, 425, 98), fill="#56695a", width=1)
for x in range(304, 425, 13):
    y = int(99 + (x-297)*.25) if x<380 else int(119-(x-380)*.47)
    d.polygon([(x, y), (x+8, y+2), (x+4, y+10)], fill=random.choice(["#e2b277", "#cf8f70", "#99a484"]))
# Flower shop crates and a bicycle near the post office.
for x in range(756, 780, 10):
    rect((x, 164, x+8, 174), "#92705c")
    for j in range(3):
        rect((x+j*3, 152+j, x+j*3+1, 164), "#618261")
        d.ellipse((x+j*3-2, 149+j, x+j*3+3, 153+j), fill="#e7ba88")
for x in [1272, 1291]:
    d.ellipse((x-6, 161, x+6, 174), outline="#455a59", width=2)
d.line((1272, 167, 1280, 154, 1288, 167, 1272, 167, 1285, 157, 1291, 167), fill="#b8735e", width=2)
d.line((1284, 156, 1287, 151, 1292, 151), fill="#455a59", width=1)
for x in range(0, 1440, 4):
    if random.random() < .37:
        y = random.randint(170, 176)
        rect((x, y, x, 176), random.choice(["#567b57", "#96a06a", "#bdba81"]))
save(town, "town.png")

tiles, d = canvas(64, 16)
for tile in range(4):
    x = tile*16
    d.rectangle((x, 0, x+15, 15), fill="#aa9677" if tile<2 else "#7e795f")
    for _ in range(17):
        xx, yy = x+random.randrange(16), random.randrange(16)
        d.rectangle((xx, yy, min(xx+2, x+15), yy), fill=random.choice(["#91876a", "#b4a080", "#9e8d6c"]))
    if tile<2:
        d.rectangle((x, 0, x+15, 2), fill="#dbca9b")
        d.rectangle((x, 3, x+15, 3), fill="#8f8b67")
        d.line((x+15, 5, x+15, 14), fill="#968567")
    else:
        d.rectangle((x, 14, x+15, 15), fill="#6f7058")
save(tiles, "tiles.png")

sheet, d = canvas(24*6, 32*4)
for row, (shirt, bright, hair, hat) in enumerate([
    ("#ba8550", "#ebba6e", "#544843", "#e3b567"),
    ("#a86665", "#df9b82", "#3c5350", "#eee0b6"),
    ("#527c66", "#90a67a", "#805d49", "#d7bf7a"),
    ("#556c87", "#8ca4ab", "#b4ac95", "#677d8e"),
]):
    for frame in range(6):
        ox, oy = frame*24, row*32
        bob = 1 if frame in [1, 3, 5] else 0

        def r(box, color):
            x1,y1,x2,y2 = box
            d.rectangle((ox+x1, oy+y1+bob, ox+x2, oy+y2+bob), fill=color)

        stride = [0, 0, -3, -1, 3, 1][frame]
        # Shoes stay on the ground while shoulders rise and fall.
        for lx, offset, col in [(9, stride, "#3d5357"), (14, -stride, "#4c6667")]:
            d.line((ox+lx, oy+23, ox+lx+offset, oy+29), fill=col, width=3)
            d.rectangle((ox+lx+offset-1, oy+29, ox+lx+offset+3, oy+31), fill="#3b4243")
            d.point((ox+lx+offset+3, oy+30), fill="#b6ab88")
        r((6, 15, 16, 23), shirt)
        r((8, 15, 16, 20), bright)
        r((7, 21, 15, 23), shirt)
        r((5, 17, 7, 23), "#735f47")
        r((4, 18, 6, 22), "#a68455")
        arm = [0, 0, 2, 0, -2, 0][frame]
        r((15+arm, 17, 17+arm, 22), shirt)
        r((15+arm, 22, 17+arm, 24), "#e3b78b")
        r((8, 5, 17, 13), hair)
        r((10, 7, 18, 13), "#e9be91")
        r((17, 10, 19, 12), "#e9be91")
        r((16, 9, 16, 10), "#353f3e")
        r((17, 12, 18, 12), "#bd8670")
        r((8, 6, 10, 9), hair)
        r((7, 4, 17, 6), hat)
        r((9, 2, 16, 4), hat)
        r((7, 6, 20, 7), hat)
        r((9, 3, 16, 3), bright)
        r((9, 14, 17, 15), "#668f88" if row==0 else "#f0d7ac")
        if row==0:
            r((7, 15, 9, 19), "#527c77")
        elif row==1:
            r((10, 17, 15, 23), "#eccea1")
        elif row==2:
            r((9, 19, 15, 23), "#466f60")
save(sheet, "people.png")

shadow, d = canvas(22, 6)
d.ellipse((0, 0, 21, 5), fill=(44, 63, 52, 72))
save(shadow, "shadow.png")

assert sheet.size == (144, 128)
assert sheet.crop((0, 0, 24, 32)).tobytes() != sheet.crop((48, 0, 72, 32)).tobytes()
assert town.size == (1440, 192)
print("Wrote 8 original pixel-art assets; dimensions and walk frames verified.")
