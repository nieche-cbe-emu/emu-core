
use crate::machine::{Emu, Mach};

pub fn one(uc: &mut Emu) {
    uc.ret(1);
}

pub fn zero(uc: &mut Emu) {
    uc.ret(0);
}

pub fn memcmp(uc: &mut Emu) {
    let n = uc.arg(2) as usize;
    if n == 0 {
        uc.ret(0);
        return;
    }
    let a = uc.mem_read_as_vec(uc.arg(0) as u64, n).unwrap_or_default();
    let b = uc.mem_read_as_vec(uc.arg(1) as u64, n).unwrap_or_default();
    uc.ret(match a.cmp(&b) {
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
        std::cmp::Ordering::Less => 0xFFFF_FFFF,
    });
}

pub fn pointer_x(uc: &mut Emu) {
    let v = uc.get_data().rt.pointer.0;
    uc.ret(v as u32);
}

pub fn pointer_y(uc: &mut Emu) {
    let v = uc.get_data().rt.pointer.1;
    uc.ret(v as u32);
}

fn s32(v: u32) -> i32 {
    v as i32
}

pub const FIX: f64 = 4096.0;

pub fn sin(uc: &mut Emu) {
    let d = s32(uc.arg(0)).rem_euclid(360) as f64;
    let v = (d.to_radians().sin() * FIX).round() as i64;
    uc.ret(v as u32);
}

pub fn cos(uc: &mut Emu) {
    let d = s32(uc.arg(0)).rem_euclid(360) as f64;
    let v = (d.to_radians().cos() * FIX).round() as i64;
    uc.ret(v as u32);
}

pub fn sqrt(uc: &mut Emu) {
    let v = s32(uc.arg(0));
    uc.ret(if v > 0 { (v as f64).sqrt() as u32 } else { 0 });
}

pub fn pointer_down(uc: &mut Emu) {
    let v = uc.get_data().rt.touch_down;
    uc.ret(v);
}

pub fn pointer_up(uc: &mut Emu) {
    let v = uc.get_data().rt.touch_up;
    uc.ret(v);
}

pub fn pointer_hold(uc: &mut Emu) {
    let v = uc.get_data().rt.touch_hold;
    uc.ret(v);
}

pub fn pointer_drag(uc: &mut Emu) {
    let v = uc.get_data().rt.touch_drag;
    uc.ret(v);
}
