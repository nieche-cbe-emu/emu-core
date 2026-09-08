
use crate::api::fill;
use crate::machine::{Emu, Mach};

pub fn memcpy(uc: &mut Emu) {
    let (d, s, n) = (uc.arg(0), uc.arg(1), uc.arg(2));
    if n != 0 {
        if let Ok(b) = uc.mem_read_as_vec(s as u64, n as usize) {
            uc.write(d, &b);
        }
    }
    uc.ret(d);
}

pub fn memset(uc: &mut Emu) {
    let (d, c, n) = (uc.arg(0), (uc.arg(1) & 0xFF) as u8, uc.arg(2));
    if n != 0 {
        fill(uc, d, c, n);
    }
    uc.ret(d);
}

pub fn strlen(uc: &mut Emu) {
    let n = uc.cstr(uc.arg(0), 0x10000).map(|v| v.len()).unwrap_or(0);
    uc.ret(n as u32);
}

pub fn strcpy(uc: &mut Emu) {
    let d = uc.arg(0);
    let mut s = uc.cstr(uc.arg(1), 512).unwrap_or_default();
    s.push(0);
    uc.write(d, &s);
    uc.ret(d);
}

pub fn strncpy(uc: &mut Emu) {
    let (d, n) = (uc.arg(0), uc.arg(2) as usize);
    let mut s = uc.cstr(uc.arg(1), 512).unwrap_or_default();
    s.resize(n, 0);
    uc.write(d, &s);
    uc.ret(d);
}

pub fn strcat(uc: &mut Emu) {
    let d = uc.arg(0);
    let cur = uc.cstr(d, 512).unwrap_or_default();
    let mut s = uc.cstr(uc.arg(1), 512).unwrap_or_default();
    s.push(0);
    uc.write(d + cur.len() as u32, &s);
    uc.ret(d);
}

pub fn atoi(uc: &mut Emu) {
    let raw = uc.cstr(uc.arg(0), 512).unwrap_or_default();
    let s: Vec<u8> = raw
        .iter()
        .copied()
        .skip_while(|c| c.is_ascii_whitespace())
        .collect();
    let mut m: Vec<u8> = Vec::new();
    for &c in &s {

        let sign_ok = (c == b'+' || c == b'-') && m.is_empty();
        if sign_ok || c.is_ascii_digit() {
            m.push(c);
        } else {
            break;
        }
    }
    let v = String::from_utf8_lossy(&m).parse::<i64>().unwrap_or(0);
    uc.ret(v as u32);
}

pub fn rand(uc: &mut Emu) {
    let st = uc.get_data().rt.rand_state;
    let st = (1103515245u64.wrapping_mul(st as u64).wrapping_add(12345) & 0x7FFF_FFFF) as u32;
    uc.get_data_mut().rt.rand_state = st;
    uc.ret(st);
}
