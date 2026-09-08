
pub const PAGE: u32 = 0x1000;

pub fn align_up(x: u32, a: u32) -> u32 {
    ((x as u64 + a as u64 - 1) / a as u64 * a as u64) as u32
}

pub fn align_page(x: u32) -> u32 {
    align_up(x, PAGE)
}

#[derive(Debug, Clone)]
pub struct Region {
    pub name: &'static str,
    pub base: u32,
    pub size: u32,
}

impl Region {
    pub fn contains(&self, a: u32) -> bool {
        a >= self.base && (a as u64) < self.base as u64 + self.size as u64
    }
}

pub struct Bump {
    pub base: u32,
    pub size: u32,
    pub cur: u32,

    blocks: std::collections::HashMap<u32, (u32, String)>,

    freelist: Vec<(u32, u32)>,
}

impl Bump {

    pub const MAX_ALLOC: u32 = 0x0400_0000;

    pub fn new(base: u32, size: u32) -> Self {
        Bump {
            base,
            size,
            cur: base,
            blocks: std::collections::HashMap::new(),
            freelist: Vec::new(),
        }
    }

    pub fn alloc(&mut self, n: u32, tag: &str, strict: bool) -> Result<u32, String> {
        if n > Self::MAX_ALLOC {
            return if strict {
                Err(format!("单次分配 {n:#x} 过大（tag={tag}）"))
            } else {
                Ok(0)
            };
        }
        let n = align_up(n.max(1), 16);
        for i in 0..self.freelist.len() {
            let (p, sz) = self.freelist[i];
            if sz >= n {
                if sz > n {
                    self.freelist[i] = (p + n, sz - n);
                } else {
                    self.freelist.remove(i);
                }
                self.blocks.insert(p, (n, tag.to_string()));
                return Ok(p);
            }
        }
        if self.cur as u64 + n as u64 > self.base as u64 + self.size as u64 {
            return if strict {
                Err(format!(
                    "模拟堆耗尽（已用 {:#x} / {:#x}）",
                    self.cur - self.base,
                    self.size
                ))
            } else {
                Ok(0)
            };
        }
        let p = self.cur;
        self.cur += n;
        self.blocks.insert(p, (n, tag.to_string()));
        Ok(p)
    }

    pub fn free(&mut self, p: u32) {
        let n = match self.blocks.remove(&p) {
            Some((n, _)) => n,
            None => return,
        };
        if p + n == self.cur {

            self.cur = p;
            return;
        }
        let i = self.freelist.partition_point(|&(a, _)| a < p);
        self.freelist.insert(i, (p, n));

        let mut j = i;
        while j + 1 < self.freelist.len()
            && self.freelist[j].0 + self.freelist[j].1 == self.freelist[j + 1].0
        {
            let (b, sb) = self.freelist.remove(j + 1);
            let _ = b;
            self.freelist[j].1 += sb;
        }

        while j > 0 && self.freelist[j - 1].0 + self.freelist[j - 1].1 == self.freelist[j].0 {
            let (_, sb) = self.freelist.remove(j);
            self.freelist[j - 1].1 += sb;
            j -= 1;
        }
    }

    pub fn block_size(&self, p: u32) -> Option<u32> {
        self.blocks.get(&p).map(|&(n, _)| n)
    }

    pub fn used(&self) -> u32 {
        self.cur - self.base
    }

    pub fn live_blocks(&self) -> usize {
        self.blocks.len()
    }

    pub fn free_blocks(&self) -> usize {
        self.freelist.len()
    }
}

pub mod layout {

    pub const NULL_GUARD: u32 = 0x0002_0000;

    pub const RO_DEFAULT: u32 = 0x0100_0000;
    pub const RW_BASE: u32 = 0x2000_0000;
    pub const STACK_BASE: u32 = 0x3000_0000;
    pub const STACK_SIZE: u32 = 0x0010_0000;
    pub const HEAP_BASE: u32 = 0x4000_0000;
    pub const HEAP_SIZE: u32 = 0x1000_0000;

    pub const NATIVE_BASE: u32 = 0x5100_0000;
    pub const NATIVE_SIZE: u32 = 0x0001_0000;
    pub const TRAP_BASE: u32 = 0x5000_0000;

    pub const TRAP_SIZE: u32 = 0x0004_0000;

    pub const DATA_BASE: u32 = 0x6000_0000;
    pub const DATA_SIZE: u32 = 0x0040_0000;

    pub const RETURN_MAGIC: u32 = 0x7FFF_0000;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placement {
    pub ro_base: u32,
    pub ro_size: u32,
    pub rw_base: u32,
    pub rw_size: u32,
}

pub fn place(m: &cbelib::CbeModule) -> Placement {
    let ro_base = if m.load_base != 0 {
        m.load_base
    } else {
        layout::RO_DEFAULT
    };
    let ro_size = align_page(m.image_size.max(m.ro.len() as u32));
    let rw_size = align_page(m.rw_size.max(m.rw.len() as u32) + 0x10000);
    let rw_base = if m.load_base != 0 {
        rw_place(m)
    } else {
        layout::RW_BASE
    };
    Placement {
        ro_base,
        ro_size,
        rw_base,
        rw_size,
    }
}

pub fn rw_place(m: &cbelib::CbeModule) -> u32 {
    let end = m.image_end;
    let lo = m.load_base;
    let hi = m.load_base as u64 + m.image_size.max(m.ro.len() as u32) as u64 + 0x10000;
    if end > lo && (end as u64) < hi {
        end
    } else {
        m.load_base + align_up(m.ro.len() as u32, 16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bump_reuses_freed_blocks() {
        let mut b = Bump::new(0x4000_0000, 0x1000);
        let a = b.alloc(0x100, "a", true).unwrap();
        let c = b.alloc(0x100, "c", true).unwrap();
        assert_eq!(a, 0x4000_0000);
        assert_eq!(c, 0x4000_0100);

        b.free(a);
        assert_eq!(b.alloc(0x100, "d", true).unwrap(), a);
    }

    #[test]
    fn bump_pops_top_block() {
        let mut b = Bump::new(0x4000_0000, 0x1000);
        let a = b.alloc(0x100, "a", true).unwrap();
        let c = b.alloc(0x100, "c", true).unwrap();
        b.free(c);

        assert_eq!(b.cur, c);
        assert_eq!(b.alloc(0x100, "e", true).unwrap(), c);
        let _ = a;
    }

    #[test]
    fn bump_merges_adjacent_free_blocks() {
        let mut b = Bump::new(0x4000_0000, 0x10000);
        let a = b.alloc(0x100, "a", true).unwrap();
        let c = b.alloc(0x100, "c", true).unwrap();
        let _keep = b.alloc(0x100, "keep", true).unwrap();
        b.free(a);
        b.free(c);
        assert_eq!(b.free_blocks(), 1, "相邻的两块应当合并成一块");

        assert_eq!(b.alloc(0x200, "big", true).unwrap(), a);
    }

    #[test]
    fn alloc_rounds_up_to_16() {
        let mut b = Bump::new(0x4000_0000, 0x1000);
        b.alloc(1, "x", true).unwrap();
        assert_eq!(b.cur, 0x4000_0010);
    }

    #[test]
    fn oversized_alloc_is_refused() {
        let mut b = Bump::new(0x4000_0000, 0x1000);
        assert!(b.alloc(Bump::MAX_ALLOC + 1, "junk", true).is_err());
        assert_eq!(b.alloc(Bump::MAX_ALLOC + 1, "junk", false).unwrap(), 0);
    }
}
