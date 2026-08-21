
import collections
from .machine import Machine
from .hostabi import vm_printf
from . import vmspec
from . import api as apireg
from .lcd import Framebuffer, write_svg
from .vfs import Vfs
from . import paths
from .images import ImageStore
from .font import load as load_font
from .audio import Audio

class Runtime:
    def __init__(self, mod, trace=True, quiet_log=False, screen=(240, 400),
                 fsroot=None, fsbase="assets/fatfs", trace_fs=True, audio=False,
                 pc_trace=False):
        self.mod = mod
        self.mach = m = Machine(mod, trace=False)
        self.mach.rt = self

        if pc_trace or trace:
            self.mach.enable_pc_trace()
        self.trace = trace
        self.quiet_log = quiet_log
        self.logs = []
        self.unimpl = collections.Counter()
        self.host_errors = collections.Counter()
        self.old_syscalls = collections.Counter()
        self.tick_spin = 0
        self.style = None
        self.managers = {}
        self.screen_w, self.screen_h = screen
        self.state = {}
        self.net_responses = {}

        init_map = {}
        for off, (nm, _t) in vmspec.MGR["VmManagerTag"].items():
            key = nm.lower().replace("vminit", "").replace("init", "", 1)
            if "init" not in nm.lower():
                continue
            for goff, (gnm, _tag) in vmspec.SYS.items():
                if gnm.lower().replace("vmget", "").replace("get", "", 1) == key:
                    init_map[off] = goff
                    break

        self.sys_tbl = m.data.alloc(vmspec.SIZE["VmManagerTag"], "VmManager")
        for off in range(0, vmspec.SIZE["VmManagerTag"], 4):
            ent = vmspec.MGR["VmManagerTag"].get(off)
            name = ent[0] if ent else f"VmManager+{off:#x}"
            if off in vmspec.SYS:
                h = self._make_getter(off)
            elif off in init_map:
                h = self._make_initer(init_map[off])
            else:
                h = lambda mc: mc.ret(0)
            m.w32(self.sys_tbl + off, m.new_trap(name, h))

        self.host = m.data.alloc(0x10, "VmSysCallRegParam")
        m.w32(self.host + 0x08, self.sys_tbl)
        m.w32(self.host + 0x0c, m.new_trap("gm_TRACE", self._vm_log))
        self.fb = Framebuffer(m, self.screen_w, self.screen_h)
        self.images = ImageStore(m)
        self.frame_hook = None
        self.vfs = Vfs(fsroot or paths.fs_dir(mod.name), fsbase)
        self.trace_fs = trace_fs
        self.screens = []
        self.pending = []
        self._installed = {}
        self.font = load_font()
        self.audio = Audio(self, outdir=f"out/{mod.name}/audio", play=audio)
        self.tick = 0
        self.frame_ms = 40
        self.timers = {}
        self._next_timer = 1
        self.text_layer = []
        self.pointer = (0, 0)
        self.keys_down = 0
        self.keys_up = 0
        self.keys_hold = 0
        self.touch_down = self.touch_up = self.touch_hold = self.touch_drag = 0
        self.mod_cb0 = self.mod_cb1 = 0

    def _make_getter(self, off):
        def getter(mc):
            mc.ret(self.get_manager(off))
        return getter

    def _make_initer(self, getoff):
        def initer(mc):
            dst = mc.arg(0)
            src = self.get_manager(getoff)
            tag = vmspec.SYS[getoff][1]
            n = vmspec.SIZE.get(tag, 0x400) or 0x400
            if dst:
                mc.uc.mem_write(dst, bytes(mc.uc.mem_read(src, n)))
            mc.ret(0)
        return initer

    def get_manager(self, sysoff):
        if sysoff in self.managers:
            return self.managers[sysoff]
        m = self.mach
        getter, tag = vmspec.SYS[sysoff]

        size = (vmspec.SIZE.get(tag, 0x400) or 0x400) + 0x100
        addr = m.data.alloc(size, tag or f"mgr{sysoff:02x}")
        self.managers[sysoff] = addr
        table = vmspec.MGR.get(tag, {})
        for off in range(0, size, 4):
            ent = table.get(off)
            nm = ent[0] if ent else f"{(tag or 'mgr')[:-3]}+{off:#x}"
            const = self.NATIVE_CONSTS.get(nm)
            if const is not None:

                m.w32(addr + off, m.native_const(nm, const(self)))
                continue
            m.w32(addr + off, m.new_trap(f"{nm}", self._make_method(tag, off, nm)))
        return addr

    NATIVE_CONSTS = {
        "VMGetCurrMainScreenImage": lambda rt: rt.fb.img,
        "VMGetLCDBuffer":           lambda rt: rt.fb.buf,
        "VmGetScreenWidth":         lambda rt: rt.screen_w,
        "GetScreenWidth":           lambda rt: rt.screen_w,
        "VmGetScreenHeight":        lambda rt: rt.screen_h,
        "GetScreenHeight":          lambda rt: rt.screen_h,
    }

    def _make_method(self, tag, off, nm):
        fn = apireg.lookup(tag, nm)

        def method(mc):
            if self.trace and self.mach.log_calls:
                a = [mc.arg(i) for i in range(4)]
                print(f"  {nm:34s}({a[0]:#x}, {a[1]:#x}, {a[2]:#x}, {a[3]:#x})"
                      f"   <- {mc.where(mc.lr() & ~1)}")
            if fn:
                try:
                    fn(mc, self)
                except Exception as e:

                    self.host_errors[(nm, type(e).__name__, str(e)[:60])] += 1
                    mc.ret(0)
            else:
                self.unimpl[(tag, off, nm)] += 1
                mc.ret(0)
        return method

    S_INIT, S_DESTROY, S_LOGIC, S_RENDER, S_PAUSE, S_RESUME, S_LOADRES = range(7)
    SPIN_PER_MS = 64

    def push_screen(self, scr, param, flag):
        m = self.mach
        self.screens.append((scr, param, flag))

        line = (f"  >> vmAddScreen VmScreen@{scr:#x}: " + " ".join(
            f"{f}={m.r32(scr + 4 * i):#x}" for i, f in enumerate(
                ("init", "destroy", "logic", "render", "pause", "resume", "loadRes"))))
        self.logs.append(line)
        if not self.quiet_log:
            print(line)

        self.defer(m.r32(scr + 4 * self.S_INIT), (param,), "screenInit")
        self.defer(m.r32(scr + 4 * self.S_LOADRES), (param,), "screenLoadResource")

    def defer(self, fn, args=(), tag=""):

        if fn:
            self.pending.append((fn, tuple(args), tag))

    def pump(self, limit=64):
        n = 0
        while self.pending and n < limit:
            fn, args, tag = self.pending.pop(0)
            if self.trace and tag:
                print(f"  >> 回调 {tag} @{fn:#x}{args}")
            self.mach.call(fn, args)
            n += 1
        return n

    def call_screen(self, scr, slot, *args):
        fn = self.mach.r32(scr + 4 * slot)
        if not fn:
            return None
        return self.mach.call(fn, args)

    def start_timer(self, ms, cb, param):
        if not cb:
            return 0
        tid = self._next_timer
        self._next_timer = self._next_timer % 0x7FFF + 1
        self.timers[tid] = [self.tick + max(ms, 1), cb, param]
        return tid

    def stop_timer(self, tid):
        self.timers.pop(tid, None)

    def _fire_timers(self):
        due = [(t, v) for t, v in self.timers.items() if v[0] <= self.tick]
        for tid, (_, cb, param) in sorted(due, key=lambda kv: kv[1][0]):
            self.timers.pop(tid, None)
            self.defer(cb, (param,), f"timer#{tid}")

    def frame(self, event=0, data=0):

        self.tick += self.frame_ms
        self.audio.tick()
        self._fire_timers()
        self.pump()
        for scr, param, _ in reversed(self.screens):
            if self.call_screen(scr, self.S_LOGIC, param, event, data):
                break
        for scr, param, _ in self.screens:
            self.call_screen(scr, self.S_RENDER, param)
        self.present()

    def net_response(self, url):

        return self.net_responses.get(url) if self.net_responses else None

    def input_query(self, mc, kind="down"):
        mask = mc.arg(0)
        bits = {"down": self.keys_down, "up": self.keys_up,
                "hold": self.keys_hold}.get(kind, 0)
        return 1 if (bits & mask) else 0

    def press(self, mask):
        self.keys_down |= mask
        self.keys_hold |= mask
        self.keys_up &= ~mask

    def release(self, mask=None):
        if mask is None:
            self.keys_up = self.keys_down
            self.keys_down = self.keys_hold = 0
        else:
            self.keys_down &= ~mask
            self.keys_hold &= ~mask
            self.keys_up |= mask

    exit_requested = False

    def clear_input(self):
        self.keys_down = self.keys_up = self.keys_hold = 0

    def note_text(self, s, x, y, color, size=12):

        try:
            txt = s.decode("gb18030", "replace") if isinstance(s, bytes) else str(s)
        except Exception:
            return
        if txt.strip():
            self.text_layer.append((x, y, color, txt, size))

    def write_svg(self, path, scale=1):
        return write_svg(path, self.fb, self.text_layer, scale)

    def trace_io(self, msg):
        if self.trace_fs:
            print(f"  [fs] {msg}")

    def lcd_buffer(self):
        return self.fb.buf

    def lcd_image(self):
        return self.fb.img

    def fill_rect(self, x, y, w, h, color):

        if x <= 0 and y <= 0 and w >= self.screen_w and h >= self.screen_h:
            self.text_layer.clear()
        self.fb.fill_rect(x, y, w, h, color)

    def present(self):
        self.fb.frames += 1
        if self.frame_hook:
            self.frame_hook(self.fb)

    def install(self, addr, name, fn):

        key = (addr, name)
        if key not in self._installed:
            self._installed[key] = self.mach.new_trap(name, lambda mc, _f=fn: _f(mc, self))
        self.mach.w32(addr, self._installed[key])
        return self._installed[key]

    def _vm_log(self, mc):
        msg = vm_printf(mc, mc.arg(0)).rstrip()
        self.logs.append(msg)
        if not self.quiet_log:
            print(f"  [trace] {msg}")
        mc.ret(len(msg))

    ENTRY_NEW, ENTRY_OLD = "new", "old"

    def entry_style(self):

        import capstone
        from capstone import arm
        mode = capstone.CS_MODE_THUMB | (0 if self.mach.le
                                         else capstone.CS_MODE_BIG_ENDIAN)
        md = capstone.Cs(capstone.CS_ARCH_ARM, mode)
        md.detail = True

        argregs = {arm.ARM_REG_R0}
        for i in md.disasm(self.mod.ro[:0x40], 0):
            if i.mnemonic == "bl":
                break
            ops = i.operands
            if i.mnemonic.startswith("str"):
                if len(ops) == 2 and ops[1].type == arm.ARM_OP_MEM:
                    if ops[1].mem.base in argregs:
                        return self.ENTRY_NEW
                    if ops[0].reg in argregs:
                        return self.ENTRY_OLD
                continue

            if (i.mnemonic in ("mov", "movs", "add", "adds")
                    and len(ops) >= 2 and ops[0].type == arm.ARM_OP_REG
                    and ops[1].type == arm.ARM_OP_REG
                    and (len(ops) == 2
                         or (ops[2].type == arm.ARM_OP_IMM and ops[2].imm == 0))
                    and ops[1].reg in argregs):
                argregs.add(ops[0].reg)
                continue
            for r in i.regs_access()[1]:
                argregs.discard(r)
        return self.ENTRY_NEW

    OLD_REGISTER_APP = 1950

    def _old_syscall(self, mc):

        sid = mc.arg(0)
        self.old_syscalls[sid] += 1
        if sid == self.OLD_REGISTER_APP:
            blk = mc.arg(1)
            if blk:
                self.mod_cb0 = mc.r32(blk + 0)
                self.mod_cb1 = mc.r32(blk + 4)
                mc.w32(blk + 8, 1)
        mc.ret(0)

    def boot(self):
        m = self.mach

        self.style = self.entry_style()

        tramp = m.new_trap("OldSysCall", self._old_syscall)
        m.w32(self.host + 0x00, 0xE51FF004)
        m.w32(self.host + 0x04, tramp)
        r = m.call(m.ro_base | 1, [self.host])
        cb0 = m.r32(self.host + 0x00)
        if cb0 != 0xE51FF004:
            self.mod_cb0 = cb0
            self.mod_cb1 = m.r32(self.host + 0x04)

        return r

    def app_start(self):
        r = self.mach.call(self.mod_cb0)
        self.pump()
        return r

    def app_stop(self):
        return self.mach.call(self.mod_cb1)

    def report_null_calls(self, top=8):
        nc = getattr(self.mach, "null_calls", None)
        if not nc:
            return
        print("\n调用了空函数指针（已兜底返回 0）:")
        for lr, n in nc.most_common(top):
            print(f"   调用点 {self.mach.where(lr)}  x{n}")

    def report_errors(self, top=15):
        if not self.host_errors:
            return
        print("\n宿主实现内部报错（不影响继续执行，但说明参数解读可能不对）:")
        for (nm, kind, msg), n in self.host_errors.most_common(top):
            print(f"   {nm:30s} {kind}: {msg}   x{n}")

    def report_unimpl(self, top=40):
        if not self.unimpl:
            print("\n所有被调用的 API 均已实现。")
            return
        print(f"\n未实现的宿主 API（按调用次数）:")
        for (tag, off, nm), n in self.unimpl.most_common(top):
            print(f"   {str(tag or '?'):24s} +{off:#05x}  {nm:34s} x{n}")
