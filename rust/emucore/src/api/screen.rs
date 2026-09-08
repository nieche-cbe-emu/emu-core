
use crate::machine::{Emu, Mach};
use crate::runtime;

pub fn add_screen(uc: &mut Emu) {
    let scr = uc.arg(0);
    runtime::push_screen(uc, scr, 0, 1);
    uc.ret(0);
}

pub fn add_screen_ex(uc: &mut Emu) {
    let (scr, param, flag) = (uc.arg(0), uc.arg(1), uc.arg(2));
    runtime::push_screen(uc, scr, param, flag);
    uc.ret(0);
}

pub fn screen_load_res(uc: &mut Emu) {
    let scr = uc.arg(0);
    if scr != 0 {
        let f = uc.r32(scr + 4 * runtime::S_LOADRES);
        runtime::defer(uc, f, vec![scr], "loadRes");
    }
    uc.ret(0);
}

pub fn is_key_down(uc: &mut Emu) {
    let mask = uc.arg(0);
    let bits = uc.get_data().rt.keys_down;
    uc.ret(u32::from(bits & mask != 0));
}

pub fn is_key_hold(uc: &mut Emu) {
    let mask = uc.arg(0);
    let bits = uc.get_data().rt.keys_hold;
    uc.ret(u32::from(bits & mask != 0));
}

pub fn cur_key_state(uc: &mut Emu) {
    let bits = uc.get_data().rt.keys_down;
    uc.ret(bits);
}

pub fn zero(uc: &mut Emu) {
    uc.ret(0);
}
