
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::machine::{Emu, Mach};
use crate::{gfx, runtime};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Touch {
    Down,
    Move,
    Up,
}

impl Touch {

    pub fn from_i32(v: i32) -> Touch {
        match v {
            1 => Touch::Up,
            2 => Touch::Move,
            _ => Touch::Down,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Event {

    Audio(String),

    Exit,

    Log(String),
}

impl Event {

    pub fn to_json(&self) -> String {
        match self {

            Event::Audio(body) => {
                let inner = body.trim();
                let inner = inner.strip_prefix('{').unwrap_or(inner);
                let inner = inner.strip_suffix('}').unwrap_or(inner);
                format!("{{\"kind\":\"audio\",{inner}}}")
            }
            Event::Exit => "{\"kind\":\"exit\",\"why\":\"module\"}".to_string(),
            Event::Log(t) => format!("{{\"kind\":\"log\",\"text\":{}}}", json_str(t)),
        }
    }
}

pub fn json_str(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

const REP_DELAY: f64 = 0.40;
const REP_RATE: f64 = 0.12;

const SOFT_POS_LEFT: (f64, f64) = (0.08, 0.96);
const SOFT_POS_RIGHT: (f64, f64) = (0.93, 0.96);
const SOFT_BIT_LEFT: u32 = 1 << 12;
const SOFT_BIT_RIGHT: u32 = 1 << 13;

const SOFT_WAIT: i32 = 8;

const SOFT_MIN_CHANGE: f64 = 0.10;

struct SoftPending {
    bit: u32,
    left: i32,
    before: Vec<u8>,
}

pub struct Session {
    pub uc: Emu<'static>,

    keys: u32,

    latched: u32,
    prev_keys: u32,

    rep_next: HashMap<u32, f64>,

    touch: Option<(i32, i32, Touch)>,
    touch_queue: Vec<(i32, i32, Touch)>,
    soft: Option<SoftPending>,

    kev: u32,
    pub frame_no: u64,

    nlog: usize,
    events: Vec<Event>,
    pub alive: bool,
    booted: bool,
}

fn now_secs() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

impl Session {

    pub fn open(path: &str) -> Result<Session, String> {
        let m = cbelib::load(path).map_err(|e| format!("{e}"))?;
        let mut uc = crate::machine::build(&m)?;
        runtime::setup(&mut uc, &m);
        let kev = uc
            .get_data_mut()
            .data
            .alloc(4, "KeyEvent", true)
            .unwrap_or(0);
        Ok(Session {
            uc,
            keys: 0,
            latched: 0,
            prev_keys: 0,
            rep_next: HashMap::new(),
            touch: None,
            touch_queue: Vec::new(),
            soft: None,
            kev,
            frame_no: 0,
            nlog: 0,
            events: Vec::new(),
            alive: true,
            booted: false,
        })
    }

    pub fn boot(&mut self) -> bool {
        if self.booted {
            return true;
        }
        runtime::boot(&mut self.uc);
        runtime::app_start(&mut self.uc);
        self.booted = true;
        true
    }

    pub fn stop(&mut self) {
        if !self.alive {
            return;
        }
        self.alive = false;
        if self.booted {
            runtime::app_stop(&mut self.uc);
        }
    }

    pub fn set_keys(&mut self, mask: u32) {
        self.latched |= mask & !self.keys;
        self.keys = mask;
    }

    pub fn set_touch(&mut self, x: i32, y: i32, state: Touch) {

        if state == Touch::Move
            && self
                .touch_queue
                .last()
                .is_some_and(|e| e.2 == Touch::Move)
        {
            *self.touch_queue.last_mut().unwrap() = (x, y, state);
        } else if self.touch_queue.len() < 64 {
            self.touch_queue.push((x, y, state));
        }
    }

    pub fn soft_key(&mut self, side: &str, pressed: bool) {
        if !pressed {
            return;
        }
        let (pos, bit) = match side {
            "left" => (SOFT_POS_LEFT, SOFT_BIT_LEFT),
            "right" => (SOFT_POS_RIGHT, SOFT_BIT_RIGHT),
            _ => return,
        };
        let (w, h) = {
            let g = &self.uc.get_data().rt.gfx;
            (g.w, g.h)
        };
        let x = (w as f64 * pos.0) as i32;
        let y = (h as f64 * pos.1) as i32;
        self.set_touch(x, y, Touch::Down);
        self.set_touch(x, y, Touch::Up);
        self.soft = Some(SoftPending {
            bit,
            left: SOFT_WAIT,
            before: gfx::raw565(&self.uc),
        });
    }

    fn soft_followup(&mut self) {
        let Some(sp) = self.soft.as_mut() else { return };
        sp.left -= 1;
        if sp.left > 0 {
            return;
        }
        let sp = self.soft.take().unwrap();
        let cur = gfx::raw565(&self.uc);

        let n = cur.len() / 16;
        let changed = if sp.before == cur {
            0
        } else {
            (0..cur.len())
                .step_by(16)
                .filter(|&i| sp.before.get(i) != cur.get(i))
                .count()
        };
        if (changed as f64) < n as f64 * SOFT_MIN_CHANGE {
            self.latched |= sp.bit;
        }
    }

    fn autorepeat(&mut self, bits: u32, now: f64) -> u32 {
        let mut out = 0u32;
        for b in 0..32u32 {
            let m = 1u32 << b;
            if bits & m == 0 {
                self.rep_next.remove(&b);
            } else {
                match self.rep_next.get(&b).copied() {
                    None => {
                        self.rep_next.insert(b, now + REP_DELAY);
                    }
                    Some(t) if now >= t => {
                        self.rep_next.insert(b, now + REP_RATE);
                        out |= m;
                    }
                    _ => {}
                }
            }
        }
        out
    }

    fn apply_input(&mut self, now: f64) {
        let bits = self.keys | self.latched;
        self.latched = 0;

        let down = bits & !self.prev_keys;
        let up = self.prev_keys & !bits;
        let rep = self.autorepeat(bits, now);
        self.prev_keys = bits;
        {
            let rt = &mut self.uc.get_data_mut().rt;
            rt.keys_down = down | rep;
            rt.keys_hold = bits;
            rt.keys_up = up;
        }

        if !self.touch_queue.is_empty() {
            self.touch = Some(self.touch_queue.remove(0));
        }
        if let Some((x, y, st)) = self.touch {
            let rt = &mut self.uc.get_data_mut().rt;
            rt.pointer = (x, y);
            rt.touch_down = u32::from(st == Touch::Down);
            rt.touch_hold = u32::from(st == Touch::Down || st == Touch::Move);
            rt.touch_up = u32::from(st == Touch::Up);
            rt.touch_drag = u32::from(st == Touch::Move);
            self.touch = match st {
                Touch::Up => None,

                Touch::Down => Some((x, y, Touch::Move)),
                Touch::Move => Some((x, y, Touch::Move)),
            };
        }
    }

    fn key_event(&mut self) -> (u32, u32) {
        let (d, u) = {
            let rt = &self.uc.get_data().rt;
            (rt.keys_down, rt.keys_up)
        };
        if d != 0 {
            let k = self.kev;
            self.uc.w32(k, d);
            return (0, k);
        }
        if u != 0 {
            let k = self.kev;
            self.uc.w32(k, u);
            return (1, k);
        }
        (runtime::NO_EVENT, 0)
    }

    pub fn step(&mut self) -> Vec<u8> {
        self.step_at(now_secs())
    }

    pub fn step_at(&mut self, now: f64) -> Vec<u8> {
        if !self.booted {
            return gfx::raw565(&self.uc);
        }
        self.apply_input(now);
        let (event, data) = self.key_event();

        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            runtime::frame(&mut self.uc, event, data);
        }));
        if r.is_err() {
            self.events.push(Event::Log("[宿主错误] 帧执行 panic".to_string()));
        }
        self.frame_no += 1;
        self.soft_followup();
        self.drain();
        gfx::raw565(&self.uc)
    }

    fn drain(&mut self) {
        let auds: Vec<String> = std::mem::take(&mut self.uc.get_data_mut().rt.audio.events);
        for a in auds {
            self.events.push(Event::Audio(a));
        }
        if self.uc.get_data().rt.exit_requested && self.alive {
            self.events.push(Event::Exit);
            self.alive = false;
        }
        let logs = &self.uc.get_data().rt.logs;
        if logs.len() > self.nlog {
            let new: Vec<String> = logs[self.nlog..].to_vec();
            self.nlog = logs.len();
            for l in new {
                self.events.push(Event::Log(l));
            }
        }
    }

    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    pub fn put_back_events(&mut self, mut evs: Vec<Event>) {
        evs.append(&mut self.events);
        self.events = evs;
    }

    pub fn peek_logs(&self) -> String {
        self.uc.get_data().rt.logs.join("\n")
    }

    pub fn take_logs(&mut self) -> String {
        let joined = self.uc.get_data().rt.logs.join("\n");
        self.uc.get_data_mut().rt.logs.clear();
        self.nlog = 0;
        joined
    }

    pub fn size(&self) -> (u32, u32) {
        let g = &self.uc.get_data().rt.gfx;
        (g.w, g.h)
    }

    pub fn name(&self) -> String {
        self.uc.get_data().rt.module_name.clone()
    }

    pub fn screens(&self) -> u32 {
        self.uc.get_data().rt.screens.len() as u32
    }

    pub fn nonblank(&self) -> u32 {
        gfx::nonblank(&self.uc)
    }
}
