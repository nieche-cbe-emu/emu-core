
import re

_GATE = re.compile(rb'\x10\xb5\x04\x1c.{4}\x01\x2c.\xd1.{4}\x01\x28.\xd0', re.S)
_CMP_OFF = 0x22
_SENTINEL = 0x7F

def offline_activate(mod, verbose=True):

    if mod.endian != "LE":
        return 0
    ro = bytearray(mod.ro)
    n = 0
    for m in _GATE.finditer(bytes(ro)):
        o = m.start() + _CMP_OFF
        if ro[o] == 0x01 and ro[o + 1] == 0x28:
            ro[o] = _SENTINEL
            n += 1
            if verbose:
                print(f"  [patch] 离线激活: RO+{o:#x}  cmp r0,#1 -> cmp r0,#{_SENTINEL:#x}")
    if n:
        mod.ro = bytes(ro)
    return n
