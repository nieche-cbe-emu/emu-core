
import ctypes
import json
import os
import sys

ABI = 2

_LIBNAMES = {
    "darwin": ["libnieche.dylib"],
    "win32": ["nieche.dll", "libnieche.dll"],
}

def _candidates():

    env = os.environ.get("NIECHE_LIB")
    if env:
        yield env
    names = _LIBNAMES.get(sys.platform, ["libnieche.so"])
    here = os.path.dirname(os.path.abspath(__file__))
    root = os.path.dirname(here)
    dirs = [
        here,
        root,
        os.path.join(root, "lib"),
        os.path.join(root, "rust", "target", "release"),
        os.path.join(os.environ.get("CARGO_TARGET_DIR", ""), "release"),
        os.path.join(os.path.expanduser("~"), ".cache", "nieche-rust", "release"),
    ]
    for d in dirs:
        if not d:
            continue
        for n in names:
            yield os.path.join(d, n)

class CoreUnavailable(Exception):
    pass

_lib = None

def load():

    global _lib
    if _lib is not None:
        return _lib
    last = None
    for p in _candidates():
        if not os.path.exists(p):
            continue
        try:
            lib = ctypes.CDLL(p)
        except OSError as e:
            last = e
            continue
        _bind(lib)
        v = lib.nieche_abi_version()
        if v != ABI:
            raise CoreUnavailable(f"{p} 的 ABI 是 {v}，本层要求 {ABI}")
        _lib = lib
        return lib
    raise CoreUnavailable(f"找不到核心库（最后一次错误：{last}）")

def _bind(lib):
    c = ctypes
    p, u8p, sz = c.c_void_p, c.POINTER(c.c_ubyte), c.c_size_t
    sig = [
        ("nieche_abi_version", [], c.c_uint32),
        ("nieche_open", [c.c_char_p], p),
        ("nieche_close", [p], None),
        ("nieche_boot", [p], c.c_int32),
        ("nieche_stop", [p], None),
        ("nieche_size", [p, c.POINTER(c.c_uint32), c.POINTER(c.c_uint32)], None),
        ("nieche_step", [p, u8p, sz], sz),
        ("nieche_frame_no", [p], c.c_uint64),
        ("nieche_set_keys", [p, c.c_uint32], None),
        ("nieche_set_touch", [p, c.c_int32, c.c_int32, c.c_int32], None),
        ("nieche_soft_key", [p, c.c_int32], None),
        ("nieche_nonblank", [p], c.c_uint32),
        ("nieche_screens", [p], c.c_uint32),
        ("nieche_name", [p, u8p, sz], sz),
        ("nieche_take_events", [p, u8p, sz], sz),
        ("nieche_take_logs", [p, u8p, sz], sz),
    ]
    for name, argtypes, restype in sig:
        f = getattr(lib, name)
        f.argtypes = argtypes
        f.restype = restype

_TOUCH = {"down": 0, "up": 1, "move": 2}

class NativeSession:

    def __init__(self, path, audio=True):
        self.lib = load()

        self._audio = audio
        self.h = self.lib.nieche_open(os.fspath(path).encode("utf-8"))
        if not self.h:
            raise RuntimeError(f"核心打不开模块：{path}")
        self.alive = True
        self._buf = None

    def boot(self):
        if self.lib.nieche_boot(self.h) != 1:
            raise RuntimeError("引导失败")
        return self

    def stop(self):

        if not self.alive:
            return
        self.alive = False
        self.lib.nieche_stop(self.h)

    def close(self):
        if self.h:
            self.stop()
            self.lib.nieche_close(self.h)
            self.h = None

    def __del__(self):
        try:
            self.close()
        except Exception:
            pass

    def set_keys(self, mask):
        self.lib.nieche_set_keys(self.h, ctypes.c_uint32(int(mask)))

    def set_touch(self, x, y, state):
        self.lib.nieche_set_touch(self.h, int(x), int(y), _TOUCH.get(state, 0))

    def soft_key(self, side, pressed=True):
        if pressed:
            self.lib.nieche_soft_key(self.h, 1 if side == "right" else 0)

    def step(self):

        if self._buf is None:
            self._buf = (ctypes.c_ubyte * (320 * 480 * 2))()
        n = self.lib.nieche_step(self.h, self._buf, len(self._buf))
        return bytes(memoryview(self._buf)[:n])

    def _take(self, fn):

        n = fn(self.h, None, 0)
        if not n:
            return ""
        buf = (ctypes.c_ubyte * n)()
        got = fn(self.h, buf, n)
        return bytes(memoryview(buf)[:got]).decode("utf-8", "replace")

    def take_events(self):
        s = self._take(self.lib.nieche_take_events)
        out = []
        for line in s.split("\n"):
            if not line.strip():
                continue
            try:
                out.append(json.loads(line))
            except ValueError:
                out.append({"kind": "log", "text": line})
        for e in out:
            if e.get("kind") == "exit":
                self.alive = False
        return out

    def take_events_json(self):
        return json.dumps(self.take_events(), ensure_ascii=False)

    def take_logs(self):
        return self._take(self.lib.nieche_take_logs)

    @property
    def size(self):
        w, h = ctypes.c_uint32(), ctypes.c_uint32()
        self.lib.nieche_size(self.h, ctypes.byref(w), ctypes.byref(h))
        return w.value, h.value

    @property
    def name(self):
        return self._take(self.lib.nieche_name)

    @property
    def screens(self):
        return self.lib.nieche_screens(self.h)

    @property
    def nonblank(self):
        return self.lib.nieche_nonblank(self.h)

    @property
    def frame_no(self):
        return self.lib.nieche_frame_no(self.h)

def open_session(path, audio=True, prefer_native=True):

    if prefer_native and os.environ.get("NIECHE_CORE") != "python":
        try:
            return NativeSession(path, audio=audio), "rust"
        except (CoreUnavailable, OSError, RuntimeError):
            pass
    from emu.host import Session
    return Session(path, audio=audio), "python"
