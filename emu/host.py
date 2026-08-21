
import io
import json
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

import cbelib
from emu.runtime import Runtime

class Session:

    REP_DELAY, REP_RATE = 0.40, 0.12

    def __init__(self, path, audio=True, budget=40_000_000):
        self.mod = cbelib.load(path)
        self.rt = Runtime(self.mod, trace=False, quiet_log=True,
                          trace_fs=False, audio=audio)
        self.rt.mach.BUDGET = budget
        self.events = []
        if audio:
            self.rt.audio.on_event = lambda e: self.events.append(dict(e, kind="audio"))
        self.keys = 0
        self.latched = 0
        self.prev_keys = 0
        self.rep_next = {}
        self.touch = None

        self.touch_queue = []
        self.frame_no = 0
        self.nlog = 0
        self.alive = True

    def boot(self):

        with _quiet():
            self.rt.boot()
            self.rt.app_start()
        return self

    def stop(self):

        if not self.alive:
            return
        self.alive = False
        try:
            with _quiet():
                self.rt.app_stop()
                self.rt.pump()
        except Exception:
            pass

    def set_keys(self, mask):
        mask = int(mask)

        self.latched |= mask & ~self.keys
        self.keys = mask

    def set_touch(self, x, y, state):
        e = (int(x), int(y), state)

        if state == "move" and self.touch_queue and self.touch_queue[-1][2] == "move":
            self.touch_queue[-1] = e
        elif len(self.touch_queue) < 64:
            self.touch_queue.append(e)

    def _apply_input(self, now):
        rt = self.rt
        bits = self.keys | self.latched
        self.latched = 0

        rt.keys_down = bits & ~self.prev_keys
        rt.keys_hold = bits
        rt.keys_up = self.prev_keys & ~bits
        rt.keys_down |= self._autorepeat(bits, now)
        self.prev_keys = bits

        if self.touch_queue:

            self.touch = self.touch_queue.pop(0)
        if self.touch:
            x, y, st = self.touch
            rt.pointer = (x, y)
            rt.touch_down = 1 if st == "down" else 0
            rt.touch_hold = 1 if st in ("down", "move") else 0
            rt.touch_up = 1 if st == "up" else 0
            rt.touch_drag = 1 if st == "move" else 0
            if st == "up":
                self.touch = None
            elif st == "down":

                self.touch = (x, y, "move")

    def _autorepeat(self, bits, now):
        out = 0
        for b in range(32):
            m = 1 << b
            if not (bits & m):
                self.rep_next.pop(b, None)
            elif b not in self.rep_next:
                self.rep_next[b] = now + self.REP_DELAY
            elif now >= self.rep_next[b]:
                self.rep_next[b] = now + self.REP_RATE
                out |= m
        return out

    def step(self):

        self._apply_input(time.time())
        try:
            self.rt.frame()
        except Exception as e:
            self.events.append({"kind": "log",
                                "text": f"[宿主错误] {type(e).__name__}: {e}"})
        self.frame_no += 1
        if self.rt.exit_requested:
            self.events.append({"kind": "exit", "why": "module"})
            self.alive = False
        logs = self.rt.logs
        if len(logs) > self.nlog:
            for ln in logs[self.nlog:]:
                self.events.append({"kind": "log", "text": ln})
            self.nlog = len(logs)
        return self.rt.fb.raw565()

    def take_events(self):

        e, self.events = self.events, []
        return e

    def take_events_json(self):
        return json.dumps(self.take_events(), ensure_ascii=False)

    @property
    def size(self):
        return (self.rt.fb.w, self.rt.fb.h)

class _quiet:

    def __enter__(self):
        self._old = sys.stdout
        sys.stdout = io.StringIO()

    def __exit__(self, *a):
        sys.stdout = self._old
        return False

def open_module(path, audio=True):

    return Session(path, audio=audio).boot()
