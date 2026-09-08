
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

pub const OPEN_FAIL: u32 = 0xFFFF_FFFF;

pub const GLUE_PREFIXES: &[&str] = &[
    "dfwsms",
    "dfwmix",
    "wpay",
    "cdlist",
    "cwstorecfg",
    "wstore_host",
    "coolbar_list",
];

pub struct Vfs {
    pub root: PathBuf,
    pub base: Option<PathBuf>,
    pub handles: HashMap<u32, File>,
    next_h: u32,
}

impl Vfs {
    pub fn new(root: PathBuf, base: Option<PathBuf>) -> Vfs {
        let _ = std::fs::create_dir_all(&root);
        let base = base.filter(|b| b.is_dir());
        Vfs {
            root,
            base,
            handles: HashMap::new(),
            next_h: 1,
        }
    }

    fn norm(path: &str) -> String {
        let p = path.replace('\\', "/");
        let p = p.trim_start_matches('/');

        let b = p.as_bytes();
        if b.len() > 1 && b[1] == b':' {
            p[2..].trim_start_matches('/').to_string()
        } else {
            p.to_string()
        }
    }

    pub fn host_path(&self, path: &str) -> PathBuf {
        self.root.join(Self::norm(path))
    }

    pub fn resolve(&self, path: &str) -> PathBuf {
        let rel = Self::norm(path);
        let over = self.root.join(&rel);
        if over.exists() {
            return over;
        }
        if let Some(b) = &self.base {
            let p = b.join(&rel);
            if p.exists() {
                return p;
            }
        }
        over
    }

    pub fn is_glue_file(path: &str) -> bool {
        let n = Self::norm(path);
        let base = n.rsplit('/').next().unwrap_or("").to_lowercase();
        GLUE_PREFIXES.iter().any(|p| base.starts_with(p))
    }

    pub fn open(&mut self, path: &str, mode: &str) -> u32 {
        let m = mode.to_lowercase();
        let m = if m.chars().any(|c| "rwa+".contains(c)) {
            m
        } else {

            "r".to_string()
        };
        let writing = m.contains('w') || m.contains('a') || m.contains('+');
        let hp = if writing {
            self.host_path(path)
        } else {
            self.resolve(path)
        };

        if path.is_empty() || path.ends_with('\\') || path.ends_with('/') {
            return OPEN_FAIL;
        }
        let create = m.contains('w') || m.contains('a');
        if !hp.exists() {
            if !create && Self::is_glue_file(path) {
                mkparent(&hp);
                let _ = File::create(&hp);
            } else if !create {
                return OPEN_FAIL;
            } else {
                mkparent(&hp);
                let _ = File::create(&hp);
            }
        }
        let mut o = OpenOptions::new();
        let r = if m.contains('w') {

            o.read(true).write(true).truncate(true).create(true).open(&hp)
        } else if m.contains('a') {
            o.read(true).append(true).create(true).open(&hp)
        } else if m.contains('+') {
            o.read(true).write(true).open(&hp)
        } else {
            o.read(true).open(&hp)
        };
        let Ok(f) = r else { return OPEN_FAIL };
        let h = self.next_h;
        self.next_h += 1;
        self.handles.insert(h, f);
        h
    }

    pub fn close(&mut self, h: u32) {
        self.handles.remove(&h);
    }

    pub fn exists(&self, path: &str) -> bool {
        self.resolve(path).exists()
    }

    pub fn size(&mut self, h: u32) -> u32 {
        let Some(f) = self.handles.get_mut(&h) else {
            return 0;
        };
        let cur = f.stream_position().unwrap_or(0);
        let n = f.seek(SeekFrom::End(0)).unwrap_or(0);
        let _ = f.seek(SeekFrom::Start(cur));
        n as u32
    }

    pub fn read(&mut self, h: u32, n: usize) -> Vec<u8> {
        let Some(f) = self.handles.get_mut(&h) else {
            return Vec::new();
        };
        let mut v = vec![0u8; n];
        match f.read(&mut v) {
            Ok(k) => {
                v.truncate(k);
                v
            }
            Err(_) => Vec::new(),
        }
    }

    pub fn write(&mut self, h: u32, data: &[u8]) -> u32 {
        let Some(f) = self.handles.get_mut(&h) else {
            return 0;
        };
        match f.write(data) {
            Ok(k) => k as u32,
            Err(_) => 0,
        }
    }

    pub fn seek(&mut self, h: u32, off: i64, whence: u32) -> u32 {
        let Some(f) = self.handles.get_mut(&h) else {
            return OPEN_FAIL;
        };

        let pos = match whence {
            0 => SeekFrom::Start(off.max(0) as u64),
            1 => SeekFrom::Current(off),
            2 => SeekFrom::End(off),
            _ => return OPEN_FAIL,
        };
        match f.seek(pos) {
            Ok(p) => p as u32,
            Err(_) => OPEN_FAIL,
        }
    }

    pub fn tell(&mut self, h: u32) -> u32 {
        self.handles
            .get_mut(&h)
            .and_then(|f| f.stream_position().ok())
            .unwrap_or(0) as u32
    }

    pub fn mkdir(&self, path: &str) -> bool {
        std::fs::create_dir_all(self.host_path(path)).is_ok()
    }
}

fn mkparent(p: &Path) {
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
}
