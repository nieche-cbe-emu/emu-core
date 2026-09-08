
use std::fmt;

#[derive(Debug)]
pub struct ImgError(pub String);

impl fmt::Display for ImgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

macro_rules! ierr {
    ($($t:tt)*) => { ImgError(format!($($t)*)) };
}

type R<T> = std::result::Result<T, ImgError>;

#[derive(Debug, Clone, Default)]
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub rgb565: Vec<u16>,

    pub transparent: Option<u8>,

    pub alpha: Option<Vec<u8>>,

    pub index: Option<Vec<u8>>,
}

fn lzw(data: &[u8], min_code_size: u8) -> Vec<u8> {
    let clear = 1usize << min_code_size;
    let end = clear + 1;
    let mut code_size = min_code_size as usize + 1;
    let fresh = || -> Vec<Vec<u8>> {
        let mut d: Vec<Vec<u8>> = (0..clear).map(|i| vec![i as u8]).collect();
        d.push(Vec::new());
        d.push(Vec::new());
        d
    };
    let mut dic = fresh();
    let mut out: Vec<u8> = Vec::new();
    let mut prev: Option<Vec<u8>> = None;
    let mut bitpos = 0usize;
    let nbits = data.len() * 8;
    while bitpos + code_size <= nbits {
        let byte = bitpos >> 3;
        let mut chunk = 0u32;
        for k in 0..3 {
            if byte + k < data.len() {
                chunk |= (data[byte + k] as u32) << (8 * k);
            }
        }
        let code = ((chunk >> (bitpos & 7)) & ((1u32 << code_size) - 1)) as usize;
        bitpos += code_size;
        if code == clear {
            dic = fresh();
            code_size = min_code_size as usize + 1;
            prev = None;
            continue;
        }
        if code == end {
            break;
        }
        let entry: Vec<u8> = if code < dic.len() {
            dic[code].clone()
        } else if let Some(p) = &prev {
            let mut e = p.clone();
            e.push(p[0]);
            e
        } else {
            break;
        };
        out.extend_from_slice(&entry);
        if let Some(p) = &prev {
            let mut n = p.clone();
            n.push(entry[0]);
            dic.push(n);
            if dic.len() == (1usize << code_size) && code_size < 12 {
                code_size += 1;
            }
        }
        prev = Some(entry);
    }
    out
}

fn blocks(buf: &[u8], mut o: usize) -> (Vec<u8>, usize) {
    let mut out = Vec::new();
    while o < buf.len() {
        let n = buf[o] as usize;
        o += 1;
        if n == 0 {
            break;
        }
        let end = (o + n).min(buf.len());
        out.extend_from_slice(&buf[o.min(buf.len())..end]);
        o += n;
    }
    (out, o)
}

fn deinterlace(idx: &[u8], w: usize, h: usize) -> Vec<u8> {
    let mut out = vec![0u8; w * h];
    let mut rows: Vec<usize> = Vec::with_capacity(h);
    rows.extend((0..h).step_by(8));
    rows.extend((4..h).step_by(8));
    rows.extend((2..h).step_by(4));
    rows.extend((1..h).step_by(2));
    for (src, &dst) in rows.iter().enumerate() {
        let (sa, sb) = (src * w, (src + 1) * w);
        if sb > idx.len() || dst >= h {
            continue;
        }
        out[dst * w..(dst + 1) * w].copy_from_slice(&idx[sa..sb]);
    }
    out
}

fn u16be(b: &[u8], at: usize) -> u16 {
    if at + 2 <= b.len() {
        u16::from_be_bytes([b[at], b[at + 1]])
    } else {
        0
    }
}

fn u16le(b: &[u8], at: usize) -> u16 {
    if at + 2 <= b.len() {
        u16::from_le_bytes([b[at], b[at + 1]])
    } else {
        0
    }
}

pub fn decode_gif_variant(p: &[u8]) -> R<Image> {
    if p.len() < 8 {
        return Err(ierr!("载荷太短"));
    }
    let flags = p[4];
    let mut o = 7usize;
    let mut palette: Vec<u16> = Vec::new();
    if flags & 0x80 != 0 {
        if p[6] != 0 {
            return Err(ierr!("p[6]={:#x}，期望 0", p[6]));
        }
        let n = 1usize << ((flags & 7) + 1);
        for i in 0..n {
            palette.push(u16be(p, o + i * 2));
        }
        o += n * 2;

    }
    let mut transparent: Option<u8> = None;
    while o < p.len() {
        let b = p[o];
        if b == 0x21 {

            if o + 1 >= p.len() {
                break;
            }
            let label = p[o + 1];
            o += 2;
            if label == 0xF9 {

                if o + 4 >= p.len() {
                    break;
                }
                let size = p[o] as usize;
                let gflags = p[o + 1];
                if gflags & 1 != 0 {
                    transparent = Some(p[o + 4]);
                }
                o += size + 1;
                o += 1;
            } else {
                if o >= p.len() {
                    break;
                }
                let size = p[o] as usize;
                o += 1 + size;
                let (_, no) = blocks(p, o);
                o = no;
            }
        } else if b == 0x2C {

            if o + 10 > p.len() {
                break;
            }
            let _left = u16le(p, o + 1) as usize;
            let _top = u16le(p, o + 3) as usize;
            let w = u16le(p, o + 5) as usize;
            let h = u16le(p, o + 7) as usize;
            let lflags = p[o + 9];
            o += 10;
            if lflags & 0x80 != 0 {

                let n = 1usize << ((lflags & 7) + 1);
                palette = (0..n).map(|i| u16be(p, o + i * 2)).collect();
                o += n * 2;
            }
            if o >= p.len() {
                break;
            }
            let mcs = p[o];
            o += 1;
            let (data, _no) = blocks(p, o);
            let mut idx = lzw(&data, mcs);
            if lflags & 0x40 != 0 {
                idx = deinterlace(&idx, w, h);
            }
            let want = w * h;
            let mut rgb565: Vec<u16> = idx
                .iter()
                .take(want)
                .map(|&i| *palette.get(i as usize).unwrap_or(&0))
                .collect();
            rgb565.resize(want, 0);
            let mut index: Vec<u8> = idx.iter().copied().take(want).collect();
            index.resize(want, 0);
            return Ok(Image {
                width: w,
                height: h,
                rgb565,
                transparent,
                alpha: None,
                index: Some(index),
            });
        } else if b == 0x3B {
            break;
        } else {
            o += 1;
        }
    }
    Err(ierr!("没找到图像描述符"))
}

fn paeth(a: i32, b: i32, c: i32) -> i32 {
    let p = a + b - c;
    let (pa, pb, pc) = ((p - a).abs(), (p - b).abs(), (p - c).abs());
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

pub fn decode_png(data: &[u8]) -> R<Image> {
    if data.len() < 8 || &data[..8] != b"\x89PNG\r\n\x1a\n" {
        return Err(ierr!("不是 PNG"));
    }
    let mut o = 8usize;
    let mut idat: Vec<u8> = Vec::new();
    let mut plte: &[u8] = &[];
    let mut trns: &[u8] = &[];
    let (mut w, mut h, mut depth, mut ctype) = (0usize, 0usize, 0u8, 0u8);
    let mut have_ihdr = false;
    while o + 8 <= data.len() {
        let ln = u32::from_be_bytes([data[o], data[o + 1], data[o + 2], data[o + 3]]) as usize;
        let tag = &data[o + 4..o + 8];
        let body = data
            .get(o + 8..(o + 8 + ln).min(data.len()))
            .unwrap_or(&[]);
        match tag {
            b"IHDR" => {
                if body.len() < 13 {
                    return Err(ierr!("IHDR 太短"));
                }
                w = u32::from_be_bytes([body[0], body[1], body[2], body[3]]) as usize;
                h = u32::from_be_bytes([body[4], body[5], body[6], body[7]]) as usize;
                depth = body[8];
                ctype = body[9];
                if body[12] != 0 {
                    return Err(ierr!("暂不支持隔行 PNG"));
                }
                have_ihdr = true;
            }
            b"PLTE" => plte = body,
            b"tRNS" => trns = body,
            b"IDAT" => idat.extend_from_slice(body),
            b"IEND" => break,
            _ => {}
        }
        o += 12 + ln;
    }
    if !have_ihdr {
        return Err(ierr!("PNG 缺少 IHDR"));
    }
    let raw = miniz_oxide::inflate::decompress_to_vec_zlib(&idat)
        .map_err(|e| ierr!("IDAT 解压失败: {e:?}"))?;

    let chans: usize = match ctype {
        0 => 1,
        2 => 3,
        3 => 1,
        4 => 2,
        6 => 4,
        _ => return Err(ierr!("未知 PNG 颜色类型 {ctype}")),
    };
    let (bpp, rowbytes) = if depth == 8 {
        (chans, w * chans)
    } else if matches!(depth, 1 | 2 | 4) && ctype == 3 {
        (1usize, (w * depth as usize + 7) / 8)
    } else {
        return Err(ierr!("暂不支持 PNG 位深 {depth} 类型 {ctype}"));
    };

    let mut out = vec![0u8; rowbytes * h];
    let mut prev = vec![0u8; rowbytes];
    let mut pos = 0usize;
    for y in 0..h {
        if pos >= raw.len() {
            break;
        }
        let f = raw[pos];
        pos += 1;
        let end = (pos + rowbytes).min(raw.len());
        let mut line = vec![0u8; rowbytes];
        line[..end - pos].copy_from_slice(&raw[pos..end]);
        pos += rowbytes;
        match f {
            0 => {}
            1 => {
                for i in bpp..rowbytes {
                    line[i] = line[i].wrapping_add(line[i - bpp]);
                }
            }
            2 => {
                for i in 0..rowbytes {
                    line[i] = line[i].wrapping_add(prev[i]);
                }
            }
            3 => {
                for i in 0..rowbytes {
                    let a = if i >= bpp { line[i - bpp] as u32 } else { 0 };
                    line[i] = line[i].wrapping_add(((a + prev[i] as u32) >> 1) as u8);
                }
            }
            4 => {
                for i in 0..rowbytes {
                    let a = if i >= bpp { line[i - bpp] as i32 } else { 0 };
                    let b = prev[i] as i32;
                    let c = if i >= bpp { prev[i - bpp] as i32 } else { 0 };
                    line[i] = line[i].wrapping_add(paeth(a, b, c) as u8);
                }
            }
            _ => return Err(ierr!("未知 PNG 滤波类型 {f}")),
        }
        out[y * rowbytes..(y + 1) * rowbytes].copy_from_slice(&line);
        prev = line;
    }

    let mut rgb565: Vec<u16> = Vec::with_capacity(w * h);
    let mut alpha: Vec<u8> = Vec::with_capacity(w * h);
    let to565 = |r: u8, g: u8, b: u8| -> u16 {
        (((r as u16) & 0xF8) << 8) | (((g as u16) & 0xFC) << 3) | ((b as u16) >> 3)
    };
    if ctype == 3 {
        let pal: Vec<(u8, u8, u8)> = (0..plte.len() / 3)
            .map(|i| (plte[i * 3], plte[i * 3 + 1], plte[i * 3 + 2]))
            .collect();
        let mut pal_a: Vec<u8> = trns.to_vec();
        pal_a.resize(pal.len().max(trns.len()), 255);
        for y in 0..h {
            let base = y * rowbytes;
            for x in 0..w {
                let idx = if depth == 8 {
                    *out.get(base + x).unwrap_or(&0) as usize
                } else {
                    let per = 8 / depth as usize;
                    let b = *out.get(base + x / per).unwrap_or(&0);
                    let sh = 8 - depth as usize * (x % per + 1);
                    ((b >> sh) & ((1u8 << depth) - 1)) as usize
                };
                let (r, g, bl) = *pal.get(idx).unwrap_or(&(0, 0, 0));
                rgb565.push(to565(r, g, bl));
                alpha.push(if *pal_a.get(idx).unwrap_or(&255) < 128 { 1 } else { 0 });
            }
        }
    } else {
        for y in 0..h {
            let base = y * rowbytes;
            for x in 0..w {
                let p = base + x * chans;
                let g0 = |k: usize| *out.get(p + k).unwrap_or(&0);
                let (r, g, bl, a) = match ctype {
                    0 => (g0(0), g0(0), g0(0), 255),
                    4 => (g0(0), g0(0), g0(0), g0(1)),
                    2 => (g0(0), g0(1), g0(2), 255),
                    _ => (g0(0), g0(1), g0(2), g0(3)),
                };
                rgb565.push(to565(r, g, bl));
                alpha.push(if a < 128 { 1 } else { 0 });
            }
        }
    }
    let any_alpha = alpha.iter().any(|&a| a != 0);
    Ok(Image {
        width: w,
        height: h,
        rgb565,
        transparent: None,
        alpha: if any_alpha { Some(alpha) } else { None },
        index: None,
    })
}

pub fn decode(entry: &[u8]) -> Option<Image> {
    if entry.is_empty() {
        return None;
    }
    match entry[0] {
        0 => {
            let w = u16be(entry, 1) as usize;
            let h = u16be(entry, 3) as usize;
            let mut rgb565 = Vec::with_capacity(w * h);
            for i in 0..w * h {
                rgb565.push(u16le(entry, 8 + i * 2));
            }
            Some(Image {
                width: w,
                height: h,
                rgb565,
                transparent: None,
                alpha: None,
                index: None,
            })
        }
        1 => decode_gif_variant(&entry[1..]).ok(),
        3 => {
            if entry.len() > 9 {
                decode_png(&entry[9..]).ok()
            } else {
                None
            }
        }
        _ => None,
    }
}
