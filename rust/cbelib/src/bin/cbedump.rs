
use std::process::ExitCode;

fn dump_archive(tag: &str, a: &cbelib::ResArchive) {
    println!(
        "{tag} count={} data_off={:#x} data_size={:#x} base={:#x} size={:#x}",
        a.count, a.data_off, a.data_size, a.base, a.size
    );
    for (i, e) in a.entries.iter().enumerate() {

        let un = match cbelib::unpack_entry(&e.data) {
            Some(v) => format!(" lz={:08x}:{}", cbelib::crc32(&v), v.len()),
            None => String::new(),
        };

        let img = match cbelib::decode_image(&e.data) {
            Some(im) => {
                let mut px = Vec::with_capacity(im.rgb565.len() * 2);
                for v in &im.rgb565 {
                    px.extend_from_slice(&v.to_le_bytes());
                }
                let tr = match im.transparent {
                    Some(t) => format!("{t}"),
                    None => "-".to_string(),
                };
                let al = match &im.alpha {
                    Some(a) => format!("{:08x}", cbelib::crc32(a)),
                    None => "-".to_string(),
                };
                format!(
                    " img={}x{}:{:08x}:tr{}:a{}",
                    im.width,
                    im.height,
                    cbelib::crc32(&px),
                    tr,
                    al
                )
            }
            None => String::new(),
        };
        println!(
            "E {i} {} {:#x} {:#x} {:08x}{un}{img}",
            e.name,
            e.off,
            e.size,
            cbelib::crc32(&e.data)
        );
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("用法: cbedump <file.cbe>");
        return ExitCode::from(2);
    }
    let m = match cbelib::load(&args[1]) {
        Ok(m) => m,
        Err(e) => {
            println!("ERROR {e}");
            return ExitCode::from(1);
        }
    };
    println!("module {}", m.name);
    println!("endian {}", m.endian.as_str());
    println!("load_base {:#x}", m.load_base);
    println!("image_size {:#x}", m.image_size);
    println!("image_end {:#x}", m.image_end);
    println!("rw_size {:#x}", m.rw_size);
    println!(
        "ro off={:#x} size={:#x} chk={:#x} crc={:08x}",
        m.ro_off,
        m.ro.len(),
        m.ro_chk,
        cbelib::crc32(&m.ro)
    );
    println!(
        "rw off={:#x} size={:#x} chk={:#x} crc={:08x}",
        m.rw_off,
        m.rw.len(),
        m.rw_chk,
        cbelib::crc32(&m.rw)
    );
    match &m.icons {
        Some(a) => dump_archive("icons", a),
        None => println!("icons none"),
    }
    match &m.res {
        Some(a) => dump_archive("res", a),
        None => println!("res none"),
    }
    println!("packages {}", m.packages.len());
    for (nm, a) in &m.packages {
        dump_archive(&format!("pkg[{nm}]"), a);
    }
    ExitCode::SUCCESS
}
