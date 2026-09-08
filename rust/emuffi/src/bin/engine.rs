
use std::io::{BufRead, Write};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use emucore::session::{Event, Session, Touch};

fn emit(tag: &[u8; 4], payload: &[u8], extra: &[u8]) -> std::io::Result<()> {
    let so = std::io::stdout();
    let mut o = so.lock();
    o.write_all(tag)?;
    o.write_all(extra)?;
    o.write_all(&(payload.len() as u32).to_le_bytes())?;
    o.write_all(payload)?;
    o.flush()
}

#[derive(Debug)]
enum Cmd {
    Keys(u32),
    Touch(i32, i32, Touch),
    Soft(String),
    Fps(u32),
    Quit,
}

fn field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let at = line.find(&format!("\"{key}\""))? + key.len() + 2;
    let rest = line[at..].trim_start();
    Some(rest.strip_prefix(':')?.trim_start())
}

fn parse(line: &str) -> Option<Cmd> {
    if let Some(v) = field(line, "quit") {
        if v.starts_with("true") {
            return Some(Cmd::Quit);
        }
    }
    if let Some(v) = field(line, "keys") {
        let n: String = v.chars().take_while(|c| c.is_ascii_digit()).collect();
        return n.parse().ok().map(Cmd::Keys);
    }
    if let Some(v) = field(line, "fps") {
        let n: String = v.chars().take_while(|c| c.is_ascii_digit()).collect();
        return n.parse().ok().map(Cmd::Fps);
    }
    if let Some(v) = field(line, "soft") {
        let s = v.trim_start_matches('"');
        let s = &s[..s.find('"').unwrap_or(s.len())];
        return Some(Cmd::Soft(s.to_string()));
    }
    if let Some(v) = field(line, "touch") {
        let inner = v.trim_start_matches('[');
        let inner = &inner[..inner.find(']')?];
        let mut it = inner.split(',');
        let x: i32 = it.next()?.trim().parse().ok()?;
        let y: i32 = it.next()?.trim().parse().ok()?;
        let st = it.next()?.trim().trim_matches('"');
        let t = match st {
            "up" => Touch::Up,
            "move" => Touch::Move,
            _ => Touch::Down,
        };
        return Some(Cmd::Touch(x, y, t));
    }
    None
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("用法: engine <module.cbe> [--fps 30] [--no-audio]");
        std::process::exit(2);
    }
    let fps0: u64 = args
        .iter()
        .position(|a| a == "--fps")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);

    let mut sess = match Session::open(&args[1]) {
        Ok(s) => s,
        Err(e) => {
            let _ = emit(b"LOG0", format!("打不开模块：{e}").as_bytes(), &[]);
            std::process::exit(1);
        }
    };
    sess.boot();
    let _ = emit(
        b"LOG0",
        format!("{} 已引导，screens={}", sess.name(), sess.screens()).as_bytes(),
        &[],
    );

    let (tx, rx) = mpsc::channel::<Cmd>();

    std::thread::spawn(move || {
        let si = std::io::stdin();
        for line in si.lock().lines() {
            let Ok(line) = line else { break };
            if let Some(cmd) = parse(&line) {
                let quit = matches!(cmd, Cmd::Quit);
                if tx.send(cmd).is_err() || quit {
                    return;
                }
            }
        }
        let _ = tx.send(Cmd::Quit);
    });

    let vclock = args.iter().any(|a| a == "--vclock");
    let mut fps = fps0.clamp(1, 240);
    let mut running = true;
    while running {
        let t0 = Instant::now();

        while let Ok(cmd) = rx.try_recv() {
            match cmd {
                Cmd::Quit => running = false,
                Cmd::Fps(v) => fps = v.clamp(1, 240) as u64,
                Cmd::Keys(m) => sess.set_keys(m),
                Cmd::Touch(x, y, t) => sess.set_touch(x, y, t),
                Cmd::Soft(s) => sess.soft_key(&s, true),
            }
        }
        if !running {
            break;
        }
        let px = if vclock {
            let t = sess.frame_no as f64 / fps as f64;
            sess.step_at(t)
        } else {
            sess.step()
        };
        let (w, h) = sess.size();
        let no = sess.frame_no;
        let mut bye = false;
        for e in sess.take_events() {
            let r = match &e {
                Event::Audio(_) => emit(b"AUD0", e.to_json().as_bytes(), &[]),
                Event::Log(t) => emit(b"LOG0", t.as_bytes(), &[]),
                Event::Exit => {
                    bye = true;
                    emit(b"EXT0", b"module", &[])
                }
            };
            if r.is_err() {
                return;
            }
        }
        let mut extra = Vec::with_capacity(8);
        extra.extend_from_slice(&(no as u32).to_le_bytes());
        extra.extend_from_slice(&(w as u16).to_le_bytes());
        extra.extend_from_slice(&(h as u16).to_le_bytes());
        if emit(b"FRM0", &px, &extra).is_err() {
            return;
        }
        if bye {
            break;
        }
        let dt = Duration::from_secs_f64(1.0 / fps as f64);
        if let Some(rest) = dt.checked_sub(t0.elapsed()) {
            std::thread::sleep(rest);
        }
    }

    sess.stop();
}
