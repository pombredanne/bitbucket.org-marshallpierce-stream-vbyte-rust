fn main() {
    // Scalar tables

    // map control bytes to encoded num lengths
    println!("#[cfg_attr(rustfmt, rustfmt_skip)]");
    println!("pub const DECODE_LENGTH_PER_NUM_TABLE: &'static [(u8, u8, u8, u8); 256] = &[");

    // work around lack of closed ranges until that hits stable rust
    for b in 0..256 {
        let byte = b as u8;

        let (len0, len1, len2, len3) = lengths_for_control_byte(byte);

        println!(
            "    ({}, {}, {}, {}), // {} = 0x{:X} = 0b{:08b}, lengths {} {} {} {}",
            len0,
            len1,
            len2,
            len3,
            byte,
            byte,
            byte,
            len0,
            len1,
            len2,
            len3
        );
    }

    println!("];");
    println!();

    // SSSE3 tables

    println!("#[cfg_attr(rustfmt, rustfmt_skip)]");
    println!("pub const DECODE_LENGTH_PER_QUAD_TABLE: &'static [u8; 256] = &[");

    for b in 0..256 {
        let byte = b as u8;

        let (len0, len1, len2, len3) = lengths_for_control_byte(byte);

        println!(
            "    {}, // {} = 0x{:X} = 0b{:08b}, lengths {} {} {} {}",
            len0 + len1 + len2 + len3,
            byte,
            byte,
            byte,
            len0,
            len1,
            len2,
            len3
        );
    }

    println!("];");
    println!();

    println!("#[cfg_attr(rustfmt, rustfmt_skip)]");
    println!("#[cfg(feature = \"x86_ssse3\")]");
    println!("pub const X86_SSSE3_DECODE_SHUFFLE_TABLE: &'static [[u8; 16]; 256] = &[");

    for b in 0..256 {
        let byte = b as u8;

        let (len0, len1, len2, len3) = lengths_for_control_byte(byte);

        // map encoded numbers to 4 adjacent u32s
        let mut shuffle_bytes = Vec::new();
        push_decode_u32_shuffle_bytes(0, len0, &mut shuffle_bytes);
        push_decode_u32_shuffle_bytes(len0, len1, &mut shuffle_bytes);
        push_decode_u32_shuffle_bytes(len0 + len1, len2, &mut shuffle_bytes);
        push_decode_u32_shuffle_bytes(len0 + len1 + len2, len3, &mut shuffle_bytes);

        assert_eq!(16, shuffle_bytes.len());

        println!(
            "    [{}], // {} = 0x{:X} = 0b{:08b}, lengths {} {} {} {}",
            shuffle_bytes
                .iter()
                .map(|b| format!("{:4 }", b))
                .collect::<Vec<String>>()
                .join(", "),
            byte,
            byte,
            byte,
            len0,
            len1,
            len2,
            len3
        );
    }

    println!("];");
    println!();

    println!("#[cfg_attr(rustfmt, rustfmt_skip)]");
    println!("#[cfg(feature = \"x86_sse41\")]");
    println!("pub const X86_ENCODE_SHUFFLE_TABLE: &'static [[u8; 16]; 256] = &[");

    for b in 0..256 {
        let byte = b as u8;

        let (len0, len1, len2, len3) = lengths_for_control_byte(byte);

        // map 4 adjacent u32s to encoded numbers
        let mut shuffle_bytes = Vec::new();
        push_encode_u32_shuffle_bytes(0, len0, &mut shuffle_bytes);
        push_encode_u32_shuffle_bytes(4, len1, &mut shuffle_bytes);
        push_encode_u32_shuffle_bytes(8, len2, &mut shuffle_bytes);
        push_encode_u32_shuffle_bytes(12, len3, &mut shuffle_bytes);

        // fill the rest with bytes with the high bit set so output will be zero'd
        shuffle_bytes.resize(16, 128);

        assert_eq!(16, shuffle_bytes.len());

        println!(
            "    [{}], // {} = 0x{:X} = 0b{:08b}, lengths {} {} {} {}",
            shuffle_bytes
                .iter()
                .map(|b| format!("{:4 }", b))
                .collect::<Vec<String>>()
                .join(", "),
            byte,
            byte,
            byte,
            len0,
            len1,
            len2,
            len3
        );
    }

    println!("];");
}

/// Push 4 shuffle bytes into a SSSE3 PSHUFB mask
fn push_decode_u32_shuffle_bytes(
    start_of_encoded_num: usize,
    encoded_length: usize,
    shuffle_bytes: &mut Vec<u8>,
) {
    // Encoded nums are little-endian, and so is destination because SSSE3 is x86.
    // So, just copy the bytes in order for all the encoded bytes
    for l in 0..encoded_length {
        shuffle_bytes.push((start_of_encoded_num + l) as u8);
    }

    // Zero out any unused most significant bytes in the final u32
    // high bit set = populate destination with 0 byte
    for _ in 0..(4 - encoded_length) {
        shuffle_bytes.push(0x80);
    }
}

fn push_encode_u32_shuffle_bytes(
    start_of_num: usize,
    encoded_length: usize,
    shuffle_bytes: &mut Vec<u8>,
) {
    // copy only the low bytes that are actually set
    for l in 0..encoded_length {
        shuffle_bytes.push((start_of_num + l) as u8);
    }
}

fn lengths_for_control_byte(byte: u8) -> (usize, usize, usize, usize) {
    let len3 = (((byte & 0xC0) >> 6) + 1) as usize;
    let len2 = (((byte & 0x30) >> 4) + 1) as usize;
    let len1 = (((byte & 0x0C) >> 2) + 1) as usize;
    let len0 = ((byte & 0x3) + 1) as usize;

    (len0, len1, len2, len3)
}
