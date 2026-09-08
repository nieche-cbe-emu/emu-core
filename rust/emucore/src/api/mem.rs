
use crate::api::fill;
use crate::machine::{Emu, Mach};

pub fn df_malloc_in(uc: &mut Emu) {
    let (pp, size) = (uc.arg(0), uc.arg(1));
    let p = if size != 0 {
        uc.get_data_mut()
            .heap
            .alloc(size, "dF_Malloc", false)
            .unwrap_or(0)
    } else {
        0
    };
    if p != 0 {
        fill(uc, p, 0, size);
    }
    uc.w32(pp, p);
    uc.ret(if p != 0 { 1 } else { 0 });
}

pub fn df_free(uc: &mut Emu) {
    let pp = uc.arg(0);
    if pp != 0 {
        let p = uc.r32(pp);
        uc.get_data_mut().heap.free(p);
        uc.w32(pp, 0);
    }
    uc.ret(0);
}

pub fn malloc_big(uc: &mut Emu) {
    let size = uc.arg(0);
    let p = if size != 0 {
        uc.get_data_mut()
            .heap
            .alloc(size, "bigmem", false)
            .unwrap_or(0)
    } else {
        0
    };
    if p != 0 {
        fill(uc, p, 0, size);
    }
    uc.ret(p);
}

pub fn free_big(uc: &mut Emu) {
    let p = uc.arg(0);
    uc.get_data_mut().heap.free(p);
    uc.ret(0);
}

pub fn remalloc_big(uc: &mut Emu) {
    let (old, size) = (uc.arg(0), uc.arg(1));
    let p = uc
        .get_data_mut()
        .heap
        .alloc(size, "bigmem", false)
        .unwrap_or(0);
    if old != 0 {
        let n = uc.get_data().heap.block_size(old).unwrap_or(0).min(size);
        if n != 0 && p != 0 {
            if let Ok(b) = uc.mem_read_as_vec(old as u64, n as usize) {
                uc.write(p, &b);
            }
        }
        uc.get_data_mut().heap.free(old);
    }
    uc.ret(p);
}

pub fn get_gblock(uc: &mut Emu) {
    let cur = uc.get_data().rt.gblock;
    let p = if cur != 0 {
        cur
    } else {
        let a = uc
            .get_data_mut()
            .heap
            .alloc(0x20, "MemoryBlock", false)
            .unwrap_or(0);
        uc.get_data_mut().rt.gblock = a;
        a
    };
    uc.ret(p);
}

pub fn gblock_malloc(uc: &mut Emu) {
    let size = uc.arg(0);
    let p = if size != 0 {
        uc.get_data_mut()
            .heap
            .alloc(size, "gblock", false)
            .unwrap_or(0)
    } else {
        0
    };
    if p != 0 {
        fill(uc, p, 0, size);
    }
    uc.ret(p);
}

pub fn noop(uc: &mut Emu) {
    uc.ret(0);
}
