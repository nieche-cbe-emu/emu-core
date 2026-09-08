
use crate::api::fill;
use crate::machine::{Emu, Mach};
use crate::runtime;

pub fn total_seconds(uc: &mut Emu) {
    let t = uc.get_data().rt.tick / 1000;
    uc.ret(t as u32);
}

pub fn billing_paynum(uc: &mut Emu) {
    uc.ret(0);
}

pub fn main_screen_image(uc: &mut Emu) {
    let img = uc.get_data().rt.gfx.img;
    uc.ret(img);
}

pub fn kernel_ver(uc: &mut Emu) {
    uc.ret(42);
}

pub fn zero(uc: &mut Emu) {
    uc.ret(0);
}

pub fn screen_w(uc: &mut Emu) {
    let v = uc.get_data().rt.gfx.w;
    uc.ret(v);
}

pub fn screen_h(uc: &mut Emu) {
    let v = uc.get_data().rt.gfx.h;
    uc.ret(v);
}

pub fn screen_change(uc: &mut Emu) {
    let scr = uc.arg(0);
    uc.get_data_mut().rt.screens.clear();
    runtime::push_screen(uc, scr, 0, 1);
    uc.ret(0);
}

pub fn df_set_pkg(uc: &mut Emu) {
    let p = uc.arg(0);
    uc.get_data_mut().rt.datapackage = p;
    uc.ret(0);
}

pub fn init_memory_block(uc: &mut Emu) {
    let mut blk = uc.arg(0);
    let size = uc.arg(1);
    if blk == 0 {
        blk = uc
            .get_data_mut()
            .heap
            .alloc(0x18, "MEMORY_BLOCK", false)
            .unwrap_or(0);
        if blk == 0 {
            uc.ret(0);
            return;
        }
    }
    let base = if size != 0 {
        uc.get_data_mut()
            .heap
            .alloc(size, "memblock", false)
            .unwrap_or(0)
    } else {
        0
    };
    if base != 0 {
        fill(uc, base, 0, size);
    }
    uc.w32(blk, base);
    uc.w32(blk + 0x04, 0);
    uc.w32(blk + 0x08, size);
    runtime::install(uc, blk + 0x0c, "MB_Malloc", mb_malloc);
    runtime::install(uc, blk + 0x10, "MB_Reset", mb_reset);
    runtime::install(uc, blk + 0x14, "MB_Release", mb_release);
    uc.ret(blk);
}

fn mb_malloc(uc: &mut Emu) {
    let blk = uc.arg(0);
    let n = uc.arg(1);
    if blk == 0 {
        uc.ret(0);
        return;
    }
    let base = uc.r32(blk);
    let ptr = uc.r32(blk + 4);
    let total = uc.r32(blk + 8);
    let size = (n + 3) & !3;
    if ptr + size > total {
        uc.ret(0);
        return;
    }
    uc.w32(blk + 4, ptr + size);
    crate::api::fill(uc, base + ptr, 0, size);
    uc.ret(base + ptr);
}

fn mb_reset(uc: &mut Emu) {
    let blk = uc.arg(0);
    if blk != 0 {
        uc.w32(blk + 4, 0);
    }
    uc.ret(0);
}

fn mb_release(uc: &mut Emu) {
    uc.ret(0);
}

pub fn enter_win_close(uc: &mut Emu) {
    uc.get_data_mut().rt.exit_requested = true;
    uc.ret(0);
}
