
use crate::machine::{Emu, Mach};

pub fn read_short(uc: &mut Emu) {
    let (buf, ppos) = (uc.arg(0), uc.arg(1));
    let pos = if ppos != 0 { uc.r32(ppos) } else { 0 };
    let b0 = uc.r8(buf + pos) as u32;
    let b1 = uc.r8(buf + pos + 1) as u32;
    if ppos != 0 {
        uc.w32(ppos, pos + 2);
    }
    let v = b0 | (b1 << 8);
    uc.ret(if v & 0x8000 != 0 {
        (v as i32 - 0x10000) as u32
    } else {
        v
    });
}

pub fn read_int(uc: &mut Emu) {
    let (buf, ppos) = (uc.arg(0), uc.arg(1));
    let pos = if ppos != 0 { uc.r32(ppos) } else { 0 };
    let mut v = 0u32;
    for k in 0..4u32 {
        v |= (uc.r8(buf + pos + k) as u32) << (8 * k);
    }
    if ppos != 0 {
        uc.w32(ppos, pos + 4);
    }
    uc.ret(v);
}

pub fn write_short(uc: &mut Emu) {
    let (buf, ppos, v) = (uc.arg(0), uc.arg(1), uc.arg(2) & 0xFFFF);
    let pos = if ppos != 0 { uc.r32(ppos) } else { 0 };
    uc.write(buf + pos, &[(v & 0xFF) as u8, ((v >> 8) & 0xFF) as u8]);
    if ppos != 0 {
        uc.w32(ppos, pos + 2);
    }
    uc.ret(0);
}

pub fn write_int(uc: &mut Emu) {
    let (buf, ppos, v) = (uc.arg(0), uc.arg(1), uc.arg(2));
    let pos = if ppos != 0 { uc.r32(ppos) } else { 0 };
    uc.write(buf + pos, &v.to_le_bytes());
    if ppos != 0 {
        uc.w32(ppos, pos + 4);
    }
    uc.ret(0);
}

pub fn font_width(uc: &mut Emu) {
    let full = uc.arg(0) != 0;
    let (aw, hw) = {
        let f = &uc.get_data().rt.font;
        (f.aw, f.hw)
    };
    uc.ret(if full { hw } else { aw });
}

pub fn font_height(uc: &mut Emu) {
    let h = uc.get_data().rt.font.hh;
    uc.ret(h);
}

pub fn get_memblock(uc: &mut Emu) {
    let cur = uc.get_data().rt.dfblock;
    if cur != 0 {
        uc.ret(cur);
        return;
    }
    let blk = uc
        .get_data_mut()
        .heap
        .alloc(0x18, "MEMORY_BLOCK(DF)", false)
        .unwrap_or(0);
    if blk != 0 {
        uc.setreg(0, blk);
        uc.setreg(1, 0x40000);
        crate::api::sysmisc::init_memory_block(uc);
        uc.get_data_mut().rt.dfblock = blk;
    }
    uc.ret(blk);
}

pub fn get_stream_data(uc: &mut Emu) {
    let p = uc.arg(0);
    if p == 0 {
        uc.ret(0);
        return;
    }
    let hdr = uc.read_upto(p, 9);
    if hdr.len() < 9 || hdr[0] != 2 {
        uc.ret(p + 9);
        return;
    }

    let comp = u32::from_be_bytes([hdr[1], hdr[2], hdr[3], hdr[4]]) as usize;
    let raw = uc.read_upto(p, 9 + comp);
    let data = cbelib::unpack_entry(&raw).unwrap_or_default();
    let buf = uc
        .get_data_mut()
        .heap
        .alloc(data.len().max(1) as u32, "stream", false)
        .unwrap_or(0);
    if buf != 0 && !data.is_empty() {
        uc.write(buf, &data);
    }
    uc.ret(buf);
}

fn read_string_n(uc: &mut Emu, lenbytes: u32) {
    let (buf, ppos) = (uc.arg(0), uc.arg(1));
    let pos = if ppos != 0 { uc.r32(ppos) } else { 0 };
    let mut n = 0u32;
    for k in 0..lenbytes {
        n |= (uc.r8(buf + pos + k) as u32) << (8 * k);
    }

    if n > 0x10000 {
        n = 0;
    }
    let s = if n != 0 {
        uc.mem_read_as_vec((buf + pos + lenbytes) as u64, n as usize)
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    if ppos != 0 {
        uc.w32(ppos, pos + lenbytes + n);
    }
    let p = uc
        .get_data_mut()
        .heap
        .alloc(n + 1, "df_str", false)
        .unwrap_or(0);
    if p != 0 {
        let mut v = s;
        v.push(0);
        uc.write(p, &v);
    }
    uc.ret(p);
}

pub fn read_string(uc: &mut Emu) {
    read_string_n(uc, 1);
}

pub fn read_string2(uc: &mut Emu) {
    read_string_n(uc, 4);
}
