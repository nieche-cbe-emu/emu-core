
use crate::font;
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

fn gbk_cells(b: &[u8]) -> u32 {
    let (mut cells, mut i) = (0u32, 0usize);
    while i < b.len() {
        if b[i] >= 0x80 && i + 1 < b.len() {
            cells += 2;
            i += 2;
        } else {
            cells += 1;
            i += 1;
        }
    }
    cells
}

fn text_width(uc: &mut Emu, s: &[u8]) -> u32 {
    match uc.get_data_mut().rt.font_data.as_mut() {
        Some(f) => f.measure(s),
        None => gbk_cells(s) * 8,
    }
}

pub fn string_width(uc: &mut Emu) {
    let s = uc.cstr(uc.arg(0), 512).unwrap_or_default();
    let w = text_width(uc, &s);
    uc.ret(w);
}

pub fn string_height(uc: &mut Emu) {
    let h = uc.get_data().rt.font.hh;
    uc.ret(h);
}

pub fn char_width(uc: &mut Emu) {
    let c = uc.arg(0);
    let f = uc.get_data().rt.font;
    uc.ret(if c >= 0x80 { f.hw } else { f.aw });
}

fn target(uc: &Emu, img: u32) -> (u32, u32, u32, u32) {
    if img != 0 {
        let data = uc.r32(img);
        if data != 0 {
            let w = uc.r16(img + 4) as u32;
            let h = uc.r16(img + 6) as u32;
            return (data, gfx::stride_of(w), w, h);
        }
    }
    let g = &uc.get_data().rt.gfx;
    (g.buf, g.w, g.w, g.h)
}

pub fn gb18030_decode(b: &[u8]) -> String {
    encoding_rs::GB18030.decode(b).0.into_owned()
}

pub fn gb18030_encode(s: &str) -> Vec<u8> {
    encoding_rs::GB18030.encode(s).0.into_owned()
}

pub fn draw_bytes(uc: &mut Emu, s: &[u8], x: i32, y: i32, color: u16, img: u32) {
    draw_text(uc, s, x, y, color, img);
}

fn draw_text(uc: &mut Emu, s: &[u8], x: i32, y: i32, color: u16, img: u32) {
    if uc.get_data().rt.font_data.is_none() {
        for i in 0..gbk_cells(s) as i32 {
            gfx::fill_rect(uc, x + i * 8 + 1, y + 1, 6, 9, color);
        }
        return;
    }
    let (buf, stride, w, h) = target(uc, img);
    font::draw(uc, buf, stride, w, h, s, x, y, color);
}

pub fn draw_string(uc: &mut Emu) {
    let s = uc.cstr(uc.arg(0), 512).unwrap_or_default();
    let (x, y, c) = (s16(uc.arg(1)), s16(uc.arg(2)), uc.arg(3) as u16);
    draw_text(uc, &s, x, y, c, 0);
    uc.ret(0);
}

pub fn draw_string_ex(uc: &mut Emu) {
    let img = uc.arg(0);
    let s = uc.cstr(uc.arg(1), 512).unwrap_or_default();
    let (x, y, c) = (s16(uc.arg(2)), s16(uc.arg(3)), uc.arg(4) as u16);
    draw_text(uc, &s, x, y, c, img);
    uc.ret(0);
}

pub fn draw_string_clip(uc: &mut Emu) {
    let s = uc.cstr(uc.arg(0), 512).unwrap_or_default();
    let (x, y, c) = (s16(uc.arg(1)), s16(uc.arg(2)), uc.arg(3) as u16);
    draw_text(uc, &s, x, y, c, 0);
    uc.ret(0);
}

pub fn draw_string_rect(uc: &mut Emu) {
    let s = uc.cstr(uc.arg(0), 512).unwrap_or_default();
    draw_string_rect_with(uc, s);
}

pub fn draw_ucs2_string_rect(uc: &mut Emu) {
    let s = super::fileio::ucs2_bytes(uc, uc.arg(0));
    draw_string_rect_with(uc, s);
}

fn draw_string_rect_with(uc: &mut Emu, s: Vec<u8>) {
    let (x, y) = (uc.arg(1) as i32, uc.arg(2) as i32);
    let (w, h) = (uc.arg(3) as i32, uc.arg(4) as i32);
    let color = uc.arg(5) as u16;
    let lh = uc.get_data().rt.font.hh as i32 + 2;

    let mut lines = 0i32;
    let mut cur: Vec<u8> = Vec::new();
    let mut cw = 0i32;
    let mut i = 0usize;
    while i < s.len() {
        let two = s[i] >= 0x80 && i + 1 < s.len();
        let piece: Vec<u8> = if two {
            s[i..i + 2].to_vec()
        } else {
            s[i..i + 1].to_vec()
        };
        let pw = text_width(uc, &piece) as i32;
        if w != 0 && cw + pw > w && !cur.is_empty() {
            if h != 0 && (lines + 1) * lh > h {
                break;
            }
            draw_text(uc, &cur, x, y + lines * lh, color, 0);
            lines += 1;
            cur.clear();
            cw = 0;
        }
        i += piece.len();
        cur.extend_from_slice(&piece);
        cw += pw;
    }
    if !(h != 0 && (lines + 1) * lh > h) && !cur.is_empty() {
        draw_text(uc, &cur, x, y + lines * lh, color, 0);
        lines += 1;
    }
    let full = text_width(uc, &s) as i32;
    let cap = if w != 0 { w } else { 0xFFFF };
    uc.ret(((lines as u32) << 16) | (full.min(cap) as u32 & 0xFFFF));
}
