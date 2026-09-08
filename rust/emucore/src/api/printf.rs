
use crate::machine::{Emu, Mach};

fn match_spec(f: &[u8], at: usize) -> Option<(u8, usize)> {
    let mut i = at;
    if f.get(i) != Some(&b'%') {
        return None;
    }
    i += 1;
    while matches!(f.get(i), Some(b'-' | b'+' | b' ' | b'#' | b'0')) {
        i += 1;
    }
    while matches!(f.get(i), Some(c) if c.is_ascii_digit() || *c == b'*') {
        i += 1;
    }
    if f.get(i) == Some(&b'.') {
        let start = i + 1;
        let mut j = start;
        while matches!(f.get(j), Some(c) if c.is_ascii_digit() || *c == b'*') {
            j += 1;
        }

        if j == start {
            return None;
        }
        i = j;
    }
    while matches!(f.get(i), Some(b'h' | b'l' | b'L')) {
        i += 1;
    }
    let c = *f.get(i)?;
    if b"diouxXeEfgGcsp%".contains(&c) {
        Some((c, i + 1 - at))
    } else {
        None
    }
}

pub fn vm_printf(uc: &Emu, fmt_ptr: u32, argidx: u32) -> Vec<u8> {
    let Some(fmt) = uc.cstr(fmt_ptr, 512) else {
        return b"<null>".to_vec();
    };
    let mut out: Vec<u8> = Vec::new();
    let mut i = 0usize;
    let mut ai = argidx;
    while i < fmt.len() {
        let Some((conv, len)) = match_spec(&fmt, i) else {
            out.push(fmt[i]);
            i += 1;
            continue;
        };
        i += len;
        if conv == b'%' {
            out.push(b'%');
            continue;
        }
        let a = uc.arg(ai);
        ai += 1;
        match conv {
            b's' => {

                let s = uc.cstr(a, 512).unwrap_or_default();
                if s.is_empty() {
                    out.extend_from_slice(b"<null>");
                } else {
                    out.extend_from_slice(&s);
                }
            }
            b'd' | b'i' | b'u' => {
                let v: i64 = if conv == b'd' && a >> 31 != 0 {
                    a as i64 - (1i64 << 32)
                } else {
                    a as i64
                };
                out.extend_from_slice(v.to_string().as_bytes());
            }
            b'x' => out.extend_from_slice(format!("{a:x}").as_bytes()),
            b'X' => out.extend_from_slice(format!("{a:X}").as_bytes()),
            b'o' => out.extend_from_slice(format!("{a:o}").as_bytes()),
            b'c' => out.push((a & 0xFF) as u8),
            b'p' => out.extend_from_slice(format!("{a:#010x}").as_bytes()),
            _ => out.extend_from_slice(b"<f>"),
        }
    }
    out
}

pub fn sprintf(uc: &mut Emu) {
    let s = vm_printf(uc, uc.arg(1), 2);
    let dst = uc.arg(0);
    let mut v = s.clone();
    v.push(0);
    uc.write(dst, &v);
    uc.ret(s.len() as u32);
}

pub fn printf(uc: &mut Emu) {
    let s = vm_printf(uc, uc.arg(0), 1);
    let msg = String::from_utf8_lossy(&s).trim_end().to_string();
    uc.get_data_mut().rt.logs.push(msg);
    uc.ret(s.len() as u32);
}
