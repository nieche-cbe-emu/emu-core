
import re

SYS_SLOTS = {}

def sysapi(off, name):
    def deco(fn):
        SYS_SLOTS[off] = (name, fn)
        return fn
    return deco

_FMT = re.compile(rb'%[-+ #0]*[0-9*]*(?:\.[0-9*]+)?[hlL]*([diouxXeEfgGcsp%])')

def vm_printf(mach, fmt_ptr, argidx=1):

    fmt = mach.cstr(fmt_ptr)
    if fmt is None:
        return "<null>"
    out, pos, i = bytearray(), 0, argidx
    for m in _FMT.finditer(fmt):
        out += fmt[pos:m.start()]
        pos = m.end()
        conv = m.group(1).decode()
        if conv == '%':
            out += b'%'
            continue
        a = mach.arg(i); i += 1
        if conv == 's':
            s = mach.cstr(a) or b'<null>'
            out += s
        elif conv in 'diu':
            v = a - (1 << 32) if (conv == 'd' and a >> 31) else a
            out += str(v).encode()
        elif conv in 'xX':
            out += (f"%{conv}" % a).encode()
        elif conv == 'o':
            out += oct(a)[2:].encode()
        elif conv == 'c':
            out += bytes([a & 0xFF])
        elif conv == 'p':
            out += f"{a:#010x}".encode()
        else:
            out += b'<f>'
    out += fmt[pos:]
    return out.decode('latin1', 'replace')
