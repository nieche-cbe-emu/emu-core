
import os

def wstr(mach, addr, maxlen=260):

    if not addr:
        return ""
    out = []
    for i in range(maxlen):
        c = mach.r16(addr + i * 2)
        if c == 0:
            break
        out.append(chr(c))
    return "".join(out)

def wwrite(mach, addr, s):
    order = "little" if mach.le else "big"
    b = b"".join((c if isinstance(c, int) else ord(c)).to_bytes(2, order) for c in s)
    mach.uc.mem_write(addr, b + b"\x00\x00")
    return len(s)

class Vfs:

    def __init__(self, root, base=None):
        self.root = os.path.abspath(root)
        self.base = os.path.abspath(base) if base and os.path.isdir(base) else None
        os.makedirs(self.root, exist_ok=True)
        self.handles = {}
        self.next_h = 1
        self.glue_synthesized = []

    @staticmethod
    def _norm(path):
        p = path.replace("\\", "/").lstrip("/")
        if len(p) > 1 and p[1] == ":":
            p = p[2:].lstrip("/")
        return p

    def host_path(self, dev, path):
        return os.path.join(self.root, self._norm(path))

    def resolve(self, dev, path):

        rel = self._norm(path)
        over = os.path.join(self.root, rel)
        if os.path.exists(over):
            return over
        if self.base:
            b = os.path.join(self.base, rel)
            if os.path.exists(b):
                return b
        return over

    OPEN_FAIL = -1

    GLUE_PREFIXES = ("dfwsms", "dfwmix", "wpay", "cdlist", "cwstorecfg",
                     "wstore_host", "coolbar_list")

    @classmethod
    def is_glue_file(cls, path):
        base = cls._norm(path).split("/")[-1].lower()
        return base.startswith(cls.GLUE_PREFIXES)

    def open(self, dev, path, mode):

        m0 = (mode or "r").lower()
        hp = self.host_path(dev, path) if ("w" in m0 or "a" in m0 or "+" in m0)            else self.resolve(dev, path)
        m = (mode or "r").lower()
        if not any(c in m for c in "rwa+"):
            m = "r"
        create = "w" in m or "a" in m
        if not path or path.endswith(("\\", "/")):
            return self.OPEN_FAIL
        if not os.path.exists(hp):
            if not create and self.is_glue_file(path):

                os.makedirs(os.path.dirname(hp), exist_ok=True)
                open(hp, "wb").close()
                self.glue_synthesized.append(self._norm(path))
            elif not create:
                return self.OPEN_FAIL
            os.makedirs(os.path.dirname(hp), exist_ok=True)
            open(hp, "wb").close()
        try:
            if "w" in m:

                f = open(hp, "w+b")
            elif "a" in m:
                f = open(hp, "a+b")
            elif "+" in m:
                f = open(hp, "r+b")
            else:
                f = open(hp, "rb")
        except OSError:
            return self.OPEN_FAIL
        h = self.next_h; self.next_h += 1
        self.handles[h] = f
        return h

    def close(self, h):
        f = self.handles.pop(h, None)
        if f:
            f.close()

    def exists(self, dev, path):
        return os.path.exists(self.resolve(dev, path))

    def size(self, h):
        f = self.handles.get(h)
        if not f:
            return 0
        cur = f.tell(); f.seek(0, 2); n = f.tell(); f.seek(cur)
        return n
