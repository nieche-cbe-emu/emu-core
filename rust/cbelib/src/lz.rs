
pub fn decompress(src: &[u8], out_size: usize) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::with_capacity(out_size);
    let mut i = 0usize;
    let n = src.len();
    while i < n && out.len() < out_size {
        let c = src[i];
        if c & 0x80 != 0 {
            let k = (c & 0x7F) as usize;
            i += 1;

            let end = (i + k).min(n);
            out.extend_from_slice(&src[i.min(n)..end]);
            i += k;
        } else {
            if i + 1 >= n {
                break;
            }
            let k = (c >> 1) as usize;
            let dist = (((c & 1) as usize) << 8) | src[i + 1] as usize;
            i += 2;
            if dist == 0 || dist > out.len() {
                break;
            }
            for _ in 0..k.min(out_size - out.len()) {
                let b = out[out.len() - dist];
                out.push(b);
            }
        }
    }
    out
}

pub fn unpack_entry(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 9 || data[0] != 2 {
        return None;
    }
    let comp = u32::from_be_bytes([data[1], data[2], data[3], data[4]]) as usize;
    let unc = u32::from_be_bytes([data[5], data[6], data[7], data[8]]) as usize;
    let end = (9 + comp).min(data.len());
    Some(decompress(&data[9..end], unc))
}
