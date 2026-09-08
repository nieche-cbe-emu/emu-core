
use crate::machine::{Emu, Mach};
use crate::runtime;

use super::fileio::{wstr, COOLBAR_DIR};

fn s32(v: u32) -> i32 {
    v as i32
}

pub fn cd_rect_point(uc: &mut Emu) {
    let (x1, y1, x2, y2) = (s32(uc.arg(0)), s32(uc.arg(1)), s32(uc.arg(2)), s32(uc.arg(3)));
    let (px, py) = (s32(uc.arg(4)), s32(uc.arg(5)));
    uc.ret(u32::from(x1 <= px && px <= x2 && y1 <= py && py <= y2));
}

pub fn cd_rect(uc: &mut Emu) {
    let a: Vec<i32> = (0..4).map(|i| s32(uc.arg(i))).collect();
    let b: Vec<i32> = (4..8).map(|i| s32(uc.arg(i))).collect();
    let hit = a[0] <= b[2] && b[0] <= a[2] && a[1] <= b[3] && b[1] <= a[3];
    uc.ret(u32::from(hit));
}

pub fn cd_rect_point2(uc: &mut Emu) {
    let (x, y, w, h) = (s32(uc.arg(0)), s32(uc.arg(1)), s32(uc.arg(2)), s32(uc.arg(3)));
    let (px, py) = (s32(uc.arg(4)), s32(uc.arg(5)));
    uc.ret(u32::from(x <= px && px <= x + w && y <= py && py <= y + h));
}

pub fn cd_rect2(uc: &mut Emu) {
    let (ax, ay, aw, ah) = (s32(uc.arg(0)), s32(uc.arg(1)), s32(uc.arg(2)), s32(uc.arg(3)));
    let (bx, by, bw, bh) = (s32(uc.arg(4)), s32(uc.arg(5)), s32(uc.arg(6)), s32(uc.arg(7)));
    let hit = ax <= bx + bw && bx <= ax + aw && ay <= by + bh && by <= ay + ah;
    uc.ret(u32::from(hit));
}

pub fn df_degree(uc: &mut Emu) {
    let (dx, dy) = (s32(uc.arg(0)) as f64, s32(uc.arg(1)) as f64);
    let d = dy.atan2(dx).to_degrees().round() as i64;
    uc.ret(d.rem_euclid(360) as u32);
}

pub fn is_key_up(uc: &mut Emu) {
    let mask = uc.arg(0);
    let bits = uc.get_data().rt.keys_up;
    uc.ret(u32::from(bits & mask != 0));
}

pub fn change_screen(uc: &mut Emu) {
    let scr = uc.arg(0);
    uc.get_data_mut().rt.screens.clear();
    runtime::push_screen(uc, scr, 0, 1);
    uc.ret(0);
}

pub fn change_screen_ex(uc: &mut Emu) {
    let (scr, param, flag) = (uc.arg(0), uc.arg(1), uc.arg(2));
    uc.get_data_mut().rt.screens.clear();
    runtime::push_screen(uc, scr, param, flag);
    uc.ret(0);
}

pub fn is_bottom_screen(uc: &mut Emu) {
    let scr = uc.arg(0);
    let r = uc
        .get_data()
        .rt
        .screens
        .first()
        .is_some_and(|&(s, _, _)| s == scr);
    uc.ret(u32::from(r));
}

pub fn get_lcd_buffer(uc: &mut Emu) {
    let v = uc.get_data().rt.gfx.buf;
    uc.ret(v);
}

pub fn strncmp(uc: &mut Emu) {
    let n = uc.arg(2) as usize;
    let a = uc.cstr(uc.arg(0), 4096).unwrap_or_default();
    let b = uc.cstr(uc.arg(1), 4096).unwrap_or_default();
    let (a, b) = (&a[..n.min(a.len())], &b[..n.min(b.len())]);
    uc.ret(if a == b {
        0
    } else if a > b {
        1
    } else {
        0xFFFF_FFFF
    });
}

pub fn stricmp(uc: &mut Emu) {
    let a = uc.cstr(uc.arg(0), 4096).unwrap_or_default().to_ascii_lowercase();
    let b = uc.cstr(uc.arg(1), 4096).unwrap_or_default().to_ascii_lowercase();
    uc.ret(u32::from(a != b));
}

pub fn df_string_equal(uc: &mut Emu) {
    let a = uc.cstr(uc.arg(0), 4096).unwrap_or_default();
    let b = uc.cstr(uc.arg(1), 4096).unwrap_or_default();
    uc.ret(u32::from(a == b));
}

pub fn ucs2_strcmp(uc: &mut Emu) {
    let a = wstr(uc, uc.arg(0), 4096);
    let b = wstr(uc, uc.arg(1), 4096);
    uc.ret(u32::from(a != b));
}

pub fn coolbar_full_path(uc: &mut Emu) {
    let (out, name) = (uc.arg(0), uc.arg(1));
    let s = format!("{COOLBAR_DIR}{}", wstr(uc, name, 4096));
    super::fileio::wwrite(uc, out, &s);
    uc.ret(out);
}

pub fn open_channel(uc: &mut Emu) {
    let cb = uc.arg(2);
    runtime::defer(
        uc,
        cb,
        vec![0, 0, 0, NET_OPENCHANNEL_ERROR],
        "netOpenChannel(离线)",
    );
    uc.ret(0);
}

pub const NET_OPENCHANNEL_ERROR: u32 = 6;

pub fn close_channel(uc: &mut Emu) {
    uc.ret(1);
}

pub fn audio_pause(uc: &mut Emu) {
    let a = &mut uc.get_data_mut().rt.audio;
    if a.state == crate::audio::PLAYING {
        a.state = crate::audio::PAUSED;
    }
    uc.ret(1);
}

pub fn audio_resume(uc: &mut Emu) {
    let a = &mut uc.get_data_mut().rt.audio;
    if a.state == crate::audio::PAUSED {
        a.state = crate::audio::PLAYING;
    }
    uc.ret(1);
}

pub fn audio_play_by_data(uc: &mut Emu) {
    let p = uc.arg(0);
    if p == 0 {
        uc.ret(0);
        return;
    }
    let looping = uc.arg(1) != 0;
    let hdr = uc.mem_read_as_vec(p as u64, 5).unwrap_or_default();
    if hdr.len() < 5 {
        uc.ret(0);
        return;
    }
    let mut n = u32::from_be_bytes([hdr[1], hdr[2], hdr[3], hdr[4]]) as usize;
    if n == 0 || n > (4 << 20) {
        n = 4096;
    }
    let data = uc.mem_read_as_vec((p + 5) as u64, n).unwrap_or_default();
    let now = uc.get_data().rt.tick;
    let name = format!("data{p:x}");
    let r = uc
        .get_data_mut()
        .rt
        .audio
        .play_data(&data, looping, now, &name);
    uc.ret(r);
}

pub fn zero(uc: &mut Emu) {
    uc.ret(0);
}

pub fn df_get_data_package(uc: &mut Emu) {
    let mut pkg = uc.get_data().rt.datapackage;
    if pkg == 0 {
        pkg = crate::machine::new_table(uc, "DF_DataPackage(auto)", 96);
        uc.get_data_mut().rt.datapackage = pkg;
    }
    uc.ret(pkg);
}
