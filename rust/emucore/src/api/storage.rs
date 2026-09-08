
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

use crate::machine::{Emu, Mach};

fn safe(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| if "/\\:*?\"<>|".contains(c) { '_' } else { c })
        .collect();
    let s = s.trim().to_string();
    if s.is_empty() {
        "unnamed".into()
    } else {
        s
    }
}

fn home() -> PathBuf {
    match std::env::var("NIECHE_HOME").or_else(|_| std::env::var("NICAI_HOME")) {
        Ok(v) if !v.is_empty() => PathBuf::from(v),
        _ => {
            let h = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            PathBuf::from(h).join(".nieche-emu")
        }
    }
}

fn save_path(module: &str, name: &str) -> PathBuf {
    home().join("saves").join(safe(module)).join(safe(name))
}

pub fn storage_date(uc: &mut Emu) {
    let raw = uc.cstr(uc.arg(0), 256).unwrap_or_else(|| b"save".to_vec());
    let name = String::from_utf8_lossy(&raw).to_string();
    let (buf, n) = (uc.arg(1), uc.arg(2));
    let write = uc.arg(3) == 0;
    let module = uc.get_data().rt.module_name.clone();
    let path = save_path(&module, &name);

    if write {

        if n == 0 {
            uc.ret(0);
            return;
        }
        let Ok(data) = uc.mem_read_as_vec(buf as u64, n as usize) else {
            uc.ret(0);
            return;
        };
        if let Some(d) = path.parent() {
            let _ = std::fs::create_dir_all(d);
        }

        let r = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .and_then(|mut f| {
                f.seek(SeekFrom::Start(0))?;
                f.write_all(&data)
            });
        uc.ret(u32::from(r.is_ok()));
        return;
    }

    let mut d = Vec::new();
    let ok = std::fs::File::open(&path)
        .and_then(|f| Read::take(f, n as u64).read_to_end(&mut d))
        .is_ok();

    if buf != 0 && n != 0 {
        let mut v = d.clone();
        v.resize(n as usize, 0);
        uc.write(buf, &v);
    }

    uc.ret(u32::from(ok && !d.is_empty()));
}
