
pub const STOPPED: u32 = 0;
pub const PLAYING: u32 = 1;
pub const PAUSED: u32 = 2;

#[derive(Debug, Default)]
pub struct Audio {
    pub state: u32,
    pub volume: u32,
    pub ends_at: u64,
    pub looping: bool,

    pub outdir: Option<std::path::PathBuf>,

    pub dumped: std::collections::HashMap<(String, usize), String>,

    pub events: Vec<String>,
}

const MAGIC: &[(&[u8], &str)] = &[
    (b"MThd", "mid"),
    (b"RIFF", "wav"),
    (b"#!AMR", "amr"),
    (b"ID3", "mp3"),
    (b"\xff\xfb", "mp3"),
    (b"\xff\xf3", "mp3"),
    (b"\xff\xe3", "mp3"),
    (b"\xff\xf2", "mp3"),
];

pub fn sniff(d: &[u8]) -> &'static str {
    for (m, ext) in MAGIC {
        if d.starts_with(m) {
            return ext;
        }
    }
    "bin"
}

pub fn midi_duration_ms(d: &[u8]) -> u64 {
    if !d.starts_with(b"MThd") || d.len() < 14 {
        return 1000;
    }
    let div = i16::from_be_bytes([d[12], d[13]]) as i64;
    let mut ticks_total: u64 = 0;
    let mut tempo: u64 = 500_000;
    let mut o = 14usize;
    while o + 8 <= d.len() {
        let tag = &d[o..o + 4];
        let ln = u32::from_be_bytes([d[o + 4], d[o + 5], d[o + 6], d[o + 7]]) as usize;
        let end = (o + 8 + ln).min(d.len());
        let body = &d[(o + 8).min(d.len())..end];
        if tag == b"MTrk" {
            let (mut t, mut p, mut run) = (0u64, 0usize, 0u8);
            while p < body.len() {
                let mut dt: u64 = 0;
                while p < body.len() {
                    let b = body[p];
                    p += 1;
                    dt = (dt << 7) | (b & 0x7F) as u64;
                    if b & 0x80 == 0 {
                        break;
                    }
                }
                t += dt;
                if p >= body.len() {
                    break;
                }
                let mut st = body[p];
                if st < 0x80 {
                    st = run;
                } else {
                    p += 1;
                    run = st;
                }
                if st == 0xFF {
                    if p >= body.len() {
                        break;
                    }
                    let mt = body[p];
                    p += 1;
                    let mut ln2: usize = 0;
                    while p < body.len() {
                        let b = body[p];
                        p += 1;
                        ln2 = (ln2 << 7) | (b & 0x7F) as usize;
                        if b & 0x80 == 0 {
                            break;
                        }
                    }
                    if mt == 0x51 && ln2 == 3 && p + 3 <= body.len() {
                        tempo = ((body[p] as u64) << 16)
                            | ((body[p + 1] as u64) << 8)
                            | body[p + 2] as u64;
                    }
                    p += ln2;
                } else if matches!(st & 0xF0, 0xC0 | 0xD0) {
                    p += 1;
                } else if st & 0xF0 == 0xF0 {

                } else {
                    p += 2;
                }
            }
            ticks_total = ticks_total.max(t);
        }
        o += 8 + ln;
    }
    if div > 0 && ticks_total > 0 {
        return ticks_total * tempo / div as u64 / 1000;
    }
    1000
}

impl Audio {

    fn dump(&mut self, data: &[u8], name: &str) -> Option<String> {
        let dir = self.outdir.clone()?;
        let key = (name.to_string(), data.len());
        if let Some(p) = self.dumped.get(&key) {
            return Some(p.clone());
        }
        std::fs::create_dir_all(&dir).ok()?;
        let ext = sniff(data);
        let base = name
            .to_lowercase()
            .ends_with(&format!(".{ext}"))
            .then(|| &name[..name.len() - ext.len() - 1])
            .unwrap_or(name);

        let safe: String = base
            .chars()
            .map(|c| if c == '/' || c == '\\' { '_' } else { c })
            .collect();
        let path = dir.join(format!("{safe}.{ext}"));
        std::fs::write(&path, data).ok()?;
        let p = path.to_string_lossy().to_string();
        self.dumped.insert(key, p.clone());
        Some(p)
    }

    pub fn play_data(&mut self, data: &[u8], looping: bool, now: u64, name: &str) -> u32 {
        if data.is_empty() {
            self.state = STOPPED;
            return 0;
        }
        let dur = match sniff(data) {
            "mid" => midi_duration_ms(data),

            "mp3" => (data.len() as u64 * 8 / 32).max(500),
            _ => (data.len() as u64 / 8).max(500),
        };
        self.looping = looping;
        self.state = PLAYING;
        self.ends_at = now + dur;
        let kind = sniff(data);
        let path = self.dump(data, name);
        self.events.push(format!(
            "{{\"op\":\"play\",\"path\":{},\"loop\":{},\"kind\":\"{}\",\"name\":{}}}",
            match &path {
                Some(p) => crate::session::json_str(p),
                None => "null".to_string(),
            },
            looping,
            kind,
            crate::session::json_str(name)
        ));
        1
    }

    pub fn tick(&mut self, now: u64) {
        if self.state == PLAYING && now >= self.ends_at {
            if self.looping {
                let d = self.ends_at.saturating_sub(now);
                self.ends_at = now + if d == 0 { 1000 } else { d };
            } else {
                self.state = STOPPED;
                self.events.push("{\"op\":\"stop\"}".to_string());
            }
        }
    }
}
