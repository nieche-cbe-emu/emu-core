
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("用法: layout <file.cbe>");
        return ExitCode::from(2);
    }
    let m = match cbelib::load(&args[1]) {
        Ok(m) => m,
        Err(e) => {
            println!("ERROR {e}");
            return ExitCode::from(1);
        }
    };
    let p = emucore::place(&m);
    println!("module {}", m.name);
    println!("endian {}", m.endian.as_str());
    println!("ro_base {:#x}", p.ro_base);
    println!("ro_size {:#x}", p.ro_size);
    println!("rw_base {:#x}", p.rw_base);
    println!("rw_size {:#x}", p.rw_size);
    println!("ro_len {:#x}", m.ro.len());
    println!("rw_len {:#x}", m.rw.len());
    ExitCode::SUCCESS
}
