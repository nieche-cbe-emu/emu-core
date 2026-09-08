
use std::ffi::{c_char, CStr};
use std::panic::{catch_unwind, AssertUnwindSafe};

use emucore::session::{Session as Inner, Touch};

pub struct Session {
    inner: Inner,
}

fn to_str<'a>(p: *const c_char) -> Option<&'a str> {
    if p.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(p) }.to_str().ok()
}

unsafe fn emit(b: &[u8], out: *mut u8, cap: usize) -> usize {
    if out.is_null() || cap < b.len() {
        return b.len();
    }
    std::ptr::copy_nonoverlapping(b.as_ptr(), out, b.len());
    b.len()
}

#[no_mangle]
pub unsafe extern "C" fn nieche_open(path: *const c_char) -> *mut Session {
    let r = catch_unwind(AssertUnwindSafe(|| {
        let p = to_str(path)?;
        let inner = Inner::open(p).ok()?;
        Some(Box::into_raw(Box::new(Session { inner })))
    }));
    match r {
        Ok(Some(p)) => p,
        _ => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn nieche_close(s: *mut Session) {
    if s.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let mut b = Box::from_raw(s);
        b.inner.stop();
        drop(b);
    }));
}

unsafe fn with<R>(s: *mut Session, f: impl FnOnce(&mut Inner) -> R, dflt: R) -> R {
    if s.is_null() {
        return dflt;
    }
    match catch_unwind(AssertUnwindSafe(|| f(&mut (*s).inner))) {
        Ok(v) => v,
        Err(_) => dflt,
    }
}

#[no_mangle]
pub unsafe extern "C" fn nieche_boot(s: *mut Session) -> i32 {
    with(s, |ss| i32::from(ss.boot()), 0)
}

#[no_mangle]
pub unsafe extern "C" fn nieche_stop(s: *mut Session) {
    with(s, |ss| ss.stop(), ())
}

#[no_mangle]
pub unsafe extern "C" fn nieche_size(s: *mut Session, w: *mut u32, h: *mut u32) {
    with(
        s,
        |ss| {
            let (gw, gh) = ss.size();
            if !w.is_null() {
                *w = gw;
            }
            if !h.is_null() {
                *h = gh;
            }
        },
        (),
    )
}

#[no_mangle]
pub unsafe extern "C" fn nieche_step(s: *mut Session, out: *mut u8, cap: usize) -> usize {
    with(s, |ss| emit(&ss.step(), out, cap), 0)
}

#[no_mangle]
pub unsafe extern "C" fn nieche_step_at(
    s: *mut Session,
    now: f64,
    out: *mut u8,
    cap: usize,
) -> usize {
    with(s, |ss| emit(&ss.step_at(now), out, cap), 0)
}

#[no_mangle]
pub unsafe extern "C" fn nieche_frame_no(s: *mut Session) -> u64 {
    with(s, |ss| ss.frame_no, 0)
}

#[no_mangle]
pub unsafe extern "C" fn nieche_set_keys(s: *mut Session, mask: u32) {
    with(s, |ss| ss.set_keys(mask), ())
}

#[no_mangle]
pub unsafe extern "C" fn nieche_set_touch(s: *mut Session, x: i32, y: i32, state: i32) {
    with(s, |ss| ss.set_touch(x, y, Touch::from_i32(state)), ())
}

#[no_mangle]
pub unsafe extern "C" fn nieche_soft_key(s: *mut Session, side: i32) {
    with(
        s,
        |ss| ss.soft_key(if side == 0 { "left" } else { "right" }, true),
        (),
    )
}

#[no_mangle]
pub unsafe extern "C" fn nieche_nonblank(s: *mut Session) -> u32 {
    with(s, |ss| ss.nonblank(), 0)
}

#[no_mangle]
pub unsafe extern "C" fn nieche_screens(s: *mut Session) -> u32 {
    with(s, |ss| ss.screens(), 0)
}

#[no_mangle]
pub unsafe extern "C" fn nieche_name(s: *mut Session, out: *mut u8, cap: usize) -> usize {
    with(s, |ss| emit(ss.name().as_bytes(), out, cap), 0)
}

#[no_mangle]
pub unsafe extern "C" fn nieche_take_events(s: *mut Session, out: *mut u8, cap: usize) -> usize {
    with(
        s,
        |ss| {

            let evs = ss.take_events();
            let joined = evs
                .iter()
                .map(|e| e.to_json())
                .collect::<Vec<_>>()
                .join("\n");
            let b = joined.as_bytes();
            if out.is_null() || cap < b.len() {
                ss.put_back_events(evs);
                return b.len();
            }
            std::ptr::copy_nonoverlapping(b.as_ptr(), out, b.len());
            b.len()
        },
        0,
    )
}

#[no_mangle]
pub unsafe extern "C" fn nieche_take_logs(s: *mut Session, out: *mut u8, cap: usize) -> usize {
    with(
        s,
        |ss| {

            let joined = ss.peek_logs();
            let b = joined.as_bytes();
            if out.is_null() || cap < b.len() {
                return b.len();
            }
            std::ptr::copy_nonoverlapping(b.as_ptr(), out, b.len());
            ss.take_logs();
            b.len()
        },
        0,
    )
}

#[no_mangle]
pub extern "C" fn nieche_abi_version() -> u32 {
    2
}
