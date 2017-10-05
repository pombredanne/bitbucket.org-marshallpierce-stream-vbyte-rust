extern crate x86intrin;

use std::cmp;

use self::x86intrin::{sse2, ssse3, m128i};

use super::tables;

/// Decoder using SSSE3 instructions.
pub struct Ssse3;

impl super::Decoder for Ssse3 {
    fn decode_quads(control_bytes: &[u8], encoded_nums: &[u8], output: &mut [u32],
                    control_bytes_to_decode: usize) -> (usize, usize) {
        debug_assert!(control_bytes_to_decode <= output.len());

        let mut bytes_read: usize = 0;
        let mut nums_decoded: usize = 0;

        // Decoding reads 16 bytes at a time from input, so we won't be able to read the last few
        // control byte's worth because they may be encoded at 1 byte per number, so we need 3
        // additional control bytes' worth of numbers to provide the extra 12 bytes.
        // However, if control_bytes_to_decode is short enough, we can decode all the requested numbers
        // because we'll have un-processed input to ensure we can read 16 bytes.
        let control_byte_limit = cmp::min(control_bytes_to_decode,
                                          control_bytes.len().saturating_sub(3));

        // need to ensure that we can copy 16 encoded bytes, so last few quads will be handled
        // by a slower loop
        for &control_byte in control_bytes[0..control_byte_limit].iter() {
            let length = tables::DECODE_LENGTH_PER_QUAD_TABLE[control_byte as usize];
            let mask_bytes = tables::X86_SSSE3_DECODE_SHUFFLE_TABLE[control_byte as usize];
            // we'll read 16 bytes from this always, so using explicit slice size to make sure it's
            // ok to read unsafe
            let next_4 = &encoded_nums[bytes_read..(bytes_read + 16)];

            let mask;
            let data;
            unsafe {
                // TODO load mask unaligned once https://github.com/rust-lang/rust/issues/33626
                // hits stable
                mask = sse2::mm_loadu_si128(mask_bytes.as_ptr() as *const m128i);
                data = sse2::mm_loadu_si128(next_4.as_ptr() as *const m128i);
            }

            let shuffled = ssse3::mm_shuffle_epi8(data, mask);

            unsafe {
                // using slice size to make sure it's ok to write 16 bytes = 4 u32s
                sse2::mm_storeu_si128(output[nums_decoded..(nums_decoded + 4)].as_ptr() as *mut m128i, shuffled);
            }

            bytes_read += length as usize;
            nums_decoded += 4;
        }

        (nums_decoded, bytes_read)
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;

    #[test]
    fn reads_all_requested_control_bytes_when_12_extra_input_bytes() {
        let nums: Vec<u32> = (0..64).map(|i| i * 100).collect();
        let mut encoded = Vec::new();
        let mut decoded: Vec<u32> = Vec::new();
        encoded.resize(nums.len() * 5, 0xFF);
        decoded.resize(nums.len(), 54321);

        encode::<Scalar>(&nums, &mut encoded);

        // 16 control bytes
        let control_bytes = &encoded[0..16];
        let encoded_nums = &encoded[16..];

        for control_bytes_to_decode in 0..14 {
            // requesting 13 or fewer control bytes decodes all requested bytes
            let (nums_decoded, bytes_read) = Ssse3::decode_quads(&control_bytes,
                                                                 &encoded_nums,
                                                                 &mut decoded,
                                                                 control_bytes_to_decode);
            assert_eq!(control_bytes_to_decode * 4, nums_decoded);
            assert_eq!(cumulative_encoded_len(&control_bytes[0..control_bytes_to_decode]),
                       bytes_read);
            assert_eq!(&nums[0..nums_decoded], &decoded[0..nums_decoded]);
            assert!(&decoded[nums_decoded..].iter().all(|&i| i == 54321_u32));
        }

        for control_bytes_to_decode in 14..17 {
            // requesting more than 13 gets capped to 13 because there may not be enough encoded
            // nums to read 16 bytes at a time
            let (nums_decoded, bytes_read) = Ssse3::decode_quads(&control_bytes,
                                                                 &encoded_nums,
                                                                 &mut decoded,
                                                                 control_bytes_to_decode);
            assert_eq!(13 * 4, nums_decoded);
            assert_eq!(cumulative_encoded_len(&control_bytes[0..(nums_decoded / 4)]),
                       bytes_read);
            assert_eq!(&nums[0..nums_decoded], &decoded[0..nums_decoded]);
            assert!(&decoded[nums_decoded..].iter().all(|&i| i == 54321_u32));
        }
    }
}
