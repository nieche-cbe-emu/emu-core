
use emucore::{machine, Mach};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("用法: smoke <file.cbe>");
        return ExitCode::from(2);
    }
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
    println!("module {}", m.name);
    println!("endian {}", m.endian.as_str());

    let ro0 = uc.r32(uc.get_data().ro_base);
    let want = if m.endian == cbelib::Endian::Le {
        u32::from_le_bytes([m.ro[0], m.ro[1], m.ro[2], m.ro[3]])
    } else {
        u32::from_be_bytes([m.ro[0], m.ro[1], m.ro[2], m.ro[3]])
    };
    println!("ro_head {:#010x} {}", ro0, if ro0 == want { "ok" } else { "MISMATCH" });

    if m.rw.len() >= 4 {
        let rw0 = uc.r32(uc.get_data().rw_base);
        let w = if m.endian == cbelib::Endian::Le {
            u32::from_le_bytes([m.rw[0], m.rw[1], m.rw[2], m.rw[3]])
        } else {
            u32::from_be_bytes([m.rw[0], m.rw[1], m.rw[2], m.rw[3]])
        };
        println!("rw_head {:#010x} {}", rw0, if rw0 == w { "ok" } else { "MISMATCH" });
    }

    let t = machine::new_table(&mut uc, "smoke", 64);
    let mut bad = 0;
    for k in 0..64u32 {
        let p = uc.r32(t + k * 4);
        if p & 1 == 0 || p < 0x5000_0000 || p >= 0x5004_0000 {
            bad += 1;
        }
    }
    println!("table {:#x} slots_bad {bad}", t);

    let a = uc.get_data_mut().heap.alloc(0x1000, "smoke", true).unwrap();
    uc.w32(a, 0xDEAD_BEEF);
    let back = uc.r32(a);
    println!("heap {:#x} rw {}", a, if back == 0xDEAD_BEEF { "ok" } else { "MISMATCH" });

    println!("nullguard {:#x}", uc.r32(0));
    ExitCode::SUCCESS
}
