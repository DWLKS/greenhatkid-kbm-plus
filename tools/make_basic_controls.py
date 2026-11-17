r"""Draws the keyboard and mouse pictures of the game's "Basic Controls" screen and writes them to
native/res/basic-controls.bin.zst, which the app puts into the game's texture archive. The guide lines from the
labels to the keys are not drawn here: the app draws them for the player's own keys (native/src/guides.rs).

    pip install -r tools/requirements.txt
    python tools/make_basic_controls.py <game-cache folder>

The game-cache folder is the one the app fills from your own game file the first time it makes the in-game key
icons: %LOCALAPPDATA%\GreenHatKid-KBM-Plus\game-cache. Nothing from the game is stored in this repository.

Output records: u16 name length, name, u32 size, the texture's bytes as the archive holds them (swizzled ASTC)."""
import math
import os
import struct
import sys
from collections import deque

import astc_encoder as A
import numpy as np
import zstandard
from PIL import Image, ImageDraw, ImageFilter, ImageFont

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(REPO, 'native', 'res', 'basic-controls.bin.zst')

# ---- placement inside the 1160x680 picture; must match KB_SC / KB_ORG / MS_SC / MS_ORG in native/src/guides.rs ----
PW, PH = 1160, 680
KB_SC, KB_ORG = 1.22, (127, 216)           # keyboard picture point (17, 22) lands here, scaled
MS_SC, MS_ORG = 1.08, (831, 131)          # top-left of the mouse body (MS_BBOX) lands here, scaled
MS_BBOX = (36, 81, 223, 390)


# ---- the source drawings: an ANSI keyboard (570x244) and a mouse seen from above (271x419) ----------------------
# keyboard rows: (y0, y1, [(x0, x1, label), ...])
KEY_ROWS = [
    (22, 40, [(17, 49, "Esc"), (56, 88, "F1"), (95, 127, "F2"), (133, 166, "F3"), (172, 204, "F4"), (211, 243, "F5"),
              (250, 282, "F6"), (288, 321, "F7"), (327, 359, "F8"), (366, 398, "F9"), (405, 437, "F10"), (443, 475, "F11"),
              (482, 514, "F12"), (521, 553, "PrtSc")]),
    (46, 76, [(17, 47, "~"), (55, 85, "1"), (92, 122, "2"), (130, 159, "3"), (167, 197, "4"), (205, 234, "5"), (242, 272, "6"),
              (280, 309, "7"), (317, 347, "8"), (354, 384, "9"), (392, 422, "0"), (429, 459, "-"), (467, 497, "="),
              (504, 553, "Backspace")]),
    (82, 112, [(17, 66, "Tab"), (74, 103, "Q"), (111, 141, "W"), (148, 178, "E"), (186, 216, "R"), (223, 253, "T"),
               (261, 291, "Y"), (298, 328, "U"), (336, 365, "I"), (373, 403, "O"), (411, 440, "P"), (448, 478, "["),
               (486, 515, "]"), (523, 553, "\\")]),
    (118, 148, [(17, 79, "Caps"), (86, 116, "A"), (124, 153, "S"), (161, 191, "D"), (198, 228, "F"), (236, 266, "G"),
                (273, 303, "H"), (311, 340, "J"), (348, 378, "K"), (386, 415, "L"), (423, 453, ";"), (461, 490, "'"),
                (498, 553, "Enter")]),
    (154, 184, [(17, 97, "Shift"), (104, 134, "Z"), (142, 171, "X"), (179, 209, "C"), (217, 246, "V"), (254, 284, "B"),
                (292, 321, "N"), (329, 359, "M"), (366, 396, ","), (404, 434, "."), (441, 471, "/"), (479, 553, "Shift")]),
    (190, 226, [(17, 47, "Ctrl"), (55, 84, ""), (92, 122, "Fn"), (130, 172, "Alt"), (179, 359, ""), (367, 409, "Alt"),
                (416, 446, "Ctrl")]),
]
ARROWS = [(452, 210, 482, 226, "<"), (488, 190, 518, 206, "^"), (488, 210, 518, 226, "v"), (523, 210, 553, 226, ">")]
KEY_FILL, KEY_TEXT = (35, 31, 32), (255, 255, 255)


def font(size):
    return ImageFont.load_default(size=size)  # Pillow's built-in vector font (Pillow >= 10.1)


def keyboard_drawing():
    im = Image.new("RGB", (570, 244), (255, 255, 255))
    d = ImageDraw.Draw(im)
    for y0, y1, keys in KEY_ROWS:
        for x0, x1, label in keys:
            d.rounded_rectangle((x0, y0, x1, y1), radius=4, fill=KEY_FILL)
            if label:
                size = 10 if len(label) > 2 else 15
                if y1 - y0 < 20:
                    size = 9
                f = font(size)
                bb = d.textbbox((0, 0), label, font=f)
                d.text(((x0 + x1) / 2 - (bb[0] + bb[2]) / 2, (y0 + y1) / 2 - (bb[1] + bb[3]) / 2), label, font=f, fill=KEY_TEXT)
    for x0, y0, x1, y1, ch in ARROWS:
        d.rounded_rectangle((x0, y0, x1, y1), radius=3, fill=KEY_FILL)
        cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
        tri = {"<": [(cx + 4, cy - 5), (cx + 4, cy + 5), (cx - 4, cy)], ">": [(cx - 4, cy - 5), (cx - 4, cy + 5), (cx + 4, cy)],
               "^": [(cx - 5, cy + 3), (cx + 5, cy + 3), (cx, cy - 4)], "v": [(cx - 5, cy - 3), (cx + 5, cy - 3), (cx, cy + 4)]}[ch]
        d.polygon(tri, fill=KEY_TEXT)
    return im


def mouse_drawing():
    im = Image.new("RGB", (271, 419), (255, 255, 255))
    d = ImageDraw.Draw(im)
    body, ink = (140, 140, 140), (52, 52, 52)
    d.rounded_rectangle(MS_BBOX, radius=92, fill=body, outline=ink, width=3)
    d.line([(135, 84), (135, 208)], fill=ink, width=3)                      # between the buttons
    d.line([(60, 213), (210, 213)], fill=ink, width=3)
    d.rounded_rectangle((122, 108, 148, 152), radius=11, fill=(72, 72, 72), outline=(34, 34, 34), width=2)   # wheel
    return im


# ---- the drawings in the screen's style: light strokes on dark shapes, soft edges ---------------------------------
NEIGHBOURS = ((1, 0), (-1, 0), (0, 1), (0, -1))


def flood(mask, starts):
    """pixels of `mask` connected to `starts`"""
    h, w = mask.shape
    out = np.zeros_like(mask)
    q = deque()
    for y, x in starts:
        if mask[y, x] and not out[y, x]:
            out[y, x] = True
            q.append((y, x))
    while q:
        y, x = q.popleft()
        for dy, dx in NEIGHBOURS:
            ny, nx = y + dy, x + dx
            if 0 <= ny < h and 0 <= nx < w and mask[ny, nx] and not out[ny, nx]:
                out[ny, nx] = True
                q.append((ny, nx))
    return out


def regions(mask):
    """label of every pixel of `mask` (0 = not in it) and the pixel count of each label"""
    h, w = mask.shape
    lab = np.zeros((h, w), np.int32)
    sizes = [0]
    for y0 in range(h):
        for x0 in range(w):
            if mask[y0, x0] and not lab[y0, x0]:
                n = len(sizes)
                lab[y0, x0] = n
                q = deque([(y0, x0)])
                count = 0
                while q:
                    cy, cx = q.popleft()
                    count += 1
                    for dy, dx in NEIGHBOURS:
                        ny, nx = cy + dy, cx + dx
                        if 0 <= ny < h and 0 <= nx < w and mask[ny, nx] and not lab[ny, nx]:
                            lab[ny, nx] = n
                            q.append((ny, nx))
                sizes.append(count)
    return lab, sizes


def tinted(val, alpha):
    rgba = np.zeros(val.shape + (4,), np.uint8)
    rgba[..., 0] = val
    rgba[..., 1] = val
    rgba[..., 2] = val * 0.92
    rgba[..., 3] = alpha
    return Image.fromarray(rgba, "RGBA")


def keyboard_art(dark):
    lum = np.asarray(keyboard_drawing()).astype(np.float32).mean(2)
    # background = the big bright regions (outside and the gaps between keys); letters = small bright islands
    lab, sizes = regions(lum > 200)
    background = np.array([s > 900 for s in sizes])[lab] & (lab > 0)
    mask = ~background                                              # keys and their letters
    m_img = Image.fromarray(mask.astype(np.uint8) * 255)
    edge = mask & ~(np.asarray(m_img.filter(ImageFilter.MinFilter(3))) > 0)   # 1 px rim: no light fringe
    body, letter = (28, 226) if dark else (74, 244)
    t = np.clip((lum - 40) / 200.0, 0, 1)
    val = body + t * (letter - body)
    val[edge] = body
    return tinted(val, np.asarray(m_img.filter(ImageFilter.GaussianBlur(0.6))))


def mouse_art(dark):
    a = np.asarray(mouse_drawing()).astype(np.float32)
    r, g, b = a[..., 0], a[..., 1], a[..., 2]
    neutral = (np.abs(r - g) < 14) & (np.abs(g - b) < 14) & (r < 215)
    lab, sizes = regions(neutral)
    silhouette = lab == int(np.argmax(sizes))                       # the largest grey shape: the mouse
    h, w = silhouette.shape
    border = [(y, x) for x in range(w) for y in (0, h - 1)] + [(y, x) for y in range(h) for x in (0, w - 1)]
    filled = ~flood(~silhouette, border)                            # holes filled (wheel, button lines)
    lum = a.mean(2)
    lo, hi = (30, 170) if dark else (76, 215)
    val = np.clip(lo + (150 - lum) * 1.25, lo, hi)                  # dark outlines -> light strokes, body -> dark
    m_img = Image.fromarray(filled.astype(np.uint8) * 255)
    rim = filled & ~(np.asarray(m_img.filter(ImageFilter.MinFilter(5))) > 0)
    val[rim] = np.maximum(val[rim], lo + 70)                        # a lighter rim around the silhouette
    return tinted(val, np.asarray(m_img.filter(ImageFilter.GaussianBlur(0.8))))


def picture(dark):
    out = Image.new("RGBA", (PW, PH), (0, 0, 0, 0))
    k = keyboard_art(dark).crop((14, 18, 557, 230))
    k = k.resize((int(round(k.width * KB_SC)), int(round(k.height * KB_SC))), Image.LANCZOS)
    out.alpha_composite(k, (int(round(KB_ORG[0] - 3 * KB_SC)), int(round(KB_ORG[1] - 4 * KB_SC))))
    mo = mouse_art(dark).crop((MS_BBOX[0] - 4, MS_BBOX[1] - 4, MS_BBOX[2] + 5, MS_BBOX[3] + 5))
    mo = mo.resize((int(round(mo.width * MS_SC)), int(round(mo.height * MS_SC))), Image.LANCZOS)
    out.alpha_composite(mo, (int(round(MS_ORG[0] - 4 * MS_SC)), int(round(MS_ORG[1] - 4 * MS_SC))))
    return out


# ---- the game's texture archive -------------------------------------------------------------------------------
def sarc_files(data):
    """files of a SARC archive: name -> bytes"""
    hdr_len = struct.unpack_from('<H', data, 4)[0]
    data_off = struct.unpack_from('<I', data, 0xC)[0]
    count = struct.unpack_from('<H', data, hdr_len + 6)[0]
    names = hdr_len + 0xC + count * 16 + 8
    out = {}
    for i in range(count):
        _, attr, s, e = struct.unpack_from('<IIII', data, hdr_len + 0xC + i * 16)
        at = names + (attr & 0xFFFF) * 4
        out[data[at:data.index(b'\0', at)].decode()] = data[data_off + s:data_off + e]
    return out


def textures(b):
    """every texture header in the archive: name -> its format, size, layout and data place"""
    out = {}
    at = b.find(b"BRTI")
    while at >= 0:
        mips = struct.unpack_from('<H', b, at + 0x16)[0]
        fmt = struct.unpack_from('<I', b, at + 0x1C)[0]
        w, h = struct.unpack_from('<ii', b, at + 0x24)
        layout = struct.unpack_from('<I', b, at + 0x34)[0]
        offs = list(struct.unpack_from('<%dQ' % mips, b, struct.unpack_from('<Q', b, at + 0x70)[0]))
        end = struct.unpack_from('<I', b, at + 0x50)[0] + offs[0]
        name_at = struct.unpack_from('<Q', b, at + 0x60)[0]
        n = struct.unpack_from('<H', b, name_at)[0]
        out[b[name_at + 2:name_at + 2 + n].decode()] = dict(
            mips=mips, fmt=fmt, w=w, h=h, bh_log2=layout & 7, off=offs[0], size=end - offs[0])
        at = b.find(b"BRTI", at + 4)
    return out


def block_height(hb, bh):
    """block height (in GOBs) the console uses for a level `hb` blocks tall"""
    while bh > 1 and hb <= (bh // 2) * 8:
        bh //= 2
    return bh


def swizzle(lin, wb, hb, bh, size):
    """16-byte blocks, row by row -> the block-linear layout (this game's GOB order: y0, y1, x0, y2, x1)"""
    gw = math.ceil(wb * 16 / 64)
    y = np.repeat(np.arange(hb), wb)
    x = np.tile(np.arange(wb), hb)
    gob = (y // (8 * bh)) * 512 * bh * gw + (x * 16 // 64) * 512 * bh + ((y % (8 * bh)) // 8) * 512
    idx = gob + (y & 1) * 16 + ((y >> 1) & 1) * 32 + (x & 1) * 64 + ((y >> 2) & 1) * 128 + ((x >> 1) & 1) * 256
    out = np.zeros(size, np.uint8)
    src = np.frombuffer(lin, np.uint8).reshape(-1, 16)
    for i in range(16):
        out[idx + i] = src[:, i]
    return out.tobytes()


def astc_texture(t, img):
    """the picture as the texture's bytes: ASTC 4x4, sRGB, edge-extended to whole blocks"""
    assert t['mips'] == 1 and t['fmt'] == 0x2d06, (t['mips'], hex(t['fmt']))
    wb, hb = math.ceil(t['w'] / 4), math.ceil(t['h'] / 4)
    arr = np.array(img.convert('RGBA'))
    padded = Image.fromarray(np.pad(arr, ((0, hb * 4 - t['h']), (0, wb * 4 - t['w']), (0, 0)), mode='edge'))
    cfg = A.ASTCConfig(A.ASTCProfile.LDR_SRGB, 4, 4, 1, A.ASTCQualityPreset.MEDIUM)
    lin = A.ASTCContext(cfg).compress(A.ASTCImage(A.ASTCType.U8, wb * 4, hb * 4, 1, padded.tobytes()), A.ASTCSwizzle())
    data = swizzle(lin, wb, hb, block_height(hb, 1 << t['bh_log2']), t['size'])
    assert len(data) == t['size']
    return data


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    cache = sys.argv[1]
    packed = open(os.path.join(cache, 'ZsDic.pack.zs'), 'rb').read()
    dictionary = sarc_files(zstandard.ZstdDecompressor().decompress(packed, max_output_size=1 << 26))['zs.zsdic']
    unpack = zstandard.ZstdDecompressor(dict_data=zstandard.ZstdCompressionDict(dictionary))
    archive = unpack.decompress(open(os.path.join(cache, 'Common.bntx.zs'), 'rb').read(), max_output_size=1 << 28)
    tex = textures(archive)
    out = b''
    # Controller_Pro is the light version of the picture, Controller_Pro1 the dark one
    for name, dark in (('Controller_Pro', False), ('Controller_Pro1', True)):
        data = astc_texture(tex[name], picture(dark))
        out += struct.pack('<H', len(name)) + name.encode() + struct.pack('<I', len(data)) + data
    open(OUT, 'wb').write(zstandard.ZstdCompressor(level=19).compress(out))
    print(OUT, os.path.getsize(OUT))


if __name__ == '__main__':
    main()
