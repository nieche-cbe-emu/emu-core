
use crate::gfx;
use crate::machine::{Emu, Mach};

fn s16(v: u32) -> i32 {
    let v = v & 0xFFFF;
    if v & 0x8000 != 0 {
        v as i32 - 0x10000
    } else {
        v as i32
    }
}

pub fn get_tick(uc: &mut Emu) {
    let n = uc.get_data().same_trap;
    {
        let d = uc.get_data_mut();
        if n > 64 {
            d.rt.tick += 16;
        } else if n > 8 {
            d.rt.tick += 1;
        }
    }
    let t = uc.get_data().rt.tick as u32;
    uc.ret(t);
}

pub fn current_time(uc: &mut Emu) {
    let p = uc.arg(0);
    if p != 0 {
        crate::api::fill(uc, p, 0, 24);
        uc.w32(p, 2013);
    }
    uc.ret(0);
}

pub fn invalidate(uc: &mut Emu) {
    uc.get_data_mut().rt.frames += 1;
    uc.get_data_mut().rt.gfx.frames += 1;
    uc.ret(0);
}

pub fn image_width(uc: &mut Emu) {
    let p = uc.arg(0);
    uc.ret(if p != 0 { uc.r16(p + 4) as u32 } else { 0 });
}

pub fn image_height(uc: &mut Emu) {
    let p = uc.arg(0);
    uc.ret(if p != 0 { uc.r16(p + 6) as u32 } else { 0 });
}

pub fn draw_rect_ex(uc: &mut Emu) {
    let (x, y, w, h) = (
        uc.arg(0) as i32,
        uc.arg(1) as i32,
        uc.arg(2) as i32,
        uc.arg(3) as i32,
    );
    let c = uc.arg(4) as u16;
    gfx::fill_rect(uc, x, y, w, 1, c);
    gfx::fill_rect(uc, x, y + h - 1, w, 1, c);
    gfx::fill_rect(uc, x, y, 1, h, c);
    gfx::fill_rect(uc, x + w - 1, y, 1, h, c);
    uc.ret(1);
}

pub fn draw_line_ex(uc: &mut Emu) {
    let (x1, y1, x2, y2) = (
        uc.arg(0) as i32,
        uc.arg(1) as i32,
        uc.arg(2) as i32,
        uc.arg(3) as i32,
    );
    let c = uc.arg(4) as u16;
    if y1 == y2 {
        gfx::fill_rect(uc, x1.min(x2), y1, (x2 - x1).abs() + 1, 1, c);
    } else if x1 == x2 {
        gfx::fill_rect(uc, x1, y1.min(y2), 1, (y2 - y1).abs() + 1, c);
    } else {
        let (dx, dy) = (x2 - x1, y2 - y1);
        let n = dx.abs().max(dy.abs()).max(1);
        for i in 0..=n {
            let px = x1 + dx * i / n;
            let py = y1 + dy * i / n;
            gfx::fill_rect(uc, px, py, 1, 1, c);
        }
    }
    uc.ret(1);
}

pub fn softkey_bar(uc: &mut Emu) {
    let (w, h) = {
        let g = &uc.get_data().rt.gfx;
        (g.w as i32, g.h as i32)
    };
    gfx::fill_rect(uc, 0, h - 20, w, 20, 0x2104);
    uc.ret(0);
}

pub fn win_title(uc: &mut Emu) {
    let w = uc.get_data().rt.gfx.w as i32;
    gfx::fill_rect(uc, 0, 0, w, 20, 0x39C7);
    uc.ret(0);
}

pub fn fill_rect_with_image(uc: &mut Emu) {
    let (x, y, w, h) = (
        s16(uc.arg(0)),
        s16(uc.arg(1)),
        s16(uc.arg(2)),
        s16(uc.arg(3)),
    );
    let img = uc.arg(4);
    if img == 0 || uc.r32(img) == 0 || w <= 0 || h <= 0 {
        uc.ret(0);
        return;
    }
    let iw = uc.r16(img + 4) as i32;
    let ih = uc.r16(img + 6) as i32;
    if iw <= 0 || ih <= 0 {
        uc.ret(0);
        return;
    }
    let screen = uc.get_data().rt.gfx.img;
    let mut yy = y;
    while yy < y + h {
        let mut xx = x;
        while xx < x + w {
            let cw = iw.min(x + w - xx);
            let ch = ih.min(y + h - yy);
            gfx::blit(uc, img, screen, xx, yy, Some(cw), Some(ch), 0, 0, false);
            xx += iw;
        }
        yy += ih;
    }
    uc.ret(0);
}

pub fn format_string(uc: &mut Emu) {
    let s = crate::api::printf::vm_printf(uc, uc.arg(0), 1);
    let p = uc
        .get_data_mut()
        .heap
        .alloc(s.len() as u32 + 1, "fmtstr", false)
        .unwrap_or(0);
    if p != 0 {
        let mut v = s;
        v.push(0);
        uc.write(p, &v);
    }
    uc.ret(p);
}

pub fn draw_rect(uc: &mut Emu) {
    let (r0, r1, color) = (uc.arg(0), uc.arg(1), uc.arg(2) as u16);
    let (l, t) = (s16(r0 & 0xFFFF), s16(r0 >> 16));
    let (rr, b) = (s16(r1 & 0xFFFF), s16(r1 >> 16));
    let (w, h) = (rr - l + 1, b - t + 1);
    gfx::fill_rect(uc, l, t, w, 1, color);
    gfx::fill_rect(uc, l, b, w, 1, color);
    gfx::fill_rect(uc, l, t, 1, h, color);
    gfx::fill_rect(uc, rr, t, 1, h, color);
    uc.ret(1);
}
