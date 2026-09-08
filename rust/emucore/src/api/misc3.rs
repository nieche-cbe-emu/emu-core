
use crate::machine::{self, Emu, Mach};
use crate::runtime;

pub const IMEI: &str = "356938035643809";
pub const APPID: u32 = 1002;

pub fn start_timer(uc: &mut Emu) {
    let (ms, cb, param) = (uc.arg(0), uc.arg(1), uc.arg(2));
    let tid = runtime::start_timer(uc, ms as u64, cb, param);
    uc.ret(tid);
}

pub fn stop_timer(uc: &mut Emu) {
    let tid = uc.arg(0);
    uc.get_data_mut().rt.timers.retain(|t| t.id != tid);
    uc.ret(0);
}

pub fn is_focus(uc: &mut Emu) {
    let s = uc.arg(0);
    let top = uc.get_data().rt.screens.last().map(|&(x, _, _)| x);
    uc.ret(u32::from(top == Some(s)));
}

pub fn get_imei(uc: &mut Emu) {
    let (buf, n) = (uc.arg(0), uc.arg(1));
    let n = if n == 0 { 26 } else { n } as usize;
    let s = &IMEI.as_bytes()[..IMEI.len().min(n.saturating_sub(1))];
    let mut v = s.to_vec();
    v.push(0);
    uc.write(buf, &v);
    uc.ret(s.len() as u32);
}

pub fn cur_appid(uc: &mut Emu) {
    uc.ret(APPID);
}

pub fn prj_version(uc: &mut Emu) {
    let p = uc.arg(0);
    if p != 0 {
        uc.write(p, b"V017\0");
    }
    uc.ret(17);
}

pub fn set_fps(uc: &mut Emu) {
    let fps = uc.arg(0);
    let fps = if fps == 0 { 25 } else { fps };
    uc.get_data_mut().rt.frame_ms = (1000 / fps.min(100)).max(1) as u64;
    uc.ret(0);
}

pub fn res_get_txt(uc: &mut Emu) {
    let cur = uc.get_data().rt.emptystr;
    let p = if cur != 0 {
        cur
    } else {
        let a = uc
            .get_data_mut()
            .heap
            .alloc(4, "emptystr", false)
            .unwrap_or(0);
        if a != 0 {
            uc.w32(a, 0);
        }
        uc.get_data_mut().rt.emptystr = a;
        a
    };
    uc.ret(p);
}

pub fn get_data_package(uc: &mut Emu) {
    let cur = uc.get_data().rt.datapackage;
    if cur != 0 {
        uc.ret(cur);
        return;
    }

    let t = machine::new_table(uc, "DF_DataPackage(auto)", 96);
    uc.get_data_mut().rt.datapackage = t;
    uc.ret(t);
}

pub fn create_image(uc: &mut Emu) {
    let rid = (uc.arg(0) & 0xFFFF) as usize;
    let out = uc.arg(1);
    let data = {
        let rt = &uc.get_data().rt;
        let a = rt.icons.as_ref().or(rt.res.as_ref());
        let pick = a.and_then(|a| a.entries.get(rid)).map(|e| e.data.clone());

        match pick {
            Some(d) => Some(d),
            None => rt
                .res
                .as_ref()
                .and_then(|a| a.entries.get(rid))
                .map(|e| e.data.clone()),
        }
    };
    let Some(d) = data else {
        uc.ret(0);
        return;
    };
    let Some(im) = cbelib::decode_image(&d) else {
        uc.ret(0);
        return;
    };
    let vt = crate::gfx::upload(uc, &im, out);
    uc.ret(vt);
}

pub const PRJ: &str = "7835";
pub const SMSC: &str = "+8613800100500";

pub fn free_space(uc: &mut Emu) {
    uc.ret(64 * 1024 * 1024);
}

pub fn get_prj(uc: &mut Emu) {
    let p = uc.arg(0);
    if p != 0 {
        let mut v = PRJ.as_bytes()[..4].to_vec();
        v.push(0);
        uc.write(p, &v);
    }
    uc.ret(4);
}

pub fn get_smsc(uc: &mut Emu) {
    let p = uc.arg(0);
    if p != 0 {
        let mut v = SMSC.as_bytes().to_vec();
        v.push(0);
        uc.write(p, &v);
    }
    uc.ret(SMSC.len() as u32);
}

pub fn write_empty(uc: &mut Emu) {
    let p = uc.arg(0);
    if p != 0 {
        uc.write(p, &[0]);
    }
    uc.ret(0);
}

pub fn audio_supported(uc: &mut Emu) {
    uc.ret(1);
}

pub fn strnicmp(uc: &mut Emu) {
    let n = uc.arg(2) as usize;
    let a: Vec<u8> = uc
        .cstr(uc.arg(0), 512)
        .unwrap_or_default()
        .into_iter()
        .take(n)
        .map(|c| c.to_ascii_lowercase())
        .collect();
    let b: Vec<u8> = uc
        .cstr(uc.arg(1), 512)
        .unwrap_or_default()
        .into_iter()
        .take(n)
        .map(|c| c.to_ascii_lowercase())
        .collect();
    uc.ret(u32::from(a != b));
}

pub fn block_malloc(uc: &mut Emu) {
    let size = uc.arg(1);
    let p = if size != 0 {
        uc.get_data_mut()
            .heap
            .alloc(size, "gblock", false)
            .unwrap_or(0)
    } else {
        0
    };
    if p != 0 {
        crate::api::fill(uc, p, 0, size);
    }
    uc.ret(p);
}

pub fn screen_notify_loadres(uc: &mut Emu) {
    let scr = uc.arg(0);
    if scr != 0 {
        let f = uc.r32(scr + 4 * runtime::S_LOADRES);
        runtime::defer(uc, f, vec![0], "screenLoadResource");
    }
    uc.ret(0);
}

pub const NETREQUEST_ERROR: u32 = 9;

fn http(uc: &mut Emu, url_ptr: u32, cb: u32, out: u32) {
    let _ = url_ptr;
    let h = {
        let d = &mut uc.get_data_mut().rt;
        d.nethandle += 1;
        d.nethandle
    };
    if out != 0 {
        uc.w32(out, h);
    }
    runtime::defer(uc, cb, vec![0, 0, 0, NETREQUEST_ERROR], "netCallBack(离线)");
    uc.ret(1);
}

pub fn get_http(uc: &mut Emu) {
    let (u, c, o) = (uc.arg(0), uc.arg(1), uc.arg(2));
    http(uc, u, c, o);
}

pub fn post_http(uc: &mut Emu) {
    let (u, c, o) = (uc.arg(0), uc.arg(3), uc.arg(4));
    http(uc, u, c, o);
}

fn res_audio(uc: &Emu, rid: u32) -> Vec<u8> {
    let ents = crate::api::dfpkg::entry_data(uc, rid as usize);
    let Some(d) = ents else { return Vec::new() };
    if d.is_empty() {
        return Vec::new();
    }
    match d[0] {
        10 if d.len() >= 5 => {
            let n = u32::from_be_bytes([d[1], d[2], d[3], d[4]]) as usize;
            d.get(5..(5 + n).min(d.len())).unwrap_or(&[]).to_vec()
        }
        2 => cbelib::unpack_entry(&d).unwrap_or_default(),
        _ => d.get(5..).unwrap_or(&[]).to_vec(),
    }
}

pub fn audio_play(uc: &mut Emu) {
    let (rid, looping) = (uc.arg(0), uc.arg(1) != 0);
    let data = res_audio(uc, rid);
    let now = uc.get_data().rt.tick;
    let name = format!("res{rid:x}");
    let r = uc
        .get_data_mut()
        .rt
        .audio
        .play_data(&data, looping, now, &name);
    uc.ret(r);
}

pub fn audio_state(uc: &mut Emu) {
    let s = uc.get_data().rt.audio.state;
    uc.ret(s);
}

pub fn audio_stop(uc: &mut Emu) {
    let a = &mut uc.get_data_mut().rt.audio;
    if a.state == crate::audio::PLAYING {
        a.events.push("{\"op\":\"stop\"}".to_string());
    }
    a.state = crate::audio::STOPPED;
    uc.ret(0);
}

pub fn audio_volume(uc: &mut Emu) {
    let v = uc.arg(0);
    uc.get_data_mut().rt.audio.volume = v;
    uc.ret(0);
}
