
use std::collections::HashMap;

const RAW: &[u8] = include_bytes!("font12.cbef");
const MAGIC: &[u8] = b"CBEF";

pub struct Font {

    pub aw: u32,
    pub ah: u32,
    pub hw: u32,
    pub hh: u32,
    abpr: usize,
    hbpr: usize,
    ascii: &'static [u8],
    hanzi: &'static [u8],
    rows: HashMap<(bool, i32), Vec<Vec<u32>>>,
    mcache: HashMap<Vec<u8>, u32>,
}

impl Font {
    pub fn load() -> Option<Font> {
        let d = RAW;
        if d.len() < 18 || &d[..4] != MAGIC {
            return None;
        }
        let (aw, ah, hw, hh) = (d[6] as u32, d[7] as u32, d[8] as u32, d[9] as u32);
        let n_ascii = u32::from_le_bytes([d[10], d[11], d[12], d[13]]) as usize;
        let n_hanzi = u32::from_le_bytes([d[14], d[15], d[16], d[17]]) as usize;
        let abpr = ((aw + 7) / 8) as usize;
        let hbpr = ((hw + 7) / 8) as usize;
        let mut o = 18usize;
        let alen = n_ascii * abpr * ah as usize;
        let ascii = d.get(o..o + alen)?;
        o += alen;
        let hlen = n_hanzi * hbpr * hh as usize;
        let hanzi = d.get(o..o + hlen)?;
        Some(Font {
            aw,
            ah,
            hw,
            hh,
            abpr,
            hbpr,
            ascii,
            hanzi,
            rows: HashMap::new(),
            mcache: HashMap::new(),
        })
    }

    fn glyph_rows(&mut self, is_hanzi: bool, idx: i32) -> &Vec<Vec<u32>> {
        let key = (is_hanzi, idx);
        if !self.rows.contains_key(&key) {
            let (w, h, bpr, buf) = if is_hanzi {
                (self.hw, self.hh, self.hbpr, self.hanzi)
            } else {
                (self.aw, self.ah, self.abpr, self.ascii)
            };
            let base = idx.max(0) as usize * bpr * h as usize;
            let mut out: Vec<Vec<u32>> = Vec::with_capacity(h as usize);
            if idx >= 0 && base + bpr * h as usize <= buf.len() {
                for y in 0..h as usize {
                    let mut bits: u128 = 0;
                    for k in 0..bpr {
                        bits = (bits << 8) | buf[base + y * bpr + k] as u128;
                    }
                    let top = bpr * 8 - 1;
                    out.push(
                        (0..w)
                            .filter(|x| bits & (1u128 << (top - *x as usize)) != 0)
                            .collect(),
                    );
                }
            } else {
                out = vec![Vec::new(); h as usize];
            }
            self.rows.insert(key, out);
        }
        &self.rows[&key]
    }

    pub fn iter_glyphs(&self, b: &[u8]) -> Vec<(bool, i32, u32)> {
        let mut out = Vec::new();
        let (mut i, n) = (0usize, b.len());
        while i < n {
            let c = b[i];
            if c >= 0xA1 && i + 1 < n && b[i + 1] >= 0xA1 {
                let (hi, lo) = ((c - 0xA1) as i32, (b[i + 1] - 0xA1) as i32);
                out.push((true, hi * 94 + lo, self.hw));
                i += 2;
            } else if c >= 0x80 && i + 1 < n {

                out.push((true, -1, self.hw));
                i += 2;
            } else {
                out.push((false, if c < 128 { c as i32 } else { 0 }, self.aw));
                i += 1;
            }
        }
        out
    }

    pub fn measure(&mut self, b: &[u8]) -> u32 {
        if let Some(&w) = self.mcache.get(b) {
            return w;
        }
        let w: u32 = self.iter_glyphs(b).iter().map(|&(_, _, x)| x).sum();
        if self.mcache.len() > 4096 {
            self.mcache.clear();
        }
        self.mcache.insert(b.to_vec(), w);
        w
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw(
    uc: &mut crate::machine::Emu,
    buf: u32,
    stride: u32,
    sw: u32,
    sh: u32,
    text: &[u8],
    mut x: i32,
    y: i32,
    color: u16,
) {
    use crate::machine::Mach;
    let le = uc.le();
    let col = if le {
        color.to_le_bytes()
    } else {
        color.to_be_bytes()
    };
    let glyphs = {
        let Some(f) = uc.get_data().rt.font_data.as_ref() else {
            return;
        };
        f.iter_glyphs(text)
    };
    for (is_h, idx, w) in glyphs {
        if idx < 0 || x >= sw as i32 {
            x += w as i32;
            continue;
        }
        let rows = {
            let f = uc.get_data_mut().rt.font_data.as_mut().unwrap();
            f.glyph_rows(is_h, idx).clone()
        };
        for (dy, cols) in rows.iter().enumerate() {
            if cols.is_empty() {
                continue;
            }
            let yy = y + dy as i32;
            if yy < 0 || yy >= sh as i32 {
                continue;
            }
            let x0 = x.max(0);
            let x1 = (x + w as i32).min(sw as i32);
            if x1 <= x0 {
                continue;
            }
            let off = buf + (yy as u32 * stride + x0 as u32) * 2;
            let n = ((x1 - x0) * 2) as usize;
            let Ok(mut line) = uc.mem_read_as_vec(off as u64, n) else {
                continue;
            };
            let mut dirty = false;
            for &cx in cols {
                let px = x + cx as i32;
                if px >= x0 && px < x1 {
                    let k = ((px - x0) * 2) as usize;
                    line[k] = col[0];
                    line[k + 1] = col[1];
                    dirty = true;
                }
            }
            if dirty {
                uc.write(off, &line);
            }
        }
        x += w as i32;
    }
}
