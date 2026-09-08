
pub mod container;
pub mod imgcodec;
pub mod lz;

pub use imgcodec::{decode as decode_image, Image, ImgError};
pub use lz::{decompress, unpack_entry};
pub use container::{
    load, load_bytes, parse_multi, parse_res, CbeError, CbeModule, Endian, ResArchive, ResEntry,
};

pub fn crc32(data: &[u8]) -> u32 {
    const POLY: u32 = 0xEDB8_8320;
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ POLY } else { crc >> 1 };
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_zlib() {

        assert_eq!(crc32(b""), 0x0000_0000);
        assert_eq!(crc32(b"a"), 0xE8B7_BE43);
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b"CoolBars"), 0xB6A3_0BFB);
    }
}
