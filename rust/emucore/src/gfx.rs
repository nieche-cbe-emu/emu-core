
use std::collections::HashMap;

use crate::machine::{Emu, Mach};

pub fn stride_of(w: u32) -> u32 {
    w + ((4u32.wrapping_sub(w)) & 3)
}

pub struct Gfx {

    pub buf: u32,

    pub img: u32,
    pub w: u32,
    pub h: u32,

    pub bytes: u32,

    pub frames: u64,

    pub masks: HashMap<u32, Vec<u8>>,

    pub by_stream: HashMap<u32, u32>,
}

impl Default for Gfx {
    fn default() -> Self {
        Gfx {
            buf: 0,
            img: 0,
            w: 240,
            h: 400,
            bytes: 240 * 400 * 2,
            frames: 0,
            masks: HashMap::new(),
            by_stream: HashMap::new(),
        }
    }
}

pub fn init_fb(uc: &mut Emu, w: u32, h: u32) {
    let bytes = w * h * 2;
    let buf = uc
        .get_data_mut()
        .heap
        .alloc(bytes, "LCD", true)
        .expect("帧缓冲分配失败");
    crate::api::fill(uc, buf, 0, bytes);
    let img = uc
        .get_data_mut()
        .heap
        .alloc(12, "VmImageType(screen)", true)
        .expect("VmImageType 分配失败");
    uc.w32(img, buf);
    uc.w16(img + 4, w as u16);
    uc.w16(img + 6, h as u16);
    uc.w32(img + 8, 0);
    let g = &mut uc.get_data_mut().rt.gfx;
    g.buf = buf;
    g.img = img;
    g.w = w;
    g.h = h;
    g.bytes = bytes;
    g.frames = 0;
}

pub const ADOPT_BEFORE_FRAME: u64 = 4;

pub const ADOPT_RANGE: (u32, u32, u32, u32) = (80, 320, 80, 480);

pub fn maybe_adopt(uc: &mut Emu, w: u32, h: u32) {
    let (lo_w, hi_w, lo_h, hi_h) = ADOPT_RANGE;
    {
        let g = &uc.get_data().rt.gfx;
        if g.frames >= ADOPT_BEFORE_FRAME
            || (w, h) == (g.w, g.h)
            || !(lo_w <= w && w <= hi_w && lo_h <= h && h <= hi_h)
            || w * h * 2 > g.bytes
        {
            return;
        }
    }
    let buf = uc.get_data().rt.gfx.buf;
    let bytes = w * h * 2;
    crate::api::fill(uc, buf, 0, bytes);
    let img = uc.get_data().rt.gfx.img;
    uc.w16(img + 4, w as u16);
    uc.w16(img + 6, h as u16);
    uc.w32(img + 8, 0);
    let g = &mut uc.get_data_mut().rt.gfx;
    g.w = w;
    g.h = h;
    g.bytes = bytes;
}

pub fn raw565(uc: &Emu) -> Vec<u8> {
    let g = &uc.get_data().rt.gfx;
    let n = (g.w * g.h * 2) as usize;
    let mut raw = uc.mem_read_as_vec(g.buf as u64, n).unwrap_or_default();
    if !uc.le() {
        for c in raw.chunks_exact_mut(2) {
            c.swap(0, 1);
        }
    }
    raw
}

pub fn nonblank(uc: &Emu) -> u32 {
    raw565(uc).chunks_exact(2).filter(|c| c != &[0, 0]).count() as u32
}

pub fn fill_rect(uc: &mut Emu, x: i32, y: i32, w: i32, h: i32, color: u16) {
    let (gw, gh, buf) = {
        let g = &uc.get_data().rt.gfx;
        (g.w as i32, g.h as i32, g.buf)
    };
    let (x0, y0) = (x.max(0), y.max(0));
    let (x1, y1) = ((x + w).min(gw), (y + h).min(gh));
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    let px = if uc.le() {
        color.to_le_bytes()
    } else {
        color.to_be_bytes()
    };
    let row: Vec<u8> = px.iter().copied().cycle().take(((x1 - x0) * 2) as usize).collect();
    for yy in y0..y1 {
        let at = buf as i32 + (yy * gw + x0) * 2;
        uc.write(at as u32, &row);
    }
}

struct Info {
    data: u32,
    w: u32,
    h: u32,
    st: u32,
}

fn info(uc: &Emu, addr: u32) -> Option<Info> {
    if addr == 0 {
        return None;
    }
    let data = uc.r32(addr);
    let w = uc.r16(addr + 4) as u32;
    let h = uc.r16(addr + 6) as u32;
    Some(Info {
        data,
        w,
        h,
        st: stride_of(w),
    })
}

pub fn upload(uc: &mut Emu, im: &cbelib::Image, out_addr: u32) -> u32 {
    let (w, h) = (im.width as u32, im.height as u32);
    let st = stride_of(w);
    let bytes = (st * h * 2).max(2);
    let data = uc
        .get_data_mut()
        .heap
        .alloc(bytes, "img_data", false)
        .unwrap_or(0);
    if data == 0 {
        return 0;
    }
    let le = uc.le();
    let mut raw = vec![0u8; bytes as usize];
    for y in 0..h as usize {
        for x in 0..w as usize {
            let v = im.rgb565.get(y * w as usize + x).copied().unwrap_or(0);
            let b = if le { v.to_le_bytes() } else { v.to_be_bytes() };
            let o = (y * st as usize + x) * 2;
            raw[o] = b[0];
            raw[o + 1] = b[1];
        }
    }
    uc.write(data, &raw);

    let vt = if out_addr != 0 {
        out_addr
    } else {
        uc.get_data_mut()
            .heap
            .alloc(12, "VmImageType", false)
            .unwrap_or(0)
    };
    if vt == 0 {
        return 0;
    }
    uc.w32(vt, data);
    uc.w16(vt + 4, w as u16);
    uc.w16(vt + 6, h as u16);
    uc.w32(vt + 8, 0);

    let src_mask: Option<Vec<u8>> = match (im.transparent, &im.index, &im.alpha) {
        (Some(t), Some(idx), _) => Some(idx.iter().map(|&v| u8::from(v == t)).collect()),
        (_, _, Some(a)) => Some(a.clone()),
        _ => None,
    };
    if let Some(sm) = src_mask {
        let mut mask = vec![0u8; (st * h) as usize];
        for y in 0..h as usize {
            for x in 0..w as usize {
                if sm.get(y * w as usize + x).copied().unwrap_or(0) != 0 {
                    mask[y * st as usize + x] = 1;
                }
            }

            for x in w as usize..st as usize {
                mask[y * st as usize + x] = 1;
            }
        }
        uc.get_data_mut().rt.gfx.masks.insert(data, mask);
    }
    vt
}

#[allow(clippy::too_many_arguments)]
pub fn blit(
    uc: &mut Emu,
    src: u32,
    dst: u32,
    mut dx: i32,
    mut dy: i32,
    w: Option<i32>,
    h: Option<i32>,
    mut sx: i32,
    mut sy: i32,
    alpha: bool,
) {
    let (Some(s), Some(d)) = (info(uc, src), info(uc, dst)) else {
        return;
    };
    if s.data == 0 || d.data == 0 {
        return;
    }
    let mut w = w.unwrap_or(s.w as i32);
    let mut h = h.unwrap_or(s.h as i32);

    w = w.min(d.w as i32);
    if dy + h > d.h as i32 {
        h = d.h as i32 - dy;
    }
    if sy + h > s.h as i32 {
        h = s.h as i32 - sy;
    }
    if dx + w > d.st as i32 {
        w = d.st as i32 - dx;
    }
    if sx + w > s.st as i32 {
        w = s.st as i32 - sx;
    }
    if dx < 0 {
        sx -= dx;
        w += dx;
        dx = 0;
    }
    if dy < 0 {
        sy -= dy;
        h += dy;
        dy = 0;
    }
    if w <= 0 || h <= 0 || sx < 0 || sy < 0 {
        return;
    }

    let mask = if alpha {
        uc.get_data().rt.gfx.masks.get(&s.data).cloned()
    } else {
        None
    };

    let (srow, drow) = ((s.st * 2) as usize, (d.st * 2) as usize);
    let sbuf = match uc.mem_read_as_vec((s.data as i32 + sy * srow as i32) as u64, h as usize * srow)
    {
        Ok(v) => v,
        Err(_) => return,
    };
    let dbase = (d.data as i32 + dy * drow as i32) as u32;
    let mut dbuf = match uc.mem_read_as_vec(dbase as u64, h as usize * drow) {
        Ok(v) => v,
        Err(_) => return,
    };
    let (sx2, dx2, w2) = (sx as usize * 2, dx as usize * 2, w as usize * 2);
    for row in 0..h as usize {
        let so = row * srow + sx2;
        let dof = row * drow + dx2;
        if so + w2 > sbuf.len() || dof + w2 > dbuf.len() {
            break;
        }
        match &mask {
            None => dbuf[dof..dof + w2].copy_from_slice(&sbuf[so..so + w2]),
            Some(mk) => {
                let mo = (sy as usize + row) * s.st as usize + sx as usize;
                let mrow = mk.get(mo..mo + w as usize).unwrap_or(&[]);
                if mrow.iter().all(|&t| t == 0) {
                    dbuf[dof..dof + w2].copy_from_slice(&sbuf[so..so + w2]);
                } else {
                    for (i, &t) in mrow.iter().enumerate() {
                        if t == 0 {
                            dbuf[dof + i * 2] = sbuf[so + i * 2];
                            dbuf[dof + i * 2 + 1] = sbuf[so + i * 2 + 1];
                        }
                    }
                }
            }
        }
    }
    uc.write(dbase, &dbuf);
}
