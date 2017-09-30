extern crate x86intrin;

use super::tables;

use self::x86intrin::{sse2, ssse3, m128i};

pub struct Ssse3;

impl super::Decoder for Ssse3 {
    fn decode_quads(control_bytes: &[u8], encoded_nums: &[u8], output: &mut [u32]) -> (usize, usize) {
        let mut bytes_read: usize = 0;
        let mut nums_decoded = 0;

        // need to ensure that we can copy 16 encoded bytes, so last few quads will be handled
        // by a slower loop
        for &control_byte in control_bytes[0..(control_bytes.len().saturating_sub(4))].iter() {
            let length = tables::X86_SSSE3_DECODE_LENGTH_TABLE[control_byte as usize];
            let mask_bytes = tables::X86_SSSE3_DECODE_SHUFFLE_TABLE[control_byte as usize];
            let next_4 = &encoded_nums[bytes_read..(bytes_read + 16)];

            let mask;
            let data;
            unsafe {
                // TODO can I load mask with mm_load_si128? Will it always be 16-byte aligned?
                mask = sse2::mm_loadu_si128(mask_bytes.as_ptr() as *const m128i);
                data = sse2::mm_loadu_si128(next_4.as_ptr() as *const m128i);
            }

            let shuffled = ssse3::mm_shuffle_epi8(data, mask).as_u32x4();

            output[nums_decoded] = shuffled.extract(0);
            output[nums_decoded + 1] = shuffled.extract(1);
            output[nums_decoded + 2] = shuffled.extract(2);
            output[nums_decoded + 3] = shuffled.extract(3);


//            println!("control byte {:08b}\n\tmask {}\n\tdata {}\n\tout  {}",
//                     control_byte,
//                     mask_bytes.iter().map(|b| format!("{:02X}", b))
//                             .collect::<Vec<String>>()
//                             .join(" "),
//                     next_4.iter().map(|b| format!("{:02X}", b))
//                             .collect::<Vec<String>>()
//                             .join(" "),
//                     output[nums_decoded..(nums_decoded + 16)].iter()
//                             .map(|b| format!("{:08X}", b))
//                             .collect::<Vec<String>>()
//                             .join(" ")
//            );

            bytes_read += length as usize;
            nums_decoded += 4;
        }

        (nums_decoded, bytes_read)
    }
}
