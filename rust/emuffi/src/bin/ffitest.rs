
use std::ffi::CString;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("用法: ffitest <file.cbe> [帧数]");
        return ExitCode::from(2);
    }
    let frames: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(60);
    const FPS_DT: f64 = 0.04;
    let path = CString::new(args[1].clone()).unwrap();

    unsafe {
        assert_eq!(nieche::nieche_abi_version(), 2, "ABI 版本不匹配");
        let s = nieche::nieche_open(path.as_ptr());
        if s.is_null() {
            println!("ERROR open");
            return ExitCode::from(1);
        }
        if nieche::nieche_boot(s) != 1 {
            println!("ERROR boot");
            return ExitCode::from(1);
        }
        let mut rows: Vec<String> = Vec::with_capacity(frames);
        let mut buf = vec![0u8; 320 * 480 * 2];
        let (mut w, mut h) = (0u32, 0u32);
        for i in 0..frames {

            nieche::nieche_size(s, &mut w, &mut h);
            if i % 37 == 0 {
                nieche::nieche_set_keys(s, 1 << ((i / 37) % 12));
            } else if i % 37 == 4 {
                nieche::nieche_set_keys(s, 0);
            }
            if i % 53 == 7 {
                nieche::nieche_set_touch(s, (w / 2) as i32, (h / 2) as i32, 0);
            } else if i % 53 == 9 {
                nieche::nieche_set_touch(s, (w / 2) as i32, (h / 2) as i32, 1);
            }
            let n = nieche::nieche_step_at(s, i as f64 * FPS_DT, buf.as_mut_ptr(), buf.len());
            rows.push(format!("{i} {:08x}", cbelib::crc32(&buf[..n])));
        }
        nieche::nieche_size(s, &mut w, &mut h);
        println!("screen {w} {h}");
        println!("screens {}", nieche::nieche_screens(s));
        println!("nonblank {}", nieche::nieche_nonblank(s));
        for r in rows {
            println!("{r}");
        }
        nieche::nieche_close(s);

        nieche::nieche_size(std::ptr::null_mut(), &mut w, &mut h);
        assert_eq!(nieche::nieche_boot(std::ptr::null_mut()), 0);
        assert_eq!(
            nieche::nieche_step(std::ptr::null_mut(), std::ptr::null_mut(), 0),
            0
        );
        assert_eq!(nieche::nieche_nonblank(std::ptr::null_mut()), 0);
        nieche::nieche_close(std::ptr::null_mut());
        eprintln!("空句柄检查通过");
    }
    ExitCode::SUCCESS
}
