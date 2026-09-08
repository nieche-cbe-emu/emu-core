
use crate::machine::{Emu, Mach};
use crate::vfs::OPEN_FAIL;

pub const COOLBAR_DIR: &str = ".system/MB_MSTAR_WQVGA";

pub fn wstr(uc: &Emu, addr: u32, maxlen: usize) -> String {
    if addr == 0 {
        return String::new();
    }
    let mut out = String::new();
    for i in 0..maxlen {
        let c = uc.r16(addr + (i as u32) * 2);
        if c == 0 {
            break;
        }
        out.push(char::from_u32(c as u32).unwrap_or('?'));
    }
    out
}

pub fn wwrite(uc: &mut Emu, addr: u32, s: &str) -> u32 {
    let le = uc.le();
    let mut b = Vec::new();
    let mut n = 0u32;
    for ch in s.chars() {
        let v = ch as u32 as u16;
        b.extend_from_slice(&if le { v.to_le_bytes() } else { v.to_be_bytes() });
        n += 1;
    }
    b.extend_from_slice(&[0, 0]);
    uc.write(addr, &b);
    n
}

fn open_mode(uc: &Emu, p: u32) -> String {
    if p == 0 {
        return "r".into();
    }
    let raw = uc.cstr(p, 8).unwrap_or_default();
    let m: String = raw.iter().map(|&c| c as char).collect();
    if m.chars().any(|c| "rwa".contains(c)) {
        return m;
    }

    let w = wstr(uc, p, 8);
    if w.chars().any(|c| "rwa".contains(c)) {
        w
    } else {
        "r".into()
    }
}

pub fn file_open(uc: &mut Emu) {
    let path = wstr(uc, uc.arg(1), 260);
    let mode = open_mode(uc, uc.arg(2));
    let h = uc.get_data_mut().rt.vfs.open(&path, &mode);
    uc.ret(h);
}

pub fn file_close(uc: &mut Emu) {
    let h = uc.arg(0);
    uc.get_data_mut().rt.vfs.close(h);
    uc.ret(0);
}

pub fn file_exist(uc: &mut Emu) {
    let path = wstr(uc, uc.arg(1), 260);
    let ok = uc.get_data().rt.vfs.exists(&path);
    uc.ret(u32::from(ok));
}

pub fn file_size(uc: &mut Emu) {
    let h = uc.arg(0);
    let n = uc.get_data_mut().rt.vfs.size(h);
    uc.ret(n);
}

pub fn file_read(uc: &mut Emu) {
    let (buf, size, h) = (uc.arg(0), uc.arg(1), uc.arg(2));
    let d = uc.get_data_mut().rt.vfs.read(h, size as usize);
    if buf != 0 && !d.is_empty() {
        uc.write(buf, &d);
    }
    uc.ret(d.len() as u32);
}

pub fn file_write(uc: &mut Emu) {
    let (buf, n, h) = (uc.arg(0), uc.arg(1), uc.arg(2));
    let Ok(data) = uc.mem_read_as_vec(buf as u64, n as usize) else {
        uc.ret(0);
        return;
    };
    let k = uc.get_data_mut().rt.vfs.write(h, &data);
    uc.ret(k);
}

pub fn file_seek(uc: &mut Emu) {
    let (h, off, whence) = (uc.arg(0), uc.arg(1) as i32 as i64, uc.arg(2));
    let r = uc.get_data_mut().rt.vfs.seek(h, off, whence);
    uc.ret(r);
}

pub fn file_tell(uc: &mut Emu) {
    let h = uc.arg(0);
    let r = uc.get_data_mut().rt.vfs.tell(h);
    uc.ret(r);
}

pub fn file_mkdir(uc: &mut Emu) {
    let path = wstr(uc, uc.arg(1), 260);
    let ok = uc.get_data().rt.vfs.mkdir(&path);
    uc.ret(u32::from(ok));
}

pub fn coolbar_dir(uc: &mut Emu) {
    let out = uc.arg(0);
    let n = wwrite(uc, out, COOLBAR_DIR);
    uc.ret(n * 2);
}

pub fn sdcard(uc: &mut Emu) {
    uc.ret(1);
}

pub fn cdown_str(uc: &mut Emu) {
    let p = uc.arg(0);
    if p != 0 {
        uc.write(p, &[0]);
    }
    uc.ret(0);
}

pub fn ucs2_strlen(uc: &mut Emu) {
    let n = wstr(uc, uc.arg(0), 4096).chars().count() as u32;
    uc.ret(n);
}

pub fn ucs2_strcpy(uc: &mut Emu) {
    let s = wstr(uc, uc.arg(1), 4096);
    let d = uc.arg(0);
    wwrite(uc, d, &s);
    uc.ret(d);
}

pub fn ucs2_strcat(uc: &mut Emu) {
    let d = uc.arg(0);
    let cur = wstr(uc, d, 4096);
    let s = wstr(uc, uc.arg(1), 4096);
    let at = d + cur.chars().count() as u32 * 2;
    wwrite(uc, at, &s);
    uc.ret(d);
}

pub fn ucs2_strncmp(uc: &mut Emu) {
    let n = uc.arg(2) as usize;
    let a: String = wstr(uc, uc.arg(0), 4096).chars().take(n).collect();
    let b: String = wstr(uc, uc.arg(1), 4096).chars().take(n).collect();
    uc.ret(match a.cmp(&b) {
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
        std::cmp::Ordering::Less => 0xFFFF_FFFF,
    });
}

pub fn expand_strcpy(uc: &mut Emu) {
    let raw = uc.cstr(uc.arg(1), 512).unwrap_or_default();
    let s: String = raw.iter().map(|&c| c as char).collect();
    let d = uc.arg(0);
    wwrite(uc, d, &s);
    uc.ret(d);
}

pub fn gb2ucs2(uc: &mut Emu) {
    let raw = uc.cstr(uc.arg(0), 512).unwrap_or_default();
    let s = crate::api::text::gb18030_decode(&raw);
    let n = uc.arg(2) as usize;
    let n = if n == 0 { s.chars().count() + 1 } else { n };
    let s: String = s.chars().take(n.saturating_sub(1)).collect();
    let out = uc.arg(1);
    let k = wwrite(uc, out, &s);
    uc.ret(k);
}

pub fn ucs2_bytes(uc: &Emu, addr: u32) -> Vec<u8> {
    crate::api::text::gb18030_encode(&wstr(uc, addr, 4096))
}

pub fn ucs2_width(uc: &mut Emu) {
    let b = ucs2_bytes(uc, uc.arg(0));
    let w = match uc.get_data_mut().rt.font_data.as_mut() {
        Some(f) => f.measure(&b),
        None => 8 * b.len() as u32,
    };
    uc.ret(w);
}

pub fn draw_ucs2(uc: &mut Emu) {
    let b = ucs2_bytes(uc, uc.arg(0));
    let (x, y, c) = (uc.arg(1) as i32, uc.arg(2) as i32, uc.arg(3) as u16);
    crate::api::text::draw_bytes(uc, &b, x, y, c, 0);
    uc.ret(0);
}

fn glob_match(pat: &str, name: &str) -> bool {
    fn go(p: &[char], s: &[char]) -> bool {
        match p.first() {
            None => s.is_empty(),
            Some('*') => go(&p[1..], s) || (!s.is_empty() && go(p, &s[1..])),
            Some('?') => !s.is_empty() && go(&p[1..], &s[1..]),
            Some(c) => !s.is_empty() && s[0] == *c && go(&p[1..], &s[1..]),
        }
    }
    let p: Vec<char> = pat.to_lowercase().chars().collect();
    let s: Vec<char> = name.to_lowercase().chars().collect();
    go(&p, &s)
}

fn find_matches(uc: &Emu, path: &str, pattern: &str) -> Vec<String> {
    let root = uc.get_data().rt.vfs.resolve(path);
    let Ok(rd) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut names: Vec<String> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    let pat = if pattern.trim().is_empty() {
        "*"
    } else {
        pattern.trim()
    };
    names.into_iter().filter(|n| glob_match(pat, n)).collect()
}

pub fn find_first(uc: &mut Emu) {
    let path = wstr(uc, uc.arg(1), 260);
    let pattern = wstr(uc, uc.arg(2), 64);
    let out = uc.arg(3);
    let mut names = find_matches(uc, &path, &pattern);
    if names.is_empty() {
        uc.ret(OPEN_FAIL);
        return;
    }
    let first = names.remove(0);
    if out != 0 {
        wwrite(uc, out, &first);
    }
    let h = {
        let d = &mut uc.get_data_mut().rt;
        let h = d.next_find;
        d.next_find += 1;
        d.finds.insert(h, names);
        h
    };
    uc.ret(h);
}

pub fn find_next(uc: &mut Emu) {
    let h = uc.arg(1);
    let out = uc.arg(3);
    let next = {
        let d = &mut uc.get_data_mut().rt;
        match d.finds.get_mut(&h) {
            Some(v) if !v.is_empty() => Some(v.remove(0)),
            _ => None,
        }
    };
    match next {
        Some(n) => {
            if out != 0 {
                wwrite(uc, out, &n);
            }
            uc.ret(0);
        }
        None => uc.ret(OPEN_FAIL),
    }
}

pub fn find_close(uc: &mut Emu) {
    let h = uc.arg(0);
    uc.get_data_mut().rt.finds.remove(&h);
    uc.ret(0);
}

pub fn file_delete(uc: &mut Emu) {
    let path = wstr(uc, uc.arg(1), 260);
    let hp = uc.get_data().rt.vfs.host_path(&path);
    uc.ret(u32::from(std::fs::remove_file(hp).is_ok()));
}

pub fn strcmp(uc: &mut Emu) {
    let a = uc.cstr(uc.arg(0), 512).unwrap_or_default();
    let b = uc.cstr(uc.arg(1), 512).unwrap_or_default();
    uc.ret(match a.cmp(&b) {
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
        std::cmp::Ordering::Less => 0xFFFF_FFFF,
    });
}

pub fn sys_sleep(uc: &mut Emu) {
    let n = uc.arg(0) as u64;
    uc.get_data_mut().rt.tick += n;
    uc.ret(0);
}

pub fn delete_screen(uc: &mut Emu) {
    let s = uc.arg(0);
    uc.get_data_mut().rt.screens.retain(|&(scr, _, _)| scr != s);
    uc.ret(0);
}

pub fn ucs2gb(uc: &mut Emu) {
    let s = crate::api::text::gb18030_encode(&wstr(uc, uc.arg(0), 4096));
    let n = uc.arg(2) as usize;
    let n = if n == 0 { s.len() + 1 } else { n };
    let s = &s[..s.len().min(n.saturating_sub(1))];
    let mut v = s.to_vec();
    v.push(0);
    uc.write(uc.arg(1), &v);
    uc.ret(s.len() as u32);
}
