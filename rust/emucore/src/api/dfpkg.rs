
use crate::machine::{Emu, Mach};
use crate::runtime;

pub mod off {
    pub const IS_LOADED: u32 = 0x00;
    pub const PACKAGE_NAME: u32 = 0x04;
    pub const FILE_NUM: u32 = 0x08;
    pub const SUB_PACKAGE_NUM: u32 = 0x0a;
    pub const FILE_NAME_TABLE: u32 = 0x0c;
    pub const FILE_OFFSET_TABLE: u32 = 0x10;
    pub const FILE_DATA: u32 = 0x18;
    pub const SUB_DATA_PACKAGE: u32 = 0x1c;
    pub const IS_MOMENT_READ: u32 = 0x54;
    pub const FILE: u32 = 0x5c;
    pub const DATA_SIZE: u32 = 0x60;
    pub const TXT_FILE_DATA: u32 = 0x64;
}

const METHODS: &[(u32, &str, crate::machine::ApiFn)] = &[
    (0x20, "DP_LoadPackage", dp_load),
    (0x24, "DP_ReleasePackage", dp_release),
    (0x28, "DP_LoadFromTResource", dp_load),
    (0x2c, "DP_LoadFormTCard", dp_load),
    (0x30, "DP_DoLoading", dp_noop),
    (0x34, "DP_LocateDataPackage", dp_locate),
    (0x38, "DP_GetFile", dp_get_file),
    (0x3c, "DP_GetFileByID", dp_get_by_id),
    (0x40, "DP_GetFileNameByID", dp_name_by_id),
    (0x44, "DP_GetFileID", dp_file_id),
    (0x48, "DP_ShowFileList", dp_noop),
    (0x4c, "DP_LoadFormTCardEx", dp_load),
    (0x50, "DF_DataPackage_InitTxt", dp_noop),
];

pub fn init_df_datapackage(uc: &mut Emu) {
    let pkg = uc.arg(0);
    let nsub = (uc.arg(1) & 0xFFFF) as u32;
    if pkg == 0 {
        uc.ret(0);
        return;
    }
    for o in [0x04u32, 0x0c, 0x10, 0x18, 0x1c, 0x64] {
        uc.w32(pkg + o, 0);
    }
    uc.w32(pkg + off::FILE_NUM, 0);
    uc.write(pkg + off::IS_LOADED, &[1]);
    uc.write(pkg + off::IS_MOMENT_READ, &[0]);
    uc.w32(pkg + off::FILE, 0xFFFF_FFFF);
    let n = nsub.max(1) * 4;
    let subs = uc
        .get_data_mut()
        .heap
        .alloc(n, "subDataPackage", false)
        .unwrap_or(0);
    if subs != 0 {
        crate::api::fill(uc, subs, 0, n);
    }
    uc.w32(pkg + off::SUB_DATA_PACKAGE, subs);

    uc.write(pkg + off::SUB_PACKAGE_NUM, &(nsub as u16).to_le_bytes());
    for &(o, name, f) in METHODS {
        runtime::install(uc, pkg + o, name, f);
    }
    uc.ret(pkg);
}

fn pick_entries(uc: &Emu, name: &str) -> (String, Vec<(String, Vec<u8>)>) {
    let rt = &uc.get_data().rt;
    let root = rt.packages.iter().find(|(k, _)| k.is_empty());
    let mut arch = None;
    if !name.is_empty() && !rt.packages.is_empty() {
        arch = rt.packages.iter().find(|(k, _)| k == name);
        if arch.is_none() {
            let base = name
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or(name);
            arch = rt
                .packages
                .iter()
                .find(|(k, _)| !k.is_empty() && (k == base || k.eq_ignore_ascii_case(base)));
        }
    }
    let flat = |a: &cbelib::ResArchive| -> Vec<(String, Vec<u8>)> {
        a.entries
            .iter()
            .map(|e| (e.name.clone(), e.data.clone()))
            .collect()
    };
    let Some((k, a)) = arch else {
        return match root {
            Some((_, r)) => ("<root>".to_string(), flat(r)),
            None => (
                "<res>".to_string(),
                rt.res
                    .as_ref()
                    .or(rt.icons.as_ref())
                    .map(flat)
                    .unwrap_or_default(),
            ),
        };
    };
    let key = format!("<sub>{k}");
    let Some((_, r)) = root else { return (key, flat(a)) };

    let mut ents: Vec<(String, Vec<u8>)> = Vec::new();
    let mut pos: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for e in r.entries.iter().chain(a.entries.iter()) {
        match pos.get(&e.name) {
            Some(&i) => ents[i] = (e.name.clone(), e.data.clone()),
            None => {
                pos.insert(e.name.clone(), ents.len());
                ents.push((e.name.clone(), e.data.clone()));
            }
        }
    }
    (format!("<comb>{k}"), ents)
}

fn materialize(uc: &mut Emu, pkg: u32, key: &str, ents: &[(String, Vec<u8>)]) -> u32 {
    let n = ents.len() as u32;
    let cached = uc.get_data().rt.dp_cache.get(key).copied();
    let (names_tbl, offs_tbl, data_p, total) = match cached {
        Some(v) => v,
        None => {
            let blob: Vec<u8> = ents.iter().flat_map(|(_, d)| d.iter().copied()).collect();
            let data_p = uc
                .get_data_mut()
                .heap
                .alloc(blob.len().max(1) as u32, "dp_fileData", false)
                .unwrap_or(0);
            if data_p != 0 && !blob.is_empty() {
                uc.write(data_p, &blob);
            }
            let names_tbl = uc
                .get_data_mut()
                .heap
                .alloc((n.max(1)) * 4, "dp_names", false)
                .unwrap_or(0);
            let offs_tbl = uc
                .get_data_mut()
                .heap
                .alloc((n + 1) * 4, "dp_offsets", false)
                .unwrap_or(0);
            let mut cur = 0u32;
            for (i, (nm, d)) in ents.iter().enumerate() {
                let bytes = nm.as_bytes();
                let p = uc
                    .get_data_mut()
                    .heap
                    .alloc(bytes.len() as u32 + 1, "dp_name", false)
                    .unwrap_or(0);
                if p != 0 {
                    let mut v = bytes.to_vec();
                    v.push(0);
                    uc.write(p, &v);
                }
                uc.w32(names_tbl + i as u32 * 4, p);
                uc.w32(offs_tbl + i as u32 * 4, cur);
                cur += d.len() as u32;
            }
            uc.w32(offs_tbl + n * 4, cur);
            let v = (names_tbl, offs_tbl, data_p, cur);
            if std::env::var("DPDBG").is_ok() {
                eprintln!("MAT\t{key}\t{n}\t{data_p:#x}");
            }
            uc.get_data_mut().rt.dp_cache.insert(key.to_string(), v);
            v
        }
    };
    uc.write(pkg + off::FILE_NUM, &(n as u16).to_le_bytes());
    uc.w32(pkg + off::FILE_NAME_TABLE, names_tbl);
    uc.w32(pkg + off::FILE_OFFSET_TABLE, offs_tbl);
    uc.w32(pkg + off::FILE_DATA, data_p);
    uc.w32(pkg + off::DATA_SIZE, total);
    uc.write(pkg + off::IS_LOADED, &[1]);
    n
}

pub fn dp_load(uc: &mut Emu) {
    let pkg = uc.arg(0);
    let nameptr = uc.arg(1);
    let raw = uc.cstr(nameptr, 256).unwrap_or_default();
    let name = String::from_utf8_lossy(&raw).to_string();
    let (key, ents) = pick_entries(uc, &name);
    materialize(uc, pkg, &key, &ents);
    let names: Vec<(String, u32)> = ents
        .iter()
        .map(|(n, d)| (n.clone(), d.len() as u32))
        .collect();
    {
        let v = &mut uc.get_data_mut().rt.pkg_entries;
        match v.iter_mut().find(|(k, _)| *k == pkg) {
            Some(e) => e.1 = names,
            None => v.push((pkg, names)),
        }
    }
    uc.w32(pkg + off::PACKAGE_NAME, nameptr);
    uc.ret(0);
}

pub fn dp_release(uc: &mut Emu) {
    let p = uc.arg(0);
    uc.write(p + off::IS_LOADED, &[0]);
    uc.ret(0);
}

pub fn dp_noop(uc: &mut Emu) {
    uc.ret(0);
}

pub fn dp_locate(uc: &mut Emu) {
    let v = uc.arg(0);
    uc.ret(v);
}

pub fn entries_opt(uc: &Emu, pkg: Option<u32>) -> Vec<(String, u32)> {
    let rt = &uc.get_data().rt;
    if let Some(p) = pkg {
        if let Some((_, v)) = rt.pkg_entries.iter().find(|(k, _)| *k == p) {
            return v.clone();
        }
    }
    if let Some((_, v)) = rt.pkg_entries.first() {
        return v.clone();
    }
    let a = rt.res.as_ref().or(rt.icons.as_ref());
    a.map(|a| {
        a.entries
            .iter()
            .map(|e| (e.name.clone(), e.size as u32))
            .collect()
    })
    .unwrap_or_default()
}

pub fn entries_for(uc: &Emu, pkg: u32) -> Vec<(String, u32)> {
    entries_opt(uc, Some(pkg))
}

pub fn dp_file_id(uc: &mut Emu) {
    let raw = uc.cstr(uc.arg(1), 256).unwrap_or_default();
    let name = String::from_utf8_lossy(&raw).to_string();
    let ents = entries_for(uc, uc.arg(0));
    match ents.iter().position(|(n, _)| *n == name) {
        Some(i) => uc.ret(i as u32),

        None => uc.ret(0xFFFF),
    }
}

pub fn dp_name_by_id(uc: &mut Emu) {
    let pkg = uc.arg(0);
    let i = uc.arg(1) & 0xFFFF;
    let tbl = uc.r32(pkg + off::FILE_NAME_TABLE);
    let n = entries_for(uc, pkg).len() as u32;
    if tbl != 0 && i < n {
        let v = uc.r32(tbl + i * 4);
        uc.ret(v);
    } else {
        uc.ret(0);
    }
}

pub fn dp_get_by_id(uc: &mut Emu) {
    let pkg = uc.arg(0);
    let i = uc.arg(1) & 0xFFFF;
    let data = uc.r32(pkg + off::FILE_DATA);
    let offs = uc.r32(pkg + off::FILE_OFFSET_TABLE);
    let n = entries_for(uc, pkg).len() as u32;
    if data == 0 || offs == 0 || i >= n {
        uc.ret(0);
        return;
    }
    let o = uc.r32(offs + i * 4);
    uc.ret(data + o);
}

pub fn dp_get_file(uc: &mut Emu) {
    let pkg = uc.arg(0);
    let raw = uc.cstr(uc.arg(1), 256).unwrap_or_default();
    let name = String::from_utf8_lossy(&raw).to_string();
    let ents = entries_for(uc, pkg);
    let Some(i) = ents.iter().position(|(n, _)| *n == name) else {
        uc.ret(0);
        return;
    };
    let data = uc.r32(pkg + off::FILE_DATA);
    let offs = uc.r32(pkg + off::FILE_OFFSET_TABLE);
    if data == 0 || offs == 0 {
        uc.ret(0);
        return;
    }
    let o = uc.r32(offs + i as u32 * 4);
    uc.ret(data + o);
}

fn res_index(uc: &Emu, name: &str) -> Option<usize> {
    entries_opt(uc, None).iter().position(|(n, _)| n == name)
}

fn res_ptr(uc: &Emu, i: usize) -> u32 {
    let pkg = uc.get_data().rt.datapackage;
    if pkg == 0 {
        return 0;
    }
    let data = uc.r32(pkg + off::FILE_DATA);
    let offs = uc.r32(pkg + off::FILE_OFFSET_TABLE);
    if data == 0 || offs == 0 {
        return 0;
    }
    data + uc.r32(offs + i as u32 * 4)
}

pub fn df_res_id_by_name(uc: &mut Emu) {
    let raw = uc.cstr(uc.arg(0), 256).unwrap_or_default();
    let name = String::from_utf8_lossy(&raw).to_string();
    match res_index(uc, &name) {
        Some(i) => uc.ret(i as u32),
        None => uc.ret(0xFFFF_FFFF),
    }
}

pub fn df_res_by_name(uc: &mut Emu) {
    let raw = uc.cstr(uc.arg(0), 256).unwrap_or_default();
    let name = String::from_utf8_lossy(&raw).to_string();
    let p = res_index(uc, &name).map(|i| res_ptr(uc, i)).unwrap_or(0);
    if std::env::var("RESDBG").is_ok() {
        let pkg = uc.get_data().rt.datapackage;
        let data = if pkg != 0 { uc.r32(pkg + off::FILE_DATA) } else { 0 };
        let offs = if pkg != 0 { uc.r32(pkg + off::FILE_OFFSET_TABLE) } else { 0 };
        let n = entries_opt(uc, None).len();
        eprintln!("RES\t{name}\tidx={:?}\tpkg={pkg:#x}\tdata={data:#x}\toffs={offs:#x}\tn={n}\t-> {p:#x}",
                  res_index(uc, &name));
    }
    uc.ret(p);
}

pub fn df_res_by_id(uc: &mut Emu) {
    let i = uc.arg(0);
    let n = entries_opt(uc, None).len() as u32;
    let p = if i < n { res_ptr(uc, i as usize) } else { 0 };
    uc.ret(p);
}

pub fn res_ptr_by_name(uc: &Emu, name: &str) -> u32 {
    match res_index(uc, name) {
        Some(i) => res_ptr(uc, i),
        None => 0,
    }
}

pub fn df_res_name_by_id(uc: &mut Emu) {
    let pkg = uc.get_data().rt.datapackage;
    let i = uc.arg(0);
    let tbl = if pkg != 0 {
        uc.r32(pkg + off::FILE_NAME_TABLE)
    } else {
        0
    };
    let n = entries_for(uc, pkg).len() as u32;
    uc.ret(if tbl != 0 && i < n { uc.r32(tbl + i * 4) } else { 0 });
}

pub fn entry_data(uc: &Emu, i: usize) -> Option<Vec<u8>> {
    let pkg = uc.get_data().rt.datapackage;
    let ents = entries_opt(uc, None);
    let (_, size) = ents.get(i)?;
    let data = uc.r32(pkg + off::FILE_DATA);
    let offs = uc.r32(pkg + off::FILE_OFFSET_TABLE);
    if data == 0 || offs == 0 {
        return None;
    }
    let o = uc.r32(offs + i as u32 * 4);
    uc.mem_read_as_vec((data + o) as u64, *size as usize).ok()
}
