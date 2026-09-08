
use emucore::{gfx, machine, runtime};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("用法: framecmp <file.cbe> [帧数]");
        return ExitCode::from(2);
    }
    let frames: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(300);
    let m = match cbelib::load(&args[1]) {
        Ok(m) => m,
        Err(e) => {
            println!("ERROR load {e}");
            return ExitCode::from(1);
        }
    };
    let mut uc = match machine::build(&m) {
        Ok(u) => u,
        Err(e) => {
            println!("ERROR build {e}");
            return ExitCode::from(1);
        }
    };
    runtime::setup(&mut uc, &m);
    runtime::boot(&mut uc);

    uc.get_data_mut().log_calls = true;
    uc.get_data_mut().call_log_cap = 1 << 20;
    uc.get_data_mut().call_log.clear();
    runtime::app_start(&mut uc);

    if let Ok(path) = std::env::var("MEMDUMP") {
        let (hb, hn) = {
            let d = uc.get_data();
            (d.heap.base, d.heap.used())
        };
        if let Ok(v) = uc.mem_read_as_vec(hb as u64, hn as usize) {
            let _ = std::fs::write(&path, v);
        }
        eprintln!("HEAP\t{hb:#x}\t{hn}");
    }
    if std::env::var("TRACE").as_deref() == Ok("-1") {
        for n in uc.get_data().call_log.iter() {
            eprintln!("{n}");
        }
    }

    if std::env::var("DIAG").is_ok() {
        let d = uc.get_data();
        eprintln!(
            "  cb0={:#x} 屏幕栈={} 待办={} 退出={:?}",
            d.rt.mod_cb0,
            d.rt.screens.len(),
            d.rt.pending.len(),
            d.exit_reason
        );
        let mut v: Vec<_> = d.rt.unimpl.iter().collect();
        v.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
        for ((tag, off), n) in v.iter().take(12) {
            let nm = emucore::vmspec::field(tag, *off)
                .map(|f| f.name)
                .unwrap_or("?");
            eprintln!("  未实现 {tag}+{off:#05x} {nm}  x{n}");
        }
        let mut t: Vec<_> = d.trap_hits.iter().collect();
        t.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
        for (idx, n) in t.iter().take(8) {
            eprintln!("  热点 {}  x{n}", d.slots[**idx as usize].name);
        }
    }

    let mut rows: Vec<String> = Vec::with_capacity(frames);
    for i in 0..frames {
        uc.get_data_mut().call_log.clear();

        if i % 37 == 0 {
            runtime::press(&mut uc, 1 << ((i / 37) % 12));
        } else if i % 37 == 4 {
            runtime::release_all(&mut uc);
        }
        runtime::frame(&mut uc, runtime::NO_EVENT, 0);

        if std::env::var("TRACE").ok().and_then(|v| v.parse::<usize>().ok()) == Some(i) {
            for n in uc.get_data().call_log.iter() {
                eprintln!("{n}");
            }
        }

        if let Ok(spec) = std::env::var("FBDUMP") {
            if let Some((f, path)) = spec.split_once(':') {
                if f.parse::<usize>().ok() == Some(i) {
                    let _ = std::fs::write(path, gfx::raw565(&uc));
                }
            }
        }

        if let Ok(spec) = std::env::var("RWDUMP") {
            if let Some((f, path)) = spec.split_once(':') {
                if f.parse::<usize>().ok() == Some(i) {
                    let (b, n) = {
                        let d = uc.get_data();
                        (d.rw_base, d.rw_size)
                    };
                    if let Ok(v) = uc.mem_read_as_vec(b as u64, n as usize) {
                        let _ = std::fs::write(path, v);
                    }

                    let (hb, hn) = {
                        let d = uc.get_data();
                        (d.heap.base, d.heap.used())
                    };
                    if let Ok(v) = uc.mem_read_as_vec(hb as u64, hn as usize) {
                        let _ = std::fs::write(format!("{path}.heap"), v);
                    }

                    if let Ok(v) = uc.mem_read_as_vec(0x300f_0000, 0x10000) {
                        let _ = std::fs::write(format!("{path}.stack"), v);
                    }
                    eprintln!("HEAPN\t{hn}");
                }
            }
        }
        let fb = cbelib::crc32(&gfx::raw565(&uc));
        let names: Vec<&str> = uc.get_data().call_log.iter().map(|s| s.as_str()).collect();
        let cl = cbelib::crc32(names.join("\n").as_bytes());
        rows.push(format!("{i} {fb:08x} {cl:08x} {}", names.len()));
    }
    if std::env::var("UNIMPL").is_ok() {
        let d = uc.get_data();
        let mut v: Vec<_> = d.rt.unimpl.iter().collect();
        v.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
        for ((tag, off), n) in v.iter().take(40) {
            let nm = emucore::vmspec::field(tag, *off).map(|f| f.name).unwrap_or("?");
            eprintln!("UNIMPL\t{nm}\t{n}");
        }
    }
    let (w, h) = (uc.get_data().rt.gfx.w, uc.get_data().rt.gfx.h);
    println!("# nieche-baseline 1");
    println!("module {}", m.name);
    println!("endian {}", m.endian.as_str());
    println!("screen {w} {h}");
    println!("input default-v1");
    println!("frames {frames}");
    println!("# idx fb call ncalls");
    for r in rows {
        println!("{r}");
    }
    ExitCode::SUCCESS
}
