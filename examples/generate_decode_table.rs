fn main() {
    // Scalar tables

    // map control bytes to encoded num lengths
    println!("pub const SCALAR_DECODE_TABLE: &'static [(u8, u8, u8, u8); 256] = &[");

    // work around lack of closed ranges until that hits stable rust
    for b in 0..256 {
        let byte = b as u8;

        let len0 = (((byte & 0xC0) >> 6) + 1) as usize;
        let len1 = (((byte & 0x30) >> 4) + 1) as usize;
        let len2 = (((byte & 0x0C) >> 2) + 1) as usize;
        let len3 = ((byte & 0x3) + 1) as usize;

        println!("    ({}, {}, {}, {}), // {} = 0x{:X} = 0b{:08b}, lengths {} {} {} {}",
                 len0, len1, len2, len3, byte, byte, byte, len0, len1, len2, len3);
    }

    println!("];");
    println!();

    // SSSE3 tables

    println!("#[cfg(feature = \"x86_ssse3\")]");
    println!("pub const X86_SSSE3_DECODE_LENGTH_TABLE: &'static [u8; 256] = &[");

    for b in 0..256 {
        let byte = b as u8;

        let len0 = (((byte & 0xC0) >> 6) + 1) as usize;
        let len1 = (((byte & 0x30) >> 4) + 1) as usize;
        let len2 = (((byte & 0x0C) >> 2) + 1) as usize;
        let len3 = ((byte & 0x3) + 1) as usize;

        println!("    {}, // {} = 0x{:X} = 0b{:08b}, lengths {} {} {} {}",
                 len0 + len1 + len2 + len3, byte, byte, byte, len0, len1, len2, len3);
    }

    println!("];");
    println!();

    // don't warn if SSSE3 is disabled
    println!("#[cfg(feature = \"x86_ssse3\")]");
    println!("pub const X86_SSSE3_DECODE_SHUFFLE_TABLE: &'static [[u8; 16]; 256] = &[");

    for b in 0..256 {
        let byte = b as u8;

        let len0 = (((byte & 0xC0) >> 6) + 1) as usize;
        let len1 = (((byte & 0x30) >> 4) + 1) as usize;
        let len2 = (((byte & 0x0C) >> 2) + 1) as usize;
        let len3 = ((byte & 0x3) + 1) as usize;

        // map encoded numbers to 4 adjacent big-endian u32s
        let mut shuffle_bytes = Vec::new();
        push_shuffle_bytes(0, len0, &mut shuffle_bytes);
        push_shuffle_bytes(len0, len1, &mut shuffle_bytes);
        push_shuffle_bytes(len0 + len1, len2, &mut shuffle_bytes);
        push_shuffle_bytes(len0 + len1 + len2, len3, &mut shuffle_bytes);

        assert_eq!(16, shuffle_bytes.len());

        println!("    [{}], // {} = 0x{:X} = 0b{:08b}, lengths {} {} {} {}",
                 shuffle_bytes.iter().map(|b| format!("{:4 }", b))
                         .collect::<Vec<String>>()
                         .join(", "),
                 byte, byte, byte, len0, len1, len2, len3);
    }

    println!("];");
}

/// Push the shuffle indices
fn push_shuffle_bytes(start_of_encoded_num: usize, encoded_length: usize, shuffle_bytes: &mut Vec<u8>) {
    // least significant byte will be at the end
    let end_of_encoded_num = start_of_encoded_num + encoded_length - 1;
    // map encoded bytes to dest bytes, least significant first
    for l in 0..encoded_length {
        shuffle_bytes.push((end_of_encoded_num - l) as u8);
    }

    // zero out unused most significant bytes
    // high bit set = populate destination with 0 byte
    for _ in 0..(4 - encoded_length) {
        shuffle_bytes.push(0x80);
    }
}
