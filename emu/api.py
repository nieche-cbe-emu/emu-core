
import os
import struct

from emu import paths

API = {}
BY_NAME = {}
_AMBIGUOUS = object()

def impl(tag, *names):
    def deco(fn):
        for n in names:
            API[(tag, n)] = fn
            if n is not None:
                prev = BY_NAME.get(n)
                BY_NAME[n] = fn if prev in (None, fn) else _AMBIGUOUS
        return fn
    return deco

def lookup(tag, name):

    fn = API.get((tag, name))
    if fn:
        return fn
    fn = API.get((tag, None))
    if fn:
        return fn
    fn = BY_NAME.get(name)
    return None if fn is _AMBIGUOUS else fn

MEM = "VmMemoryManagerTag"
STD = "VmStdManagerTag"
LCD = "VmLcdManagerTag"
IO = "VmIoManagerTag"
TIME = "VmTimeManagerTag"
BILL = "VmBillingManagerTag"
GAME = "GameManagerOldTag"
SYS = "VmSysManagerTag"

@impl(MEM, "dF_Malloc_In", "dF_Malloc_debug")
def df_malloc_in(mc, rt):

    pp, size = mc.arg(0), mc.arg(1)
    p = mc.heap.alloc(size, "dF_Malloc", strict=False) if size else 0
    if p:
        mc.uc.mem_write(p, b"\x00" * size)
    mc.w32(pp, p)
    mc.ret(1 if p else 0)

@impl(MEM, "dF_Free")
def df_free(mc, rt):
    pp = mc.arg(0)
    if pp:
        mc.heap.free(mc.r32(pp))
        mc.w32(pp, 0)
    mc.ret(0)

@impl(MEM, "mallocBigMen", "mallocBigMen2", "mallocSysMem", "mallocBigMen_debug")
def malloc_big(mc, rt):
    size = mc.arg(0)
    p = mc.heap.alloc(size, "bigmem", strict=False) if size else 0
    if p:
        mc.uc.mem_write(p, b"\x00" * size)
    mc.ret(p)

@impl(MEM, "freeBigMen", "freeBigMen2", "freeSysMen")
def free_big(mc, rt):
    mc.heap.free(mc.arg(0)); mc.ret(0)

@impl(MEM, "RemallocBigMen")
def remalloc_big(mc, rt):
    old, size = mc.arg(0), mc.arg(1)
    p = mc.heap.alloc(size, "bigmem")
    if old:
        n = min(size, mc.heap.blocks.get(old, (0, ""))[0])
        if n:
            mc.uc.mem_write(p, bytes(mc.uc.mem_read(old, n)))
        mc.heap.free(old)
    mc.ret(p)

@impl(MEM, "mF_GetGMemoryBlockPtr")
def get_gblock(mc, rt):
    if "gblock" not in rt.state:
        rt.state["gblock"] = mc.heap.alloc(0x20, "MemoryBlock")
    mc.ret(rt.state["gblock"])

def _gblock_alloc(mc, size):
    p = mc.heap.alloc(size, "gblock", strict=False) if size else 0
    if p:
        mc.uc.mem_write(p, b"\x00" * size)
    mc.ret(p)

@impl(MEM, "mF_MallocGmemoryBlock")
def gblock_malloc(mc, rt):

    _gblock_alloc(mc, mc.arg(0))

@impl(MEM, "mF_MemoryBlock_Malloc")
def block_malloc(mc, rt):

    _gblock_alloc(mc, mc.arg(1))

@impl(MEM, "dF_InitMemory", "dF_InitMemoryEx", "mF_InitMemoryBlock",
      "mF_InitGmemoryBlock", "dF_ReleaseMemory", "mF_ReleaseGmemoryBlock",
      "mF_resetGmemoryBlock", "mF_MemoryBlock_Reset", "mF_MemoryBlock_Release",
      "dF_Memory_gc")
def mem_noop(mc, rt):
    mc.ret(0)

@impl(MEM, "getShareMemAlloced")
def share_alloced(mc, rt):
    mc.ret(0)

@impl(STD, "memcpy")
def std_memcpy(mc, rt):
    d, s, n = mc.arg(0), mc.arg(1), mc.arg(2)
    if n:
        mc.uc.mem_write(d, bytes(mc.uc.mem_read(s, n)))
    mc.ret(d)

@impl(STD, "memmove")
def std_memmove(mc, rt):
    std_memcpy(mc, rt)

@impl(STD, "memset")
def std_memset(mc, rt):
    d, c, n = mc.arg(0), mc.arg(1) & 0xFF, mc.arg(2)
    if n:
        mc.uc.mem_write(d, bytes([c]) * n)
    mc.ret(d)

@impl(STD, "strlen")
def std_strlen(mc, rt):
    mc.ret(len(mc.cstr(mc.arg(0), 0x10000) or b""))

@impl(STD, "strcpy")
def std_strcpy(mc, rt):
    s = mc.cstr(mc.arg(1)) or b""
    mc.uc.mem_write(mc.arg(0), s + b"\x00")
    mc.ret(mc.arg(0))

@impl(STD, "strncpy")
def std_strncpy(mc, rt):
    d, s, n = mc.arg(0), mc.cstr(mc.arg(1)) or b"", mc.arg(2)
    b = (s + b"\x00" * n)[:n]
    mc.uc.mem_write(d, b)
    mc.ret(d)

@impl(STD, "strcat")
def std_strcat(mc, rt):
    d = mc.arg(0)
    cur = mc.cstr(d) or b""
    mc.uc.mem_write(d + len(cur), (mc.cstr(mc.arg(1)) or b"") + b"\x00")
    mc.ret(d)

@impl(STD, "strcmp")
def std_strcmp(mc, rt):
    a, b = mc.cstr(mc.arg(0)) or b"", mc.cstr(mc.arg(1)) or b""
    mc.ret(0 if a == b else (1 if a > b else 0xFFFFFFFF))

@impl(STD, "strncmp")
def std_strncmp(mc, rt):
    n = mc.arg(2)
    a = (mc.cstr(mc.arg(0)) or b"")[:n]
    b = (mc.cstr(mc.arg(1)) or b"")[:n]
    mc.ret(0 if a == b else (1 if a > b else 0xFFFFFFFF))

@impl(STD, "memcmp")
def std_memcmp(mc, rt):
    n = mc.arg(2)
    a = bytes(mc.uc.mem_read(mc.arg(0), n)) if n else b""
    b = bytes(mc.uc.mem_read(mc.arg(1), n)) if n else b""
    mc.ret(0 if a == b else (1 if a > b else 0xFFFFFFFF))

@impl(STD, "sprintf", "vsprintf")
def std_sprintf(mc, rt):
    from .hostabi import vm_printf
    s = vm_printf(mc, mc.arg(1), argidx=2).encode("latin1", "replace")
    mc.uc.mem_write(mc.arg(0), s + b"\x00")
    mc.ret(len(s))

@impl(STD, "printf")
def std_printf(mc, rt):
    from .hostabi import vm_printf
    msg = vm_printf(mc, mc.arg(0)).rstrip()
    rt.logs.append(msg)
    if not rt.quiet_log:
        print(f"  [printf] {msg}")
    mc.ret(0)

@impl(STD, "atoi", "atol")
def std_atoi(mc, rt):
    s = (mc.cstr(mc.arg(0)) or b"").strip()
    m = b""
    for c in s:
        if c in b"+-" and not m or c in b"0123456789":
            m += bytes([c])
        else:
            break
    try:
        mc.ret(int(m))
    except ValueError:
        mc.ret(0)

@impl(STD, "rand")
@impl(GAME, "VmGetRand")
def std_rand(mc, rt):
    st = rt.state.get("rand", 0x12345678)
    st = (1103515245 * st + 12345) & 0x7FFFFFFF
    rt.state["rand"] = st
    mc.ret(st)

@impl(TIME, "VMGetTickCount")
def get_tick(mc, rt):

    n = mc.same_trap
    if n > 64:
        rt.tick += 16
    elif n > 8:
        rt.tick += 1
    mc.ret(rt.tick)

@impl(TIME, "VMGetTotalSeconds")
def get_secs(mc, rt):
    mc.ret(rt.tick // 1000)

@impl(TIME, "VMSysSleep")
def sys_sleep(mc, rt):
    rt.tick += mc.arg(0)
    mc.ret(0)

@impl(TIME, "VMStartTimer")
def start_timer(mc, rt):

    tid = rt.start_timer(mc.arg(0), mc.arg(1), mc.arg(2))
    rt.trace_io(f"VMStartTimer({mc.arg(0)}ms, cb=RO+{mc.arg(1) & ~1:#x}) -> #{tid}")
    mc.ret(tid)

@impl(TIME, "VMStopTimer")
def stop_timer(mc, rt):
    rt.stop_timer(mc.arg(0))
    mc.ret(0)

@impl(TIME, "VMGetCurrentTime")
def get_current_time(mc, rt):

    p = mc.arg(0)
    if p:
        mc.uc.mem_write(p, b"\x00" * 24)
        mc.w32(p + 0, 2013)
    mc.ret(0)

@impl(BILL, "BILLING_GetPayNumByAppId")
def billing_paynum(mc, rt):

    mc.ret(0)

@impl(BILL, None)
def billing_ok(mc, rt):

    mc.ret(1)

@impl(LCD, "VMGetLCDBuffer")
def get_lcd_buffer(mc, rt):
    mc.ret(rt.lcd_buffer())

@impl(LCD, "VMGetCurrMainScreenImage")
def get_main_image(mc, rt):
    mc.ret(rt.lcd_image())

@impl(LCD, "VM_InvalidateLcd", "vM_InvalidateLcdEx")
def invalidate(mc, rt):
    rt.present()
    mc.ret(0)

@impl(LCD, "VMGetImageWidth")
def img_w(mc, rt):
    p = mc.arg(0)
    mc.ret(mc.r16(p + 4) if p else 0)

@impl(LCD, "VMGetImageHeight")
def img_h(mc, rt):
    p = mc.arg(0)
    mc.ret(mc.r16(p + 6) if p else 0)

@impl(LCD, "VMFillRectEx")
def fill_rect_ex(mc, rt):
    x, y, w, h = (mc.arg(i) for i in range(4))
    color = mc.arg(4)
    rt.fill_rect(x, y, w, h, color)
    mc.ret(1)

@impl(LCD, "VMGetFontWidth", "vMGetFontWidthEx")
def font_w(mc, rt):

    if not rt.font:
        mc.ret(16 if mc.arg(0) else 8); return
    mc.ret(rt.font.hw if mc.arg(0) else rt.font.aw)

@impl(LCD, "VMGetCharWidth")
def char_w(mc, rt):
    c = mc.arg(0)
    if not rt.font:
        mc.ret(8); return
    mc.ret(rt.font.hw if c >= 0x80 else rt.font.aw)

@impl(LCD, "VMGetFontHeight", "vMGetFontHeightEx")
def font_h(mc, rt):
    mc.ret(rt.font.hh if rt.font else 12)

from .vfs import wstr, wwrite

def _open_mode(mc, p):

    if not p:
        return "r"
    m = (mc.cstr(p) or b"")[:8].decode("latin1", "ignore")
    if any(c in m for c in "rwa"):
        return m
    w = wstr(mc, p, 8)
    return w if any(c in w for c in "rwa") else "r"

@impl(IO, "Vm_file_open")
def file_open(mc, rt):

    dev, path, mode = mc.arg(0), wstr(mc, mc.arg(1)), _open_mode(mc, mc.arg(2))
    h = rt.vfs.open(dev, path, mode)
    rt.trace_io(f"open(dev={dev}, {path!r}, mode={mode!r}) -> {h}")
    mc.ret(h & 0xFFFFFFFF)

@impl(IO, "Vm_file_close")
def file_close(mc, rt):
    rt.vfs.close(mc.arg(0)); mc.ret(0)

@impl(IO, "Vm_file_exist")
def file_exist(mc, rt):
    dev, path = mc.arg(0), wstr(mc, mc.arg(1))
    ok = rt.vfs.exists(dev, path)
    rt.trace_io(f"exist(dev={dev}, {path!r}) -> {ok}")
    mc.ret(1 if ok else 0)

@impl(IO, "Vm_file_direxist")
def dir_exist(mc, rt):
    mc.ret(1 if rt.vfs.exists(mc.arg(0), wstr(mc, mc.arg(1))) else 0)

@impl(IO, "Vm_file_getfilesize")
def file_size(mc, rt):
    mc.ret(rt.vfs.size(mc.arg(0)))

@impl(IO, "Vm_file_read")
def file_read(mc, rt):

    buf, size, h = mc.arg(0), mc.arg(1), mc.arg(2)
    f = rt.vfs.handles.get(h)
    if not f:
        mc.ret(0); return
    data = f.read(size)
    if data:
        mc.uc.mem_write(buf, data)
    mc.ret(len(data))

@impl(IO, "Vm_file_write")
def file_write(mc, rt):
    buf, n, h = mc.arg(0), mc.arg(1), mc.arg(2)
    f = rt.vfs.handles.get(h)
    if not f:
        mc.ret(0); return
    f.write(bytes(mc.uc.mem_read(buf, n)))
    mc.ret(n)

@impl(IO, "Vm_file_seek")
def file_seek(mc, rt):
    h, off, whence = mc.arg(0), mc.arg(1), mc.arg(2)
    f = rt.vfs.handles.get(h)
    if not f:
        mc.ret(-1 & 0xFFFFFFFF); return
    if off & 0x80000000:
        off -= 1 << 32
    f.seek(off, whence)
    mc.ret(f.tell())

@impl(IO, "Vm_file_tell")
def file_tell(mc, rt):
    f = rt.vfs.handles.get(mc.arg(0))
    mc.ret(f.tell() if f else 0)

@impl(IO, "Vm_get_freespace", "Vm_get_freespace_ex")
def freespace(mc, rt):
    mc.ret(64 * 1024 * 1024)

SCR = "VmScreenManagerTag"

@impl(SCR, "vmAddScreen")
def add_screen(mc, rt):
    rt.push_screen(mc.arg(0), 0, 1); mc.ret(0)

@impl(SCR, "vmAddScreenEx")
def add_screen_ex(mc, rt):
    rt.push_screen(mc.arg(0), mc.arg(1), mc.arg(2)); mc.ret(0)

@impl(SCR, "vmChangeScreen")
def change_screen(mc, rt):
    rt.screens.clear(); rt.push_screen(mc.arg(0), 0, 1); mc.ret(0)

@impl(SCR, "vmChangeScreenEx", "vmSCREEN_ChangeScreen")
def change_screen_ex(mc, rt):
    rt.screens.clear(); rt.push_screen(mc.arg(0), mc.arg(1), mc.arg(2)); mc.ret(0)

@impl(SCR, "vmDeleteScreen")
def del_screen(mc, rt):
    rt.screens = [s for s in rt.screens if s[0] != mc.arg(0)]
    mc.ret(0)

@impl(SCR, "vmIsScreenFocus")
def is_focus(mc, rt):
    mc.ret(1 if rt.screens and rt.screens[-1][0] == mc.arg(0) else 0)

@impl(SCR, "vmIsBottomScreen")
def is_bottom(mc, rt):
    mc.ret(1 if rt.screens and rt.screens[0][0] == mc.arg(0) else 0)

COOLBAR_DIR = ".system/MB_MSTAR_WQVGA"

@impl(SYS, "GetCoolbarDirPath")
def coolbar_dir(mc, rt):

    wwrite(mc, mc.arg(0), COOLBAR_DIR)
    mc.ret(len(COOLBAR_DIR) * 2)

@impl(SYS, "GetCoolBarFullPath")
def coolbar_full(mc, rt):

    out, name = mc.arg(0), mc.arg(1)
    wwrite(mc, out, COOLBAR_DIR + wstr(mc, name))
    mc.ret(out)

@impl(SYS, "VmIsInnerApp")
def is_inner(mc, rt):
    mc.ret(0)

@impl(SYS, "VmGetPlatformType")
def platform(mc, rt):
    mc.ret(0)

@impl("VmUcs2StrManagerTag", "VmutExpandStrcpy")
def expand_strcpy(mc, rt):

    src = mc.cstr(mc.arg(1)) or b""
    wwrite(mc, mc.arg(0), src.decode("latin1"))
    mc.ret(mc.arg(0))

@impl(SYS, "VmGetScreenWidth", "GetScreenWidth")
def scr_w(mc, rt):
    mc.ret(rt.screen_w)

@impl(SYS, "VmGetScreenHeight", "GetScreenHeight")
def scr_h(mc, rt):
    mc.ret(rt.screen_h)

@impl(LCD, "VMFillRect")
def fill_rect(mc, rt):

    v = mc.arg(0)
    l, t = v & 0xFFFF, (v >> 16) & 0xFFFF
    v2 = mc.arg(1)
    r, b = v2 & 0xFFFF, (v2 >> 16) & 0xFFFF
    rt.fill_rect(l, t, r - l + 1, b - t + 1, mc.arg(2))
    mc.ret(1)

@impl(LCD, "VMDrawRectEx")
def draw_rect_ex(mc, rt):
    x, y, w, h, c = (mc.arg(i) for i in range(5))
    rt.fill_rect(x, y, w, 1, c)
    rt.fill_rect(x, y + h - 1, w, 1, c)
    rt.fill_rect(x, y, 1, h, c)
    rt.fill_rect(x + w - 1, y, 1, h, c)
    mc.ret(1)

@impl(LCD, "VMDrawLineEx")
def draw_line_ex(mc, rt):
    x1, y1, x2, y2, c = (mc.arg(i) for i in range(5))
    if y1 == y2:
        rt.fill_rect(min(x1, x2), y1, abs(x2 - x1) + 1, 1, c)
    elif x1 == x2:
        rt.fill_rect(x1, min(y1, y2), 1, abs(y2 - y1) + 1, c)
    else:
        dx, dy = x2 - x1, y2 - y1
        n = max(abs(dx), abs(dy)) or 1
        for i in range(n + 1):
            rt.fill_rect(x1 + dx * i // n, y1 + dy * i // n, 1, 1, c)
    mc.ret(1)

def _text_width(rt, s):
    return rt.font.measure(s) if rt.font else _gbk_cells(s) * 8

def _gbk_cells(b):

    cells, i = 0, 0
    while i < len(b):
        if b[i] >= 0x80 and i + 1 < len(b):
            cells += 2; i += 2
        else:
            cells += 1; i += 1
    return cells

@impl(LCD, "VMGetStringWidth", "vMGetStringWidthEx")
def str_w(mc, rt):
    mc.ret(_text_width(rt, mc.cstr(mc.arg(0)) or b""))

@impl(LCD, "VMGetStringHeight", "vMGetStringHeightEx")
def str_h(mc, rt):
    mc.ret(rt.font.hh if rt.font else 12)

def _target(rt, img):

    if img:
        info = rt.images.info(img)
        if info and info[0]:
            data, w, h, st = info
            return data, st, w, h
    return rt.fb.buf, rt.fb.w, rt.fb.w, rt.fb.h

def _draw_text(rt, mc, s, x, y, color, size=12, img=0):

    rt.note_text(s, x, y, color, size)
    f = rt.font
    if f is None:
        for i in range(_gbk_cells(s)):
            rt.fill_rect(x + i * 8 + 1, y + 1, 6, 9, color)
        return
    buf, stride, w, h = _target(rt, img)
    f.draw(rt.mach, buf, stride, w, h, s, x, y, color)

@impl(LCD, "VMDrawString")
def draw_string(mc, rt):
    _draw_text(rt, mc, mc.cstr(mc.arg(0)) or b"", mc.arg(1), mc.arg(2), mc.arg(3))
    mc.ret(0)

@impl(LCD, "VMDrawStringClipAlign", "VMDrawStringClip", "vMDrawStringClipBorder",
      "vMDrawStringClipAlignBorder", "vMShowStringClipAlign", "vMShowStringClip")
def draw_string_clip(mc, rt):
    _draw_text(rt, mc, mc.cstr(mc.arg(0)) or b"", mc.arg(1), mc.arg(2), mc.arg(3))
    mc.ret(1)

@impl(LCD, "vMShowString", "vMDrawStringBorder")
def show_string(mc, rt):
    _draw_text(rt, mc, mc.cstr(mc.arg(0)) or b"", mc.arg(1), mc.arg(2), mc.arg(3))
    mc.ret(0)

@impl(SYS, "GetCoolBarKernelCurrentVersion")
def kernel_ver(mc, rt):
    mc.ret(42)

@impl(LCD, "VMGetCurrFontType", "VMIsBacklightOn", "VmGetIsNeedRefreshLcd")
def lcd_misc(mc, rt):
    mc.ret(1)

NET = "VmNetManagerTag"
IM = "VmImManagerTag"

DEVICE = {
    "imei": "356938035643809",
    "prj":  "7835",
    "smsc": "+8613800100500",
    "appid": 1002,
}

@impl(SYS, "VMGetIMEI")
def get_imei(mc, rt):

    buf, n = mc.arg(0), mc.arg(1) or 26
    s = DEVICE["imei"].encode()[:max(0, n - 1)]
    mc.uc.mem_write(buf, s + b"\x00")
    mc.ret(len(s))

@impl(SYS, "VMGetPrjName")
def get_prj(mc, rt):
    mc.uc.mem_write(mc.arg(0), DEVICE["prj"].encode()[:4] + b"\x00")
    mc.ret(4)

@impl(SYS, "VmGetSmsCenterNum")
def get_smsc(mc, rt):
    mc.uc.mem_write(mc.arg(0), DEVICE["smsc"].encode() + b"\x00")
    mc.ret(len(DEVICE["smsc"]))

@impl(SYS, "vmDlGetCurrAppId")
def cur_appid(mc, rt):
    mc.ret(DEVICE["appid"])

@impl(STD, "vMstricmp")
def stricmp(mc, rt):
    a = (mc.cstr(mc.arg(0)) or b"").lower()
    b = (mc.cstr(mc.arg(1)) or b"").lower()
    mc.ret(0 if a == b else 1)

@impl(STD, "vMstrnicmp")
def strnicmp(mc, rt):
    n = mc.arg(2)
    a = (mc.cstr(mc.arg(0)) or b"")[:n].lower()
    b = (mc.cstr(mc.arg(1)) or b"")[:n].lower()
    mc.ret(0 if a == b else 1)

@impl(IM, "reserver_func03", "reserver_func04")
def im_reserved(mc, rt):
    mc.ret(0)

HTTP_ALL_RECEIVED, HTTP_PART_RECEIVED, HTTP_RETRY, HTTP_CANCEL = 0, 1, 2, 3
HTTP_CONNECTED, NET_OPENCHANNEL_OK, NET_OPENCHANNEL_ERROR = 4, 5, 6
NET_CHANNEL_RECEIVED, NET_CHANNEL_CLOSED, NETREQUEST_ERROR = 7, 8, 9
HTTP_NO_CONTENT, NET_CHANNEL_REDIRECT = 10, 11

def _http(mc, rt, url_ptr, cb, out, body=None):
    url = (mc.cstr(url_ptr) or b"").decode("latin1", "replace")
    h = rt.state.get("nethandle", 0) + 1
    rt.state["nethandle"] = h
    if out:
        mc.w32(out, h)
    resp = rt.net_response(url)
    rt.trace_io(f"HTTP {url}  ->  {'%d 字节' % len(resp) if resp is not None else '离线(NETREQUEST_ERROR)'}")
    if resp is None:
        rt.defer(cb, (0, 0, 0, NETREQUEST_ERROR), "netCallBack(离线)")
    else:
        p = mc.heap.alloc(max(len(resp), 1), "httpresp")
        if resp:
            mc.uc.mem_write(p, resp)
        rt.defer(cb, (p, len(resp), len(resp), HTTP_ALL_RECEIVED), "netCallBack")
    mc.ret(1)

@impl(NET, "GetHttpData")
def get_http(mc, rt):
    _http(mc, rt, mc.arg(0), mc.arg(1), mc.arg(2))

@impl(NET, "GetHttpDataEx")
def get_http_ex(mc, rt):
    _http(mc, rt, mc.arg(0), mc.arg(1), mc.arg(2))

@impl(NET, "PostHttpData")
def post_http(mc, rt):
    _http(mc, rt, mc.arg(0), mc.arg(3), mc.arg(4))

@impl(NET, "OpenChannel", "OpenChannel2", "OpenQQChannel")
def open_channel(mc, rt):
    rt.defer(mc.arg(2), (0, 0, 0, NET_OPENCHANNEL_ERROR), "netOpenChannel(离线)")
    mc.ret(0)

@impl(NET, "CloseChannel", "CancelHttpConnect")
def close_channel(mc, rt):
    mc.ret(1)

@impl(NET, "VMGetLinkSetNum", "VMGetWapIndex", "VMGetNetIndex")
def link_num(mc, rt):
    mc.ret(0)

UCS = "VmUcs2StrManagerTag"

@impl(UCS, "vmutStrlenUcs2", "VmutStrlenUcs2")
def ucs2_strlen(mc, rt):
    mc.ret(len(wstr(mc, mc.arg(0), 4096)))

@impl(UCS, "vmutStrcpyUcs2", "VmutStrcpyUcs2")
def ucs2_strcpy(mc, rt):
    wwrite(mc, mc.arg(0), wstr(mc, mc.arg(1), 4096))
    mc.ret(mc.arg(0))

@impl(UCS, "vmutStrcatUcs2", "VmutStrcatUcs2")
def ucs2_strcat(mc, rt):
    d = mc.arg(0)
    cur = wstr(mc, d, 4096)
    wwrite(mc, d + len(cur) * 2, wstr(mc, mc.arg(1), 4096))
    mc.ret(d)

@impl(UCS, "vmutStrcmpUcs2", "VmutStrcmpUcs2")
def ucs2_strcmp(mc, rt):
    a, b = wstr(mc, mc.arg(0), 4096), wstr(mc, mc.arg(1), 4096)
    mc.ret(0 if a == b else 1)

@impl(LCD, "VMGB2UCS2")
def gb2ucs2(mc, rt):

    s = (mc.cstr(mc.arg(0)) or b"").decode("gb18030", "replace")
    n = mc.arg(2) or len(s) + 1
    s = s[:max(0, n - 1)]
    wwrite(mc, mc.arg(1), s)
    mc.ret(len(s))

@impl(LCD, "VMUCS2GB")
def ucs2gb(mc, rt):
    s = wstr(mc, mc.arg(0), 4096).encode("gb18030", "replace")
    n = mc.arg(2) or len(s) + 1
    s = s[:max(0, n - 1)]
    mc.uc.mem_write(mc.arg(1), s + b"\x00")
    mc.ret(len(s))

@impl(LCD, "VMDrawStringRect", "vMShowStringRect", "vMDrawUcs2StringRect",
      "vMDrawUcs2StringRectEx")
def draw_string_rect(mc, rt):

    s = mc.cstr(mc.arg(0)) or b""
    x, y, w, h, color = (mc.arg(i) for i in range(1, 6))

    lh = (rt.font.hh + 2) if rt.font else 14
    lines, cur, cw, y0 = 0, bytearray(), 0, y
    def flush():
        nonlocal cur, cw, lines
        if cur:
            _draw_text(rt, mc, bytes(cur), x, y0 + lines * lh, color)
            lines += 1
            cur, cw = bytearray(), 0
    i = 0
    while i < len(s):
        two = s[i] >= 0x80 and i + 1 < len(s)
        piece = s[i:i + 2] if two else s[i:i + 1]
        pw = _text_width(rt, piece)
        if w and cw + pw > w and cur:
            if h and (lines + 1) * lh > h:
                break
            flush()
        cur += piece; cw += pw; i += len(piece)
    if not (h and (lines + 1) * lh > h):
        flush()
    mc.ret((lines << 16) | min(_text_width(rt, s), w or 0xFFFF))

@impl(SYS, "vMGetGameWinState")
def game_win_state(mc, rt):
    mc.ret(0)

@impl(SYS, "VmEnterWinClose", "VmEnterWinOpen", "vMAssert")
def enter_win(mc, rt):
    mc.ret(0)

@impl(GAME, "initMemoryBlock")
def init_memory_block(mc, rt):

    blk, size = mc.arg(0), mc.arg(1)
    if not blk:
        blk = mc.heap.alloc(0x18, "MEMORY_BLOCK")
    base = mc.heap.alloc(size, "memblock", strict=False) if size else 0
    if base:
        mc.uc.mem_write(base, b"\x00" * size)
    mc.w32(blk + 0x00, base)
    mc.w32(blk + 0x04, 0)
    mc.w32(blk + 0x08, size)
    rt.install(blk + 0x0c, "MB_Malloc", mb_malloc)
    rt.install(blk + 0x10, "MB_Reset", mb_reset)
    rt.install(blk + 0x14, "MB_Release", mb_release)
    rt.trace_io(f"initMemoryBlock(blk={blk:#x}, size={size:#x}) -> data {base:#x}")
    mc.ret(blk)

def mb_malloc(mc, rt):

    blk, size = mc.arg(0), mc.arg(1)
    base, ptr, total = mc.r32(blk), mc.r32(blk + 4), mc.r32(blk + 8)
    size = (size + 3) & ~3
    if ptr + size > total:
        mc.ret(0); return
    mc.w32(blk + 4, ptr + size)
    mc.uc.mem_write(base + ptr, b"\x00" * size)
    mc.ret(base + ptr)

def mb_reset(mc, rt):
    mc.w32(mc.arg(0) + 4, 0); mc.ret(0)

def mb_release(mc, rt):
    blk = mc.arg(0)
    mc.heap.free(mc.r32(blk))
    mc.w32(blk, 0); mc.w32(blk + 4, 0); mc.w32(blk + 8, 0)
    mc.ret(0)

@impl(GAME, "initDreamFactoryEngine")
def init_df_engine(mc, rt):
    rt.trace_io("initDreamFactoryEngine()")
    mc.ret(0)

DP = {
    "isLoaded": 0x00, "packageIndex": 0x01, "packageName": 0x04,
    "fileNum": 0x08, "subPackageNum": 0x0a, "fileNameTable": 0x0c,
    "fileOffsetTable": 0x10, "fileIndex": 0x14, "fileData": 0x18,
    "subDataPackage": 0x1c, "isMomentRead": 0x54, "fileOffset": 0x58,
    "file": 0x5c, "dataSize": 0x60, "txtFileData": 0x64, "txtStartOffSet": 0x68,
}
DP_METHODS = [
    (0x20, "DP_LoadPackage", "dp_load"), (0x24, "DP_ReleasePackage", "dp_release"),
    (0x28, "DP_LoadFromTResource", "dp_load"), (0x2c, "DP_LoadFormTCard", "dp_load"),
    (0x30, "DP_DoLoading", "dp_noop"), (0x34, "DP_LocateDataPackage", "dp_locate"),
    (0x38, "DP_GetFile", "dp_get_file"), (0x3c, "DP_GetFileByID", "dp_get_by_id"),
    (0x40, "DP_GetFileNameByID", "dp_name_by_id"), (0x44, "DP_GetFileID", "dp_file_id"),
    (0x48, "DP_ShowFileList", "dp_noop"), (0x4c, "DP_LoadFormTCardEx", "dp_load"),
    (0x50, "DF_DataPackage_InitTxt", "dp_noop"),
]

@impl(GAME, "initDFDataPackage")
def init_df_datapackage(mc, rt):

    pkg, nsub = mc.arg(0), mc.arg(1) & 0xFFFF
    for off in (0x04, 0x0c, 0x10, 0x18, 0x1c, 0x64):
        mc.w32(pkg + off, 0)
    mc.uc.mem_write(pkg + 0x08, b"\x00\x00\x00\x00")
    mc.uc.mem_write(pkg + 0x00, b"\x01")
    mc.uc.mem_write(pkg + DP["isMomentRead"], b"\x00")
    mc.w32(pkg + DP["file"], 0xFFFFFFFF)
    subs = mc.heap.alloc(max(nsub, 1) * 4, "subDataPackage")
    mc.uc.mem_write(subs, b"\x00" * (max(nsub, 1) * 4))
    mc.w32(pkg + DP["subDataPackage"], subs)
    mc.uc.mem_write(pkg + DP["subPackageNum"], nsub.to_bytes(2, "little"))
    for off, name, fnname in DP_METHODS:
        rt.install(pkg + off, name, globals()[fnname])
    rt.trace_io(f"initDFDataPackage(pkg={pkg:#x}, subPackageNum={nsub})")
    mc.ret(pkg)

def _materialize(mc, rt, pkg, arch):

    key = ("dp", id(arch))
    if key in rt.state:
        names_tbl, offs_tbl, data_p, n, total = rt.state[key]
    else:
        n = arch.count
        blob = b"".join(e.data for e in arch.entries)
        data_p = mc.heap.alloc(len(blob) or 1, "dp_fileData")
        if blob:
            mc.uc.mem_write(data_p, blob)
        names_tbl = mc.heap.alloc(n * 4, "dp_names")
        offs_tbl = mc.heap.alloc((n + 1) * 4, "dp_offsets")
        cur = 0
        for i, e in enumerate(arch.entries):
            nm = mc.heap.alloc(len(e.name) + 1, "dp_name")
            mc.uc.mem_write(nm, e.name.encode("latin1") + b"\x00")
            mc.w32(names_tbl + i * 4, nm)
            mc.w32(offs_tbl + i * 4, cur)
            cur += e.size
        mc.w32(offs_tbl + n * 4, cur)
        total = cur
        rt.state[key] = (names_tbl, offs_tbl, data_p, n, total)
    mc.uc.mem_write(pkg + DP["fileNum"], (n & 0xFFFF).to_bytes(2, "little"))
    mc.w32(pkg + DP["fileNameTable"], names_tbl)
    mc.w32(pkg + DP["fileOffsetTable"], offs_tbl)
    mc.w32(pkg + DP["fileData"], data_p)
    mc.w32(pkg + DP["dataSize"], total)
    mc.uc.mem_write(pkg + DP["isLoaded"], b"\x01")
    return n

class _Combined:

    def __init__(self, root, sub):
        pos, ents = {}, []
        for e in list(root.entries) + list(sub.entries):
            if e.name in pos:
                ents[pos[e.name]] = e
            else:
                pos[e.name] = len(ents)
                ents.append(e)
        self.entries = ents
        self.count = len(ents)

    def names(self):
        return [e.name for e in self.entries]

def _pick_archive(rt, name):

    pk = rt.mod.packages or {}
    root = pk.get("")
    arch = None
    if name and pk:
        arch = pk.get(name)
        if arch is None:
            base = name.rsplit("/", 1)[-1].rsplit("\\", 1)[-1]
            for k, v in pk.items():
                if k and (k == base or k.lower() == base.lower()):
                    arch = v
                    break
    if arch is None:
        return root if root is not None else (rt.mod.res or rt.mod.icons)
    if root is None or root is arch:
        return arch
    key = ("comb", name)
    c = rt.state.get(key)
    if c is None:
        c = rt.state[key] = _Combined(root, arch)
    return c

def dp_load(mc, rt):

    pkg = mc.arg(0)
    name = (mc.cstr(mc.arg(1)) or b"").decode("latin1")
    arch = _pick_archive(rt, name)
    n = _materialize(mc, rt, pkg, arch) if arch else 0
    rt.state.setdefault("pkg_arch", {})[pkg] = arch
    mc.w32(pkg + DP["packageName"], mc.arg(1))
    rt.trace_io(f"DP_LoadPackage(pkg={pkg:#x}, {name!r}) -> {n} 个条目"
                + ("" if not rt.mod.packages else f"（可选子包: {list(rt.mod.packages)[:6]}）"))
    mc.ret(0)

def dp_release(mc, rt):
    mc.uc.mem_write(mc.arg(0) + DP["isLoaded"], b"\x00"); mc.ret(0)

def dp_noop(mc, rt):
    mc.ret(0)

def dp_locate(mc, rt):
    mc.ret(mc.arg(0))

def _entries(rt, pkg=None):

    m = rt.state.get("pkg_arch") or {}
    a = m.get(pkg) if pkg is not None else None
    if a is None and m:
        a = next(iter(m.values()))
    if a is None:
        a = rt.mod.res or rt.mod.icons
    return a.entries if a else []

def dp_file_id(mc, rt):

    name = (mc.cstr(mc.arg(1)) or b"").decode("latin1")
    for i, e in enumerate(_entries(rt, mc.arg(0))):
        if e.name == name:
            mc.ret(i); return
    mc.ret(0xFFFF)

def dp_name_by_id(mc, rt):
    i = mc.arg(1) & 0xFFFF
    tbl = mc.r32(mc.arg(0) + DP["fileNameTable"])
    ents = _entries(rt, mc.arg(0))
    mc.ret(mc.r32(tbl + i * 4) if tbl and i < len(ents) else 0)

def dp_get_by_id(mc, rt):

    pkg, i = mc.arg(0), mc.arg(1) & 0xFFFF
    data = mc.r32(pkg + DP["fileData"])
    offs = mc.r32(pkg + DP["fileOffsetTable"])
    if not data or not offs or i >= len(_entries(rt, pkg)):
        mc.ret(0); return
    mc.ret(data + mc.r32(offs + i * 4))

def dp_get_file(mc, rt):

    name = (mc.cstr(mc.arg(1)) or b"").decode("latin1")
    pkg = mc.arg(0)
    for i, e in enumerate(_entries(rt, pkg)):
        if e.name == name:
            data = mc.r32(pkg + DP["fileData"])
            offs = mc.r32(pkg + DP["fileOffsetTable"])
            mc.ret(data + mc.r32(offs + i * 4) if data and offs else 0)
            return
    rt.trace_io(f"DP_GetFile: 未找到 {name!r}")
    mc.ret(0)

@impl(GAME, "SCREEN_ChangeScreen")
def screen_change(mc, rt):
    rt.screens.clear()
    rt.push_screen(mc.arg(0), 0, 1)
    mc.ret(0)

@impl(GAME, "SCREEN_NotifyLoadResource")
def screen_notify_loadres(mc, rt):
    scr = mc.arg(0)
    rt.defer(mc.r32(scr + 4 * rt.S_LOADRES), (0,), "screenLoadResource")
    mc.ret(0)

@impl(GAME, "DF_SetDataPackage")
def df_set_pkg(mc, rt):
    rt.state["datapackage"] = mc.arg(0)
    mc.ret(0)

@impl(GAME, "DF_GetDataPackage")
def df_get_pkg(mc, rt):
    mc.ret(rt.state.get("datapackage", 0))

@impl(SYS, "VMGetOperator")
def get_operator(mc, rt):
    mc.ret(0)

@impl(SYS, "vMGetKeyNum")
def get_keynum(mc, rt):
    mc.ret(0)

from cbelib.imgcodec import decode as _decode_img

@impl(LCD, "IMG_CreateImageFormStream")
def img_from_stream(mc, rt):

    stream, out = mc.arg(0), mc.arg(1)
    if stream in rt.images.by_stream and not out:
        mc.ret(rt.images.by_stream[stream]); return
    raw = bytes(mc.uc.mem_read(stream, min(0x80000, 0x80000)))
    try:
        img = _decode_img(raw)
    except Exception as e:
        rt.trace_io(f"IMG_CreateImageFormStream 解码失败 @{stream:#x}: {e}")
        mc.ret(0); return
    if not img:
        mc.ret(0); return
    vt = rt.images.upload(img, out or None)
    rt.images.by_stream[stream] = vt
    mc.ret(vt)

@impl(GAME, "IMG_CreateImageFormRes")
def img_from_res(mc, rt):
    mc.setreg(1, mc.arg(1))
    img_from_stream(mc, rt)

def _blit8(mc, rt, alpha):

    dst, src = mc.arg(0), mc.arg(1)
    if not src or not mc.r32(src):
        mc.ret(1); return
    sx, sy = _s16(mc.arg(2)), _s16(mc.arg(3))
    w, h = _s16(mc.arg(4)), _s16(mc.arg(5))
    dx, dy = _s16(mc.arg(6)), _s16(mc.arg(7))
    rt.images.blit(src, dst or rt.fb.img, dx, dy, w, h, sx, sy, alpha=alpha)
    mc.ret(1)

@impl(LCD, "VMDrawImageWithClipEx")
def draw_img_clip_ex(mc, rt):
    _blit8(mc, rt, False)

@impl(LCD, "VMDrawImageClipAndAlphaEx")
def draw_img_clip_alpha_ex(mc, rt):
    _blit8(mc, rt, True)

@impl(LCD, "VMDrawImageEx")
def draw_img_ex(mc, rt):

    rt.images.blit(mc.arg(0), rt.fb.img, _s16(mc.arg(1)), _s16(mc.arg(2)))
    mc.ret(1)

def _draw_img(mc, rt, alpha):

    pt = mc.arg(1)
    rt.images.blit(mc.arg(0), rt.fb.img, _s16(pt & 0xFFFF), _s16(pt >> 16), alpha=alpha)
    mc.ret(1)

@impl(LCD, "VMDrawImage")
def draw_img(mc, rt):
    _draw_img(mc, rt, False)

@impl(LCD, "VMDrawImageWithAlpha")
def draw_img_alpha(mc, rt):
    _draw_img(mc, rt, True)

@impl("VmGameLcdManagerTag", "ReleaseImage")
@impl(GAME, "IMG_Destory")
def img_release(mc, rt):
    vt = mc.arg(0)
    if vt:
        data = mc.r32(vt)
        rt.images.masks.pop(data, None)
        rt.images.by_stream = {k: v for k, v in rt.images.by_stream.items() if v != vt}
        if data:
            mc.heap.free(data)
        mc.w32(vt, 0)
    mc.ret(0)

def _s16(v):
    v &= 0xFFFF
    return v - 0x10000 if v & 0x8000 else v

UTIL = "VmGameUtilManagerTag"

def _res_index(rt, name):
    for i, e in enumerate(_entries(rt)):
        if e.name == name:
            return i
    return -1

def _res_ptr(mc, rt, i):

    pkg = rt.state.get("datapackage")
    if not pkg:
        return 0
    data = mc.r32(pkg + DP["fileData"])
    offs = mc.r32(pkg + DP["fileOffsetTable"])
    if not data or not offs or i < 0 or i >= len(_entries(rt)):
        return 0
    return data + mc.r32(offs + i * 4)

@impl(UTIL, "DF_GetResourceIDByFileName")
@impl(GAME, "DF_GetResourceIDByFileName")
def df_res_id(mc, rt):
    mc.ret(_res_index(rt, (mc.cstr(mc.arg(0)) or b"").decode("latin1")) & 0xFFFFFFFF)

@impl(UTIL, "DF_GetResourceNameByID")
@impl(GAME, "DF_GetResourceNameByID")
def df_res_name(mc, rt):
    pkg = rt.state.get("datapackage", 0)
    tbl = mc.r32(pkg + DP["fileNameTable"]) if pkg else 0
    i = mc.arg(0)
    mc.ret(mc.r32(tbl + i * 4) if tbl and i < len(_entries(rt)) else 0)

@impl(UTIL, "DF_GetResourceByResourceID")
@impl(GAME, "DF_GetResourceByResourceID")
def df_res_by_id(mc, rt):
    mc.ret(_res_ptr(mc, rt, mc.arg(0)))

@impl(UTIL, "DF_GetResourceByFileName", "DF_GetTResource", "DF_GetStreamTResource")
@impl(GAME, "DF_GetResourceByFileName", "DF_GetTResource", "DF_GetStreamTResource")
def df_res_by_name(mc, rt):
    name = (mc.cstr(mc.arg(0)) or b"").decode("latin1")
    i = _res_index(rt, name)
    p = _res_ptr(mc, rt, i)
    if not p:
        rt.trace_io(f"DF_GetResource: 未找到 {name!r}")
    mc.ret(p)

@impl(UTIL, "DF_String_Equal")
@impl(GAME, "DF_String_Equal")
def df_str_eq(mc, rt):
    mc.ret(1 if (mc.cstr(mc.arg(0)) or b"") == (mc.cstr(mc.arg(1)) or b"") else 0)

@impl(UTIL, "DF_GetMemoryBlock")
@impl(GAME, "DF_GetMemoryBlock")
def df_get_memblock(mc, rt):
    if "dfblock" not in rt.state:
        blk = mc.heap.alloc(0x18, "MEMORY_BLOCK(DF)")
        mc.setreg(0, blk); mc.setreg(1, 0x40000)
        init_memory_block(mc, rt)
        rt.state["dfblock"] = blk
    mc.ret(rt.state["dfblock"])

@impl(UTIL, "DF_WriteShort")
@impl(GAME, "DF_WriteShort")
def df_write_short(mc, rt):

    buf, ppos, v = mc.arg(0), mc.arg(1), mc.arg(2) & 0xFFFF
    pos = mc.r32(ppos) if ppos else 0
    mc.uc.mem_write(buf + pos, bytes([v & 0xFF, (v >> 8) & 0xFF]))
    if ppos:
        mc.w32(ppos, pos + 2)
    mc.ret(0)

@impl(UTIL, "DF_WriteInt")
@impl(GAME, "DF_WriteInt")
def df_write_int(mc, rt):
    buf, ppos, v = mc.arg(0), mc.arg(1), mc.arg(2) & 0xFFFFFFFF
    pos = mc.r32(ppos) if ppos else 0
    mc.uc.mem_write(buf + pos, v.to_bytes(4, "little"))
    if ppos:
        mc.w32(ppos, pos + 4)
    mc.ret(0)

def _savelog(rt, msg):

    try:
        import time
        with open(os.path.join(paths.home(), "saves.log"), "a", encoding="utf-8") as f:
            f.write(f"{time.strftime('%H:%M:%S')} [{rt.mod.name}] {msg}\n")
    except OSError:
        pass

@impl(UTIL, "Storage_Date")
@impl(GAME, "Storage_Date")
def storage_date(mc, rt):

    name = (mc.cstr(mc.arg(0)) or b"save").decode("latin1")
    buf, n, write = mc.arg(1), mc.arg(2), 0 if mc.arg(3) else 1

    _savelog(rt, f"Storage_Date({name!r}, buf={buf:#x}, len={n}, write={write}, "
                 f"arg4={mc.arg(4)}) -> {'保存' if write else '读取'}")
    if write:

        try:
            obj = bytes(mc.uc.mem_read(mc.arg(0), 0x40))
            _savelog(rt, f"  对象 {mc.arg(0):#x}: {obj.hex()}")
            if buf and n:
                _savelog(rt, f"  缓冲前 32B: {bytes(mc.uc.mem_read(buf, min(n,32))).hex()}")
        except Exception as e:
            _savelog(rt, f"  (读对象失败: {e})")

    path = os.path.join(paths.saves_dir(rt.mod.name), paths.safe(name))
    try:
        if write and n == 0:

            _savelog(rt, f"  -> 忽略：长度为 0 的写入不落盘（{path}）")
            mc.ret(0); return
        if write:

            mode = "r+b" if os.path.exists(path) else "wb"
            with open(path, mode) as f:
                f.write(bytes(mc.uc.mem_read(buf, n)))
            rt.trace_io(f"存档写入 {path} ({n} 字节, {mode})")
            _savelog(rt, f"  -> 写入 {path} {n} 字节 mode={mode} "
                          f"落盘后大小={os.path.getsize(path)}")
            mc.ret(1); return
        with open(path, "rb") as f:
            d = f.read(n)

        if buf and n:
            mc.uc.mem_write(buf, d + b"\x00" * (n - len(d)))
        rt.trace_io(f"存档读取 {path} ({len(d)}/{n} 字节)")
        _savelog(rt, f"  -> 读取 {path} 得到 {len(d)}/{n} 字节")

        mc.ret(1 if d else 0)
    except OSError:
        if buf and n:
            mc.uc.mem_write(buf, b"\x00" * n)
        mc.ret(0)

@impl(SYS, "VmEnterWinClose")
def enter_win_close(mc, rt):

    rt.exit_requested = True
    rt.trace_io("模块请求退出（VmEnterWinClose）")
    mc.ret(0)

@impl(GAME, "SCREEN_IsKeyDown", "GAME_isKeyDown")
def key_down(mc, rt):
    mc.ret(rt.input_query(mc, "down"))

@impl(GAME, "SCREEN_IsKeyUp")
def key_up(mc, rt):
    mc.ret(rt.input_query(mc, "up"))

@impl(GAME, "SCREEN_IsKeyHold", "GAME_isKeyHold")
def key_hold(mc, rt):
    mc.ret(rt.input_query(mc, "hold"))

@impl(GAME, "SCREEN_IsPointerDown")
def ptr_down(mc, rt):
    mc.ret(rt.touch_down)

@impl(GAME, "SCREEN_IsPointerUp")
def ptr_up(mc, rt):
    mc.ret(rt.touch_up)

@impl(GAME, "SCREEN_IsPointerHold")
def ptr_hold(mc, rt):
    mc.ret(rt.touch_hold)

@impl(GAME, "SCREEN_IsPointerDrag")
def ptr_drag(mc, rt):
    mc.ret(rt.touch_drag)

@impl(GAME, "SCREEN_GetPointerX")
def ptr_x(mc, rt):
    mc.ret(rt.pointer[0])

@impl(GAME, "SCREEN_GetPointerY")
def ptr_y(mc, rt):
    mc.ret(rt.pointer[1])

@impl(GAME, "Get_CurKeyDownState")
def cur_key(mc, rt):
    mc.ret(rt.keys_down)

from cbelib.lz import unpack_entry

@impl(GAME, "GetStreamDataFormRes")
@impl("VmGameLcdManagerTag", "GetStreamDataFormRes")
def get_stream_data(mc, rt):

    p = mc.arg(0)
    if not p:
        mc.ret(0); return

    hdr = bytes(mc.uc.mem_read(p, 9))
    if hdr[0] != 2:
        mc.ret(p + 9); return
    comp = int.from_bytes(hdr[1:5], "big")
    unc = int.from_bytes(hdr[5:9], "big")
    raw = bytes(mc.uc.mem_read(p, 9 + comp))
    data = unpack_entry(raw)
    buf = mc.heap.alloc(max(len(data), 1), "stream")
    if data:
        mc.uc.mem_write(buf, data)
    rt.trace_io(f"GetStreamDataFormRes({p:#x}) 解压 {comp} -> {len(data)}/{unc} 字节")
    mc.ret(buf)

@impl(UTIL, "ReadShort", "DF_ReadShort")
@impl(GAME, "DF_ReadShort")
def df_read_short(mc, rt):

    buf, ppos = mc.arg(0), mc.arg(1)
    pos = mc.r32(ppos) if ppos else 0
    b = bytes(mc.uc.mem_read(buf + pos, 2))
    if ppos:
        mc.w32(ppos, pos + 2)
    v = b[0] | (b[1] << 8)
    mc.ret(v - 0x10000 if v & 0x8000 else v)

@impl(UTIL, "DF_ReadInt")
@impl(GAME, "DF_ReadInt")
def df_read_int(mc, rt):

    buf, ppos = mc.arg(0), mc.arg(1)
    pos = mc.r32(ppos) if ppos else 0
    b = bytes(mc.uc.mem_read(buf + pos, 4))
    if ppos:
        mc.w32(ppos, pos + 4)
    mc.ret(int.from_bytes(b, "little"))

def _df_read_string(mc, lenbytes):

    buf, ppos = mc.arg(0), mc.arg(1)
    pos = mc.r32(ppos) if ppos else 0
    n = int.from_bytes(bytes(mc.uc.mem_read(buf + pos, lenbytes)), "little")
    if n > 0x10000:
        n = 0
    s = bytes(mc.uc.mem_read(buf + pos + lenbytes, n)) if n else b""
    if ppos:
        mc.w32(ppos, pos + lenbytes + n)
    p = mc.heap.alloc(n + 1, "df_str")
    mc.uc.mem_write(p, s + b"\x00")
    mc.ret(p)

@impl(UTIL, "DF_ReadString")
@impl(GAME, "DF_ReadString")
def df_read_string(mc, rt):

    _df_read_string(mc, 1)

@impl(UTIL, "DF_ReadString2")
@impl(GAME, "DF_ReadString2")
def df_read_string2(mc, rt):

    _df_read_string(mc, 4)

@impl(LCD, "VMDrawStringEx", "vMShowStringEx")
def draw_string_ex(mc, rt):

    _draw_text(rt, mc, mc.cstr(mc.arg(1)) or b"", _s16(mc.arg(2)), _s16(mc.arg(3)),
               mc.arg(4), img=mc.arg(0))
    mc.ret(0)

@impl(SYS, "cDownGetCompanyEx", "CDownGetServicePhone", "CDownGetCompany")
def cdown_info(mc, rt):

    if mc.arg(0):
        mc.uc.mem_write(mc.arg(0), b"\x00")
    mc.ret(0)

@impl(SYS, "VmSetFPS")
def set_fps(mc, rt):
    fps = mc.arg(0) or 25
    rt.frame_ms = max(1, 1000 // min(fps, 100))
    mc.ret(0)

@impl(LCD, "vMSetFontSize", "vMResetFontSize", "VMSetCurrFontType",
      "VmAllowBackLight", "VMCtrlBacklight", "VmSetIsNeedRefreshLcd",
      "VmLCDInvalidateRectEnable", "VmSetVideoIsNeedClosed")
def lcd_setters(mc, rt):
    mc.ret(0)

AUD = "VmAudioManagerTag"

def _res_audio(rt, rid):

    ents = _entries(rt)
    if not (0 <= rid < len(ents)):
        return b"", f"res{rid}"
    e = ents[rid]
    d = e.data
    if not d:
        return b"", e.name
    t = d[0]
    if t == 10:
        n = int.from_bytes(d[1:5], "big")
        return d[5:5 + n], e.name
    if t == 2:
        from cbelib.lz import unpack_entry
        return (unpack_entry(d) or b""), e.name
    return d[5:], e.name

@impl(AUD, "vMAudioPlayForGame", "vMAudioPlayForApp")
def audio_play_id(mc, rt):

    data, name = _res_audio(rt, mc.arg(0))
    mc.ret(rt.audio.play_data(data, mc.arg(1), name))

@impl(AUD, "vMAudioPlayWithDataPackage")
def audio_play_pkg(mc, rt):
    data, name = _res_audio(rt, mc.arg(0))
    mc.ret(rt.audio.play_data(data, mc.arg(1), name))

@impl(AUD, "vMAudioPlayByData")
def audio_play_data(mc, rt):

    p = mc.arg(0)
    if not p:
        mc.ret(0); return
    hdr = bytes(mc.uc.mem_read(p, 5))
    n = int.from_bytes(hdr[1:5], "big")
    if not (0 < n <= 4 << 20):
        n = 4096
    data = bytes(mc.uc.mem_read(p + 5, n))
    mc.ret(rt.audio.play_data(data, mc.arg(1), f"data{p:x}"))

@impl(AUD, "vMAudioStop", "VmMp3StopBystream", "VmMp3StopByFile",
      "VmAMRStopPlay", "CB_AUD_StopPlayEx")
def audio_stop(mc, rt):
    from .audio import STOPPED
    rt.audio.state = STOPPED
    rt.audio.stop_proc()
    mc.ret(1)

@impl(AUD, "vMAudioPause", "VmMp3PauseByStream", "VmMp3PauseByFile")
def audio_pause(mc, rt):
    from .audio import PLAYING, PAUSED
    if rt.audio.state == PLAYING:
        rt.audio.state = PAUSED
    mc.ret(1)

@impl(AUD, "vMAudioResume", "VmMp3ResumeByStream", "VmMp3ResumeByFile")
def audio_resume(mc, rt):
    from .audio import PLAYING, PAUSED
    if rt.audio.state == PAUSED:
        rt.audio.state = PLAYING
    mc.ret(1)

@impl(AUD, "vMAduioGetState")
def audio_state(mc, rt):
    mc.ret(rt.audio.state)

@impl(AUD, "vMAudioSetVolume")
def audio_volume(mc, rt):
    rt.audio.volume = mc.arg(0)
    mc.ret(0)

@impl(AUD, None)
def audio_other(mc, rt):

    mc.ret(0)

@impl(IM, "reserver_func01", "reserver_func02", "reserver_func05",
      "reserver_func06", "reserver_func07", "reserver_func08")
def im_reserved_more(mc, rt):
    mc.ret(0)

@impl(SYS, "vMAudioIsSupportInCb")
def sys_audio_cap(mc, rt):

    mc.ret(1)

@impl(SYS, "VmSupportOpenCamera", "VmOpenCamera", "vmSysIsHaveNetWork", "vMIsSimReady")
def sys_caps(mc, rt):

    mc.ret(0)

def _clip_blit(rt, img, x, y, cx, cy, cw, ch, alpha):

    info = rt.images.info(img)
    if not info or not info[0]:
        return
    _, iw, ih, _ = info
    x0, y0 = max(x, cx), max(y, cy)
    x1, y1 = min(x + iw, cx + cw), min(y + ih, cy + ch)
    if x1 <= x0 or y1 <= y0:
        return
    rt.images.blit(img, rt.fb.img, x0, y0, x1 - x0, y1 - y0, x0 - x, y0 - y, alpha=alpha)

def _clip2(mc, rt, alpha):

    img = mc.arg(0)
    if not img or not mc.r32(img):
        mc.ret(1); return
    _clip_blit(rt, img, _s16(mc.arg(1)), _s16(mc.arg(2)),
               _s16(mc.arg(5)), _s16(mc.arg(6)), _s16(mc.arg(3)), _s16(mc.arg(4)), alpha)
    mc.ret(1)

@impl(LCD, "VMDrawImageWithClip2")
def draw_img_clip2(mc, rt):
    _clip2(mc, rt, False)

@impl(LCD, "VMDrawImageClipAndAlpha2")
def draw_img_clip_alpha2(mc, rt):
    _clip2(mc, rt, True)

def _draw_img_clip(mc, rt, alpha):

    pt, r0, r1 = mc.arg(1), mc.arg(2), mc.arg(3)
    x, y = _s16(pt & 0xFFFF), _s16(pt >> 16)
    l, t = _s16(r0 & 0xFFFF), _s16(r0 >> 16)
    rr, b = _s16(r1 & 0xFFFF), _s16(r1 >> 16)
    _clip_blit(rt, mc.arg(0), x, y, l, t, rr - l + 1, b - t + 1, alpha)
    mc.ret(1)

@impl(LCD, "VMDrawImageWithClip")
def draw_img_clip(mc, rt):
    _draw_img_clip(mc, rt, False)

@impl(LCD, "VMDrawImageClipAndAlpha")
def draw_img_clip_alpha(mc, rt):
    _draw_img_clip(mc, rt, True)

@impl(LCD, "vmResGetTxtWithDataPackage", "VmResGetDefTxt", "vmResGetTxtForGame")
def res_get_txt(mc, rt):

    p = rt.state.setdefault("emptystr", mc.heap.alloc(4, "emptystr"))
    mc.uc.mem_write(p, b"\x00\x00\x00\x00")
    mc.ret(p)

@impl(IO, "Vm_file_delete")
def file_delete(mc, rt):
    import os
    hp = rt.vfs.host_path(mc.arg(0), wstr(mc, mc.arg(1)))
    try:
        os.remove(hp); mc.ret(1)
    except OSError:
        mc.ret(0)

@impl(NET, "SetDeactiveFlag")
def set_deactive(mc, rt):
    mc.ret(0)

@impl(SYS, "VmGetPrjCustom", "VmGetOperatorMCC", "VmGetOperatorMNC")
def prj_custom(mc, rt):
    mc.ret(0)

import math

FIX = 4096

@impl(UTIL, "DF_Sin")
@impl(GAME, "DF_Sin")
def df_sin(mc, rt):
    d = _s32(mc.arg(0)) % 360
    mc.ret(int(round(math.sin(math.radians(d)) * FIX)) & 0xFFFFFFFF)

@impl(UTIL, "DF_Cos")
@impl(GAME, "DF_Cos")
def df_cos(mc, rt):
    d = _s32(mc.arg(0)) % 360
    mc.ret(int(round(math.cos(math.radians(d)) * FIX)) & 0xFFFFFFFF)

@impl(UTIL, "DF_Degree")
@impl(GAME, "DF_Degree")
def df_degree(mc, rt):
    dx, dy = _s32(mc.arg(0)), _s32(mc.arg(1))
    mc.ret(int(round(math.degrees(math.atan2(dy, dx)))) % 360)

@impl(UTIL, "CdRectPoint")
@impl(GAME, "CdRectPoint")
def cd_rect_point(mc, rt):

    x1, y1, x2, y2 = (_s32(mc.arg(i)) for i in range(4))
    px, py = _s32(mc.arg(4)), _s32(mc.arg(5))
    mc.ret(1 if (x1 <= px <= x2 and y1 <= py <= y2) else 0)

@impl(UTIL, "CdRect")
@impl(GAME, "CdRect")
def cd_rect(mc, rt):

    a = [_s32(mc.arg(i)) for i in range(4)]
    b = [_s32(mc.arg(i)) for i in range(4, 8)]
    hit = a[0] <= b[2] and b[0] <= a[2] and a[1] <= b[3] and b[1] <= a[3]
    mc.ret(1 if hit else 0)

@impl(UTIL, "CdRectPoint2")
@impl(GAME, "CdRectPoint2")
def cd_rect_point2(mc, rt):

    x, y, w, h = (_s32(mc.arg(i)) for i in range(4))
    px, py = _s32(mc.arg(4)), _s32(mc.arg(5))
    mc.ret(1 if (x <= px <= x + w and y <= py <= y + h) else 0)

@impl(UTIL, "CdRect2")
@impl(GAME, "CdRect2")
def cd_rect2(mc, rt):

    ax, ay, aw, ah = (_s32(mc.arg(i)) for i in range(4))
    bx, by, bw, bh = (_s32(mc.arg(i)) for i in range(4, 8))
    hit = ax <= bx + bw and bx <= ax + aw and ay <= by + bh and by <= ay + ah
    mc.ret(1 if hit else 0)

@impl(UTIL, "Sqrt")
@impl(GAME, "Sqrt")
def df_sqrt(mc, rt):
    v = _s32(mc.arg(0))
    mc.ret(int(math.isqrt(v)) if v > 0 else 0)

@impl(UTIL, "DF_GetFormatString", "formatString")
@impl(GAME, "DF_GetFormatString")
def df_format(mc, rt):
    from .hostabi import vm_printf
    s = vm_printf(mc, mc.arg(0), argidx=1).encode("latin1", "replace")
    p = mc.heap.alloc(len(s) + 1, "fmtstr")
    mc.uc.mem_write(p, s + b"\x00")
    mc.ret(p)

def _s32(v):
    v &= 0xFFFFFFFF
    return v - (1 << 32) if v >> 31 else v

@impl(LCD, "VMCreateImage", "VMCreateImageFromInRes")
def create_image(mc, rt):

    rid, out = mc.arg(0) & 0xFFFF, mc.arg(1)
    arch = rt.mod.icons or rt.mod.res
    ents = arch.entries if arch else []
    if rid >= len(ents):

        arch = rt.mod.res
        ents = arch.entries if arch else []
    if rid >= len(ents):
        rt.trace_io(f"VMCreateImage: 资源 id {rid} 不存在")
        mc.ret(0); return
    try:
        img = _decode_img(ents[rid].data)
    except Exception as e:
        rt.trace_io(f"VMCreateImage({rid}) 解码失败: {e}")
        mc.ret(0); return
    if not img:
        mc.ret(0); return
    mc.ret(rt.images.upload(img, out or None))

@impl(LCD, "VMDestoryImage")
def destroy_image(mc, rt):
    img_release(mc, rt)

@impl(LCD, "IMG_InitDataPage", "IMG_InitDataPageEx", "IMG_InitInnerDataPageEx",
      "IMG_InitDataPageTxt", "IMG_ReleaseDataPage",
      "vMImageDecoderRegImageCodecHandler")
def img_datapage(mc, rt):
    mc.ret(0)

@impl(LCD, "VMDrawRect")
def draw_rect(mc, rt):

    r0, r1, color = mc.arg(0), mc.arg(1), mc.arg(2)
    l, t = _s16(r0 & 0xFFFF), _s16(r0 >> 16)
    rr, b = _s16(r1 & 0xFFFF), _s16(r1 >> 16)
    w, h = rr - l + 1, b - t + 1
    rt.fill_rect(l, t, w, 1, color); rt.fill_rect(l, b, w, 1, color)
    rt.fill_rect(l, t, 1, h, color); rt.fill_rect(rr, t, 1, h, color)
    mc.ret(1)

@impl(IO, "Vm_get_sdcardStatus", "Vm_get_sdcardStatusEx")
def sdcard(mc, rt):
    mc.ret(0)

@impl(SYS, "VMGetPrjVersion")
def prj_version(mc, rt):
    if mc.arg(0):
        mc.uc.mem_write(mc.arg(0), b"V017\x00")
    mc.ret(17)

@impl(SYS, "VmGetCDownFileName", "VmGetCDownAppUrl")
def cdown_str(mc, rt):
    if mc.arg(0):
        mc.uc.mem_write(mc.arg(0), b"\x00")
    mc.ret(0)

@impl("VmCtrlManagerTag", "VmPubDrawSoftkeyBarEx", "VmPubDrawSoftkeyBar")
def softkey_bar(mc, rt):

    rt.fill_rect(0, rt.screen_h - 20, rt.screen_w, 20, 0x2104)
    mc.ret(0)

@impl(IO, "Vm_file_mkdir", "Vm_file_rmdir")
def file_mkdir(mc, rt):
    import os
    hp = rt.vfs.host_path(mc.arg(0), wstr(mc, mc.arg(1)))
    try:
        os.makedirs(hp, exist_ok=True); mc.ret(1)
    except OSError:
        mc.ret(0)

@impl("VmCtrlManagerTag", "vmPubDrawWinTitleEx", "vmPubDrawWinTitle")
def win_title(mc, rt):

    rt.fill_rect(0, 0, rt.screen_w, 20, 0x39C7)
    mc.ret(0)

@impl(LCD, "vMFillRectWithImage", "vMFillRectWithImageEx")
def fill_rect_with_image(mc, rt):

    x, y, w, h = (_s16(mc.arg(i)) for i in range(4))
    img = mc.arg(4)
    info = rt.images.info(img)
    if not info or not info[0] or w <= 0 or h <= 0:
        mc.ret(0); return
    _, iw, ih, _ = info
    for yy in range(y, y + h, max(ih, 1)):
        for xx in range(x, x + w, max(iw, 1)):
            rt.images.blit(img, rt.fb.img, xx, yy,
                           min(iw, x + w - xx), min(ih, y + h - yy), 0, 0, alpha=True)
    mc.ret(1)
