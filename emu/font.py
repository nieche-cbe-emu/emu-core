
import os
import struct

MAGIC = b"CBEF"

class Font:
    def __init__(self, path):
        self._mcache = {}
        d = open(path, "rb").read()
        if d[:4] != MAGIC:
            raise ValueError(f"{path} 不是点阵字库文件")
        self.ver = struct.unpack_from("<H", d, 4)[0]
        self.aw, self.ah, self.hw, self.hh = d[6], d[7], d[8], d[9]
        n_ascii, n_hanzi = struct.unpack_from("<II", d, 10)
        self.abpr = (self.aw + 7) // 8
        self.hbpr = (self.hw + 7) // 8
        o = 18
        self.ascii = d[o:o + n_ascii * self.abpr * self.ah]
        o += len(self.ascii)
        self.hanzi = d[o:o + n_hanzi * self.hbpr * self.hh]
        self.n_hanzi = n_hanzi
        self._rows = {}

    def glyph_rows(self, is_hanzi, idx):
        key = (is_hanzi, idx)
        r = self._rows.get(key)
        if r is not None:
            return r
        if is_hanzi:
            w, h, bpr, buf = self.hw, self.hh, self.hbpr, self.hanzi
        else:
            w, h, bpr, buf = self.aw, self.ah, self.abpr, self.ascii
        base = idx * bpr * h
        rows = []
        if 0 <= base and base + bpr * h <= len(buf):
            for y in range(h):
                bits = int.from_bytes(buf[base + y * bpr:base + (y + 1) * bpr], "big")
                shift = bpr * 8 - w
                rows.append([x for x in range(w) if bits & (1 << (bpr * 8 - 1 - x))])
        else:
            rows = [[] for _ in range(h)]
        self._rows[key] = rows
        return rows

    def iter_glyphs(self, gb_bytes):

        i, n = 0, len(gb_bytes)
        while i < n:
            c = gb_bytes[i]
            if c >= 0xA1 and i + 1 < n and gb_bytes[i + 1] >= 0xA1:
                hi, lo = c - 0xA1, gb_bytes[i + 1] - 0xA1
                yield True, hi * 94 + lo, self.hw
                i += 2
            elif c >= 0x80 and i + 1 < n:
                yield True, -1, self.hw
                i += 2
            else:
                yield False, c if c < 128 else 0, self.aw
                i += 1

    def measure(self, gb_bytes):

        w = self._mcache.get(gb_bytes)
        if w is None:
            w = sum(x for _, _, x in self.iter_glyphs(gb_bytes))
            if len(self._mcache) > 4096:
                self._mcache.clear()
            self._mcache[gb_bytes] = w
        return w

    def draw(self, mach, buf, stride, sw, sh, gb_bytes, x, y, color):

        pack = "<H" if mach.le else ">H"
        col = struct.pack(pack, color & 0xFFFF)
        for is_h, idx, w in self.iter_glyphs(gb_bytes):
            if idx < 0 or x >= sw:
                x += w
                continue
            rows = self.glyph_rows(is_h, idx)
            for dy, cols in enumerate(rows):
                if not cols:
                    continue
                yy = y + dy
                if yy < 0 or yy >= sh:
                    continue
                x0, x1 = max(x, 0), min(x + w, sw)
                if x1 <= x0:
                    continue
                off = buf + (yy * stride + x0) * 2
                line = bytearray(mach.uc.mem_read(off, (x1 - x0) * 2))
                dirty = False
                for cx in cols:
                    px = x + cx
                    if x0 <= px < x1:
                        k = (px - x0) * 2
                        line[k:k + 2] = col
                        dirty = True
                if dirty:
                    mach.uc.mem_write(off, bytes(line))
            x += w

_cached = {}

def load(path=None):

    if path is None:

        here = os.path.dirname(os.path.abspath(__file__))
        path = os.environ.get("CBE_FONT") or os.path.join(here, "font12.cbef")
        if not os.path.exists(path):
            legacy = os.path.join(os.path.dirname(here), "assets", "font12.cbef")
            if os.path.exists(legacy):
                path = legacy
    if path in _cached:
        return _cached[path]
    try:
        f = Font(path)
    except Exception:
        f = None
    _cached[path] = f
    return f
