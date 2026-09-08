
use emucore::{machine, runtime, Mach};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("用法: bootcmp <file.cbe>");
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
    runtime::setup(&mut uc, &m);
    let style = match uc.get_data().rt.style {
        runtime::Style::New => "new",
        runtime::Style::Old => "old",
    };
    println!("module {}", m.name);
    println!("style {style}");
    let _ = runtime::boot(&mut uc);
    let d = uc.get_data();
    println!("cb0 {:#x}", d.rt.mod_cb0);
    println!("cb1 {:#x}", d.rt.mod_cb1);
    println!("managers {}", d.rt.managers.len());
    println!(
        "exit {}",
        d.exit_reason.clone().unwrap_or_else(|| "-".into())
    );

    if std::env::var("SHOW_IMPL").is_ok() {
        eprintln!(
            "  已接实现 {} 槽 / 未实现桩 {} 槽 / 注册表 {} 个名字",
            d.rt.slots_impl,
            d.rt.slots_stub,
            emucore::api::implemented()
        );
    }
    let _ = uc.pc();
    ExitCode::SUCCESS
}
