
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

fn blit8(uc: &mut Emu, alpha: bool) {
    let (dst, src) = (uc.arg(0), uc.arg(1));

    if src == 0 || uc.r32(src) == 0 {
        uc.ret(1);
        return;
    }
    let (sx, sy) = (s16(uc.arg(2)), s16(uc.arg(3)));
    let (w, h) = (s16(uc.arg(4)), s16(uc.arg(5)));
    let (dx, dy) = (s16(uc.arg(6)), s16(uc.arg(7)));
    let target = if dst != 0 {
        dst
    } else {
        uc.get_data().rt.gfx.img
    };
    gfx::blit(uc, src, target, dx, dy, Some(w), Some(h), sx, sy, alpha);
    uc.ret(1);
}

pub fn draw_img_clip_ex(uc: &mut Emu) {
    blit8(uc, false);
}

pub fn draw_img_clip_alpha_ex(uc: &mut Emu) {
    blit8(uc, true);
}

fn fill_and_adopt(uc: &mut Emu, x: i32, y: i32, w: i32, h: i32, color: u16) {
    if x <= 0 && y <= 0 {
        gfx::maybe_adopt(uc, w.max(0) as u32, h.max(0) as u32);
    }
    gfx::fill_rect(uc, x, y, w, h, color);
}

pub fn fill_rect(uc: &mut Emu) {
    let v = uc.arg(0);
    let (l, t) = ((v & 0xFFFF) as i32, ((v >> 16) & 0xFFFF) as i32);
    let v2 = uc.arg(1);
    let (r, b) = ((v2 & 0xFFFF) as i32, ((v2 >> 16) & 0xFFFF) as i32);
    let color = uc.arg(2) as u16;
    fill_and_adopt(uc, l, t, r - l + 1, b - t + 1, color);
    uc.ret(1);
}

pub fn fill_rect_ex(uc: &mut Emu) {
    let (x, y, w, h) = (
        uc.arg(0) as i32,
        uc.arg(1) as i32,
        uc.arg(2) as i32,
        uc.arg(3) as i32,
    );
    let color = uc.arg(4) as u16;
    fill_and_adopt(uc, x, y, w, h, color);
    uc.ret(1);
}

pub fn img_from_stream(uc: &mut Emu) {
    let (stream, out) = (uc.arg(0), uc.arg(1));
    if out == 0 {
        if let Some(&vt) = uc.get_data().rt.gfx.by_stream.get(&stream) {
            uc.ret(vt);
            return;
        }
    }
    let raw = uc.read_upto(stream, 0x80000);
    let Some(im) = cbelib::decode_image(&raw) else {
        uc.ret(0);
        return;
    };
    let vt = gfx::upload(uc, &im, out);
    if vt != 0 {
        uc.get_data_mut().rt.gfx.by_stream.insert(stream, vt);
    }
    uc.ret(vt);
}

pub fn release_image(uc: &mut Emu) {
    let vt = uc.arg(0);
    let screen = uc.get_data().rt.gfx.img;
    if vt != 0 && vt != screen {
        let data = uc.r32(vt);
        if data != 0 {
            uc.get_data_mut().rt.gfx.masks.remove(&data);
            uc.get_data_mut().heap.free(data);
        }
        uc.w32(vt, 0);
    }
    uc.ret(0);
}

pub fn draw_img_ex(uc: &mut Emu) {
    let src = uc.arg(0);
    let (x, y) = (s16(uc.arg(1)), s16(uc.arg(2)));
    let screen = uc.get_data().rt.gfx.img;
    gfx::blit(uc, src, screen, x, y, None, None, 0, 0, false);
    uc.ret(1);
}

fn draw_img_inner(uc: &mut Emu, alpha: bool) {
    let src = uc.arg(0);
    let pt = uc.arg(1);
    let (x, y) = (s16(pt & 0xFFFF), s16(pt >> 16));
    let screen = uc.get_data().rt.gfx.img;
    gfx::blit(uc, src, screen, x, y, None, None, 0, 0, alpha);
    uc.ret(1);
}

pub fn draw_img(uc: &mut Emu) {
    draw_img_inner(uc, false);
}

pub fn draw_img_alpha(uc: &mut Emu) {
    draw_img_inner(uc, true);
}

#[allow(clippy::too_many_arguments)]
fn clip_blit(uc: &mut Emu, img: u32, x: i32, y: i32, cx: i32, cy: i32, cw: i32, ch: i32, alpha: bool) {
    if img == 0 || uc.r32(img) == 0 {
        return;
    }
    let iw = uc.r16(img + 4) as i32;
    let ih = uc.r16(img + 6) as i32;
    let (x0, y0) = (x.max(cx), y.max(cy));
    let (x1, y1) = ((x + iw).min(cx + cw), (y + ih).min(cy + ch));
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    let screen = uc.get_data().rt.gfx.img;
    gfx::blit(uc, img, screen, x0, y0, Some(x1 - x0), Some(y1 - y0), x0 - x, y0 - y, alpha);
}

fn clip2(uc: &mut Emu, alpha: bool) {
    let img = uc.arg(0);
    if img == 0 || uc.r32(img) == 0 {
        uc.ret(1);
        return;
    }
    let (x, y) = (s16(uc.arg(1)), s16(uc.arg(2)));
    let (cw, ch) = (s16(uc.arg(3)), s16(uc.arg(4)));
    let (cx, cy) = (s16(uc.arg(5)), s16(uc.arg(6)));
    clip_blit(uc, img, x, y, cx, cy, cw, ch, alpha);
    uc.ret(1);
}

pub fn draw_img_clip2(uc: &mut Emu) {
    clip2(uc, false);
}

pub fn draw_img_clip_alpha2(uc: &mut Emu) {
    clip2(uc, true);
}

fn draw_img_clip(uc: &mut Emu, alpha: bool) {
    let img = uc.arg(0);
    if img == 0 || uc.r32(img) == 0 {
        uc.ret(1);
        return;
    }
    let (pt, r0, r1) = (uc.arg(1), uc.arg(2), uc.arg(3));
    let (x, y) = (s16(pt & 0xFFFF), s16(pt >> 16));
    let (l, t) = (s16(r0 & 0xFFFF), s16(r0 >> 16));
    let (rr, b) = (s16(r1 & 0xFFFF), s16(r1 >> 16));
    clip_blit(uc, img, x, y, l, t, rr - l + 1, b - t + 1, alpha);
    uc.ret(1);
}

pub fn draw_img_with_clip(uc: &mut Emu) {
    draw_img_clip(uc, false);
}

pub fn draw_img_clip_alpha(uc: &mut Emu) {
    draw_img_clip(uc, true);
}

pub fn img_from_res(uc: &mut Emu) {
    let p = uc.arg(0);
    let raw = uc.read_upto(p, 64);
    if let Some(nul) = raw.iter().position(|&c| c == 0) {
        if nul > 0 && nul <= 48 {
            let name = String::from_utf8_lossy(&raw[..nul]).to_string();
            let q = crate::api::dfpkg::res_ptr_by_name(uc, &name);
            if q != 0 {
                uc.setreg(0, q);
                uc.setreg(1, 0);
                img_from_stream(uc);
                return;
            }
        }
    }
    img_from_stream(uc);
}
