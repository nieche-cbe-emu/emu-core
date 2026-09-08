
use std::fmt;

pub const MAGIC_TAIL: &[u8] = b"CoolBars";
pub const FOOTER_LEN: usize = 44;

#[derive(Debug)]
pub struct CbeError(pub String);

impl fmt::Display for CbeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for CbeError {}

macro_rules! err {
    ($($t:tt)*) => { CbeError(format!($($t)*)) };
}

pub type Result<T> = std::result::Result<T, CbeError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endian {
    Le,
    Be,
}

impl Endian {
    pub fn as_str(self) -> &'static str {
        match self {
            Endian::Le => "LE",
            Endian::Be => "BE",
        }
    }
}

#[derive(Clone)]
pub struct ResEntry {
    pub name: String,

    pub off: usize,
    pub size: usize,
    pub data: Vec<u8>,
}

impl fmt::Debug for ResEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResEntry")
            .field("name", &self.name)
            .field("off", &self.off)
            .field("size", &self.size)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct ResArchive {

    pub base: usize,
    pub size: usize,
    pub count: usize,

    pub data_off: usize,
    pub data_size: usize,
    pub entries: Vec<ResEntry>,
}

impl ResArchive {
    pub fn get(&self, name: &str) -> Option<&ResEntry> {
        self.entries.iter().find(|e| e.name == name)
    }

    pub fn names(&self) -> Vec<&str> {
        self.entries.iter().map(|e| e.name.as_str()).collect()
    }
}

fn u32be(buf: &[u8], at: usize) -> Result<u32> {
    buf.get(at..at + 4)
        .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| err!("读大端 u32 越界 @{at:#x}"))
}

fn u32le(buf: &[u8], at: usize) -> Result<u32> {
    buf.get(at..at + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| err!("读小端 u32 越界 @{at:#x}"))
}

fn skip_fe(buf: &[u8], mut o: usize) -> usize {
    while o < buf.len() && buf[o] == 0xFE {
        o += 1;
    }
    o
}

pub fn parse_res(buf: &[u8], base: usize, size: usize, variant: bool) -> Result<Option<ResArchive>> {
    if size < 0x1C {
        return Ok(None);
    }
    let (index_size, data_size, count, hdr, isz_at);
    if variant {
        index_size = u32le(buf, base)? as usize;
        data_size = u32le(buf, base + 4)? as usize;
        count = u32le(buf, base + 8)? as usize;
        hdr = 0x10;
        isz_at = 0x00;
    } else {
        let magic = u32le(buf, base)?;
        match magic {
            8 => {
                index_size = u32le(buf, base + 0x0C)? as usize;
                data_size = u32le(buf, base + 0x10)? as usize;
                count = u32le(buf, base + 0x14)? as usize;
                hdr = 0x1C;
                isz_at = 0x0C;
            }
            4 => {

                index_size = u32le(buf, base + 0x08)? as usize;
                data_size = u32le(buf, base + 0x0C)? as usize;
                count = u32le(buf, base + 0x10)? as usize;
                hdr = 0x18;
                isz_at = 0x08;
            }
            _ => return Ok(None),
        }
    }
    if count == 0 || count > 0x10000 {
        return Ok(None);
    }
    let data_off = isz_at + 4 + index_size;
    if data_off + data_size != size {
        return Err(err!(
            "res archive size mismatch: {data_off:#x}+{data_size:#x} != {size:#x}"
        ));
    }

    let mut offs = Vec::with_capacity(count);
    offs.push(0usize);
    for k in 0..count.saturating_sub(1) {
        offs.push(u32le(buf, base + hdr + 4 * k)? as usize);
    }

    let mut o = base + hdr + 4 * (count - 1);
    let mut names = Vec::with_capacity(count);
    for _ in 0..count {
        let ln = *buf
            .get(o)
            .ok_or_else(|| err!("名字表越界 @{o:#x}"))? as usize;
        let nm = buf
            .get(o + 1..o + 1 + ln)
            .ok_or_else(|| err!("名字越界 @{o:#x}"))?;
        names.push(nm.iter().map(|&c| c as char).collect::<String>());
        o += 1 + ln;
    }
    if o - base != data_off {
        return Err(err!(
            "res name table ends at {:#x}, expected {data_off:#x}",
            o - base
        ));
    }

    let mut entries = Vec::with_capacity(count);
    for (i, (nm, &off)) in names.iter().zip(offs.iter()).enumerate() {
        let end = if i + 1 < count { offs[i + 1] } else { data_size };
        let abs = base + data_off + off;
        let len = end.saturating_sub(off);
        let data = buf.get(abs..abs + len).unwrap_or(&[]).to_vec();
        entries.push(ResEntry {
            name: nm.clone(),
            off,
            size: len,
            data,
        });
    }
    Ok(Some(ResArchive {
        base,
        size,
        count,
        data_off,
        data_size,
        entries,
    }))
}

pub fn parse_multi(buf: &[u8], base: usize, size: usize) -> Option<Vec<(String, ResArchive)>> {
    if size < 16 {
        return None;
    }
    let count = u32le(buf, base + 8).ok()? as usize;
    if count == 0 || count >= 512 {
        return None;
    }
    let mut o = base + 12;
    let mut ents: Vec<(String, usize)> = Vec::new();
    for _ in 0..count {
        if o >= base + size {
            break;
        }
        let ln = *buf.get(o)? as usize;
        if ln == 0 || ln > 48 || o + 1 + ln + 4 > base + size {
            break;
        }
        let nm = buf.get(o + 1..o + 1 + ln)?;
        if !nm.iter().all(|&c| (32..127).contains(&c)) {
            break;
        }
        o += 1 + ln;
        let off = u32le(buf, o).ok()? as usize;
        o += 4;
        if off >= size {
            break;
        }
        ents.push((nm.iter().map(|&c| c as char).collect(), off));
    }
    if ents.is_empty() {
        return None;
    }

    let mut packs: Vec<(String, ResArchive)> = Vec::new();
    let mut bounds: Vec<usize> = ents.iter().map(|&(_, off)| off).collect();
    bounds.push(size);
    for (i, (nm, off)) in ents.iter().enumerate() {
        if let Ok(Some(a)) = parse_res(buf, base + off, bounds[i + 1] - off, true) {
            packs.push((nm.clone(), a));
        }
    }

    let root_off = o - base;
    if ents.len() < count && root_off < ents[0].1 {
        if let Ok(Some(a)) = parse_res(buf, o, ents[0].1 - root_off, true) {
            packs.push((String::new(), a));
        }
    }
    if packs.is_empty() {
        None
    } else {
        Some(packs)
    }
}

fn detect_endian(d: &[u8]) -> Endian {
    fn count_fixed(d: &[u8], pat: &[u8; 4]) -> usize {
        let mut n = 0;
        let mut i = 0;
        while i + 4 <= d.len() {
            if &d[i..i + 4] == pat {
                n += 1;
                i += 4;
            } else {
                i += 1;
            }
        }
        n
    }
    fn count_push(d: &[u8], be: bool) -> usize {
        let mut n = 0;
        let mut i = 0;
        while i + 4 <= d.len() {
            let hit = if be {
                d[i] == 0xE9 && d[i + 1] == 0x2D && (0x40..=0x4F).contains(&d[i + 2])
            } else {
                (0x40..=0x4F).contains(&d[i + 1]) && d[i + 2] == 0x2D && d[i + 3] == 0xE9
            };
            if hit {
                n += 1;
                i += 4;
            } else {
                i += 1;
            }
        }
        n
    }
    let le = count_fixed(d, &[0x1E, 0xFF, 0x2F, 0xE1]) + count_push(d, false);
    let be = count_fixed(d, &[0xE1, 0x2F, 0xFF, 0x1E]) + count_push(d, true);
    if le >= be {
        Endian::Le
    } else {
        Endian::Be
    }
}

pub struct CbeModule {
    pub path: String,
    pub raw: Vec<u8>,
    pub name: String,
    pub load_base: u32,
    pub image_size: u32,
    pub image_end: u32,

    pub rw_size: u32,
    pub ro: Vec<u8>,
    pub rw: Vec<u8>,
    pub ro_off: usize,
    pub rw_off: usize,
    pub ro_chk: u32,
    pub rw_chk: u32,
    pub endian: Endian,

    pub icons: Option<ResArchive>,

    pub res: Option<ResArchive>,

    pub packages: Vec<(String, ResArchive)>,
}

impl fmt::Debug for CbeModule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CbeModule")
            .field("name", &self.name)
            .field("endian", &self.endian.as_str())
            .field("load_base", &format_args!("{:#x}", self.load_base))
            .field("ro", &self.ro.len())
            .field("rw", &self.rw.len())
            .field("res", &self.res.as_ref().map(|a| a.count))
            .field("packages", &self.packages.len())
            .finish()
    }
}

pub fn load(path: &str) -> Result<CbeModule> {
    let d = std::fs::read(path).map_err(|e| err!("{path}: {e}"))?;
    load_bytes(path, d)
}

pub fn load_bytes(path: &str, d: Vec<u8>) -> Result<CbeModule> {
    if !d.ends_with(MAGIC_TAIL) {
        return Err(err!("missing 'CoolBars' trailer — not a CBE module?"));
    }
    if d.len() < FOOTER_LEN + 12 {
        return Err(err!("file too small: {}", d.len()));
    }
    let foot = u32be(&d, d.len() - 12)? as usize;
    if foot != d.len() - FOOTER_LEN {
        return Err(err!(
            "footer offset {foot:#x} != {:#x}",
            d.len() - FOOTER_LEN
        ));
    }

    let mut o = 0usize;
    let mut vals = [0u32; 5];
    for (i, v) in vals.iter_mut().enumerate() {
        o = skip_fe(&d, o);
        *v = u32be(&d, o).map_err(|e| err!("头部第 {i} 个字段: {e}"))?;
        o += 4;
    }
    o = skip_fe(&d, o);
    let nl = vals[4] as usize;
    let name = d
        .get(o..o + nl)
        .ok_or_else(|| err!("模块名越界"))?
        .iter()
        .map(|&c| c as char)
        .collect::<String>();
    o += nl;

    let mut sizes = [0u32; 6];
    for s in sizes.iter_mut() {
        o = skip_fe(&d, o);
        *s = u32be(&d, o)?;
        o += 4;
    }

    let [load_base, image_size, image_end, rw_size, _nl] = vals;
    let [ro_sz, ro_chk, rw_sz, rw_chk, ico_sz, _ico_chk] = sizes;
    let (ro_sz, rw_sz, ico_sz) = (ro_sz as usize, rw_sz as usize, ico_sz as usize);

    let ro_off = skip_fe(&d, o);
    let rw_off = skip_fe(&d, ro_off + ro_sz);
    let ico_off = skip_fe(&d, rw_off + rw_sz);
    let e = skip_fe(&d, ico_off + ico_sz);
    let res_size = u32be(&d, e)? as usize;
    let res_off = skip_fe(&d, e + 4);

    let mut res = parse_res(&d, res_off, res_size, false).unwrap_or(None);
    let mut packages = Vec::new();
    if res.is_none() {
        if let Some(p) = parse_multi(&d, res_off, res_size) {

            res = p
                .iter()
                .max_by_key(|(_, a)| a.count)
                .map(|(_, a)| a.clone());
            packages = p;
        }
    }

    let ro = d
        .get(ro_off..ro_off + ro_sz)
        .ok_or_else(|| err!("RO 段越界"))?
        .to_vec();
    let rw = d
        .get(rw_off..rw_off + rw_sz)
        .ok_or_else(|| err!("RW 段越界"))?
        .to_vec();
    let icons = parse_res(&d, ico_off, ico_sz, false).unwrap_or(None);
    let endian = detect_endian(&d);

    Ok(CbeModule {
        path: path.to_string(),
        raw: d,
        name,
        load_base,
        image_size,
        image_end,
        rw_size,
        ro,
        rw,
        ro_off,
        rw_off,
        ro_chk,
        rw_chk,
        endian,
        icons,
        res,
        packages,
    })
}
