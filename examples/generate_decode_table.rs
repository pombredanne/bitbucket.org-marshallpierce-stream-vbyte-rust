fn main() {
    // map control bytes to lengths

    println!("pub const DECODE_TABLE: &'static [(u8, u8, u8, u8)] = &[");

    // work around lack of closed ranges until that hits stable rust
    for b in 0..256 {
        let byte = b as u8;

        let len0 = (((byte & 0xC0) >> 6) + 1) as usize;
        let len1 = (((byte & 0x30) >> 4) + 1) as usize;
        let len2 = (((byte & 0x0C) >> 2) + 1) as usize;
        let len3 = ((byte & 0x3) + 1) as usize;

        println!("    ({}, {}, {}, {}), // {} = 0x{:X} = 0b{:08b}", len0, len1, len2, len3, byte, byte, byte);
    }

    println!("];")
}
