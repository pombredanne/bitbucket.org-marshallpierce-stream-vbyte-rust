extern crate x86intrin;

use self::x86intrin::{m128i, sse2, sse41, ssse3};

use super::super::{tables, Encoder};

/// Encoder using SSE4.1 instructions.
pub struct Sse41;

const ONES: [u8; 16] = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1];
// multiplicand to achieve shifts by multiplication
const SHIFT: u32 = 1 | 1 << 9 | 1 << 18;
const SHIFTS: [u32; 4] = [SHIFT, SHIFT, SHIFT, SHIFT];
// translate 3-bit bytemaps into lane codes. Last 8 will never be used.
// 0 = 1 byte encoded num, 1 = 2 byte, etc.
// These are concatenated into the control byte, and also used to sum to find the total length.
// The ordering of these codes is determined by how the bytemap is calculated; see comments below.
#[cfg_attr(rustfmt, rustfmt_skip)]
const LANECODES: [u8; 16] = [
    0, 3, 2, 3,
    1, 3, 2, 3,
    128, 128, 128, 128,
    128, 128, 128, 128];
// gather high bytes from each lane, 2 copies
#[cfg_attr(rustfmt, rustfmt_skip)]
const GATHER_HI: [u8; 16] = [
    15, 11, 7, 3,
    15, 11, 7, 3,
    128, 128, 128, 128,
    128, 128, 128, 128];
// mul-shift magic
// concatenate 2-bit lane codes into high byte
const CONCAT: u32 = 1 | 1 << 10 | 1 << 20 | 1 << 30;
// sum lane codes in high byte
const SUM: u32 = 1 | 1 << 8 | 1 << 16 | 1 << 24;
const AGGREGATORS: [u32; 4] = [CONCAT, SUM, 0, 0];

impl Encoder for Sse41 {
    fn encode_quads(input: &[u32], control_bytes: &mut [u8], output: &mut [u8]) -> (usize, usize) {
        let mut nums_encoded: usize = 0;
        let mut bytes_encoded: usize = 0;

        // TODO these load unaligned once https://github.com/rust-lang/rust/issues/33626
        // hits stable
        let ones = unsafe { sse2::mm_loadu_si128(ONES.as_ptr() as *const m128i) };
        let shifts = unsafe { sse2::mm_loadu_si128(SHIFTS.as_ptr() as *const m128i) };
        let lanecodes = unsafe { sse2::mm_loadu_si128(LANECODES.as_ptr() as *const m128i) };
        let gather_hi = unsafe { sse2::mm_loadu_si128(GATHER_HI.as_ptr() as *const m128i) };
        let aggregators = unsafe { sse2::mm_loadu_si128(AGGREGATORS.as_ptr() as *const m128i) };

        // Encoding writes 16 bytes at a time, but if numbers are encoded with 1 byte each, that
        // means the last 3 quads could write past what is actually necessary. So, don't process
        // the last few control bytes.
        let control_byte_limit = control_bytes.len().saturating_sub(3);

        for control_byte in &mut control_bytes[0..control_byte_limit].iter_mut() {
            let to_encode = unsafe {
                sse2::mm_loadu_si128(input[nums_encoded..(nums_encoded + 4)].as_ptr()
                    as *const m128i)
            };

            // clamp each byte to 1 if nonzero
            let mins = sse2::mm_min_epu8(to_encode, ones);

            // Apply shifts to clamped bytes. e.g. u32::max_value() would be (little endian):
            // 00000001 00000001 00000001 00000001
            // and after multiplication aka shifting:
            // 00000001 00000011 00000111 00000111
            // 1 << 16 | 1 would be:
            // 00000001 00000000 00000001 00000000
            // and shifted:
            // 00000001 00000010 00000101 00000010
            // At most the bottom 3 bits of each byte will be set by shifting.
            // What we care about is the bottom 3 bits of the high byte in each num.
            // A 1-byte number (clamped to 0x01000000) will accumulate to 0x00 in the top byte
            // because there isn't a 3-byte shift to get that set bit into the top byte.
            // A 2-byte number (clamped to 0x00010000) will accumulate to 0x04 in the top byte
            // because the set bit would have been shifted 2 bytes + 2 bits higher.
            // A 3-byte number will have the 0x02 bit set in the top byte, and possibly the 0x04
            // bit set as well if the 2nd byte was non-zero.
            // A 4-byte number will have the 0x01 bit set in the top byte, and possibly 0x02 and
            // 0x04.
            // In summary, byte lengths -> high byte:
            // 1-byte -> 0x00
            // 2-byte -> 0x04
            // 3-byte -> 0x02, 0x06
            // 4-byte -> 0x01, 0x05, 0x03, 0x07
            let bytemaps = sse41::mm_mullo_epi32(mins, shifts);

            // Map high bytes to the corresponding lane codes. (Other bytes are mapped as well
            // but are not used.)
            let shuffled_lanecodes = ssse3::mm_shuffle_epi8(lanecodes, bytemaps);

            // Assemble 2 copies of the high byte from each of the 4 numbers.
            // The first copy will be used to calculate the control byte, the second the length.
            let hi_bytes = ssse3::mm_shuffle_epi8(shuffled_lanecodes, gather_hi);

            // use CONCAT to shift the lane code bits from bytes 0-3 into 1 byte (byte 3)
            // use SUM to sum lane code bits from bytes 4-7 into 1 byte (byte 7)
            let code_and_length = sse41::mm_mullo_epi32(hi_bytes, aggregators);

            let bytes = code_and_length.as_u8x16();
            let code = bytes.extract(3);
            let length = bytes.extract(7) + 4;

            let mask_bytes = tables::X86_ENCODE_SHUFFLE_TABLE[code as usize];
            let encode_mask = unsafe { sse2::mm_loadu_si128(mask_bytes.as_ptr() as *const m128i) };

            let encoded = ssse3::mm_shuffle_epi8(to_encode, encode_mask);

            unsafe {
                sse2::mm_storeu_si128(
                    output[bytes_encoded..(bytes_encoded + 16)].as_ptr() as *mut m128i,
                    encoded,
                );
            }

            *control_byte = code;

            bytes_encoded += length as usize;
            nums_encoded += 4;
        }

        (nums_encoded, bytes_encoded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::*;

    #[test]
    fn encodes_all_but_last_3_control_bytes() {
        // cover the whole byte length range
        let nums: Vec<u32> = (0..32).map(|i| 1 << i).collect();
        let mut encoded = Vec::new();
        let mut decoded: Vec<u32> = Vec::new();

        for control_bytes_len in 0..(nums.len() / 4 + 1) {
            encoded.clear();
            encoded.resize(nums.len() * 5, 0xFF);
            decoded.clear();
            decoded.resize(nums.len(), 54321);

            let (nums_encoded, bytes_written) = {
                let (control_bytes, num_bytes) = encoded.split_at_mut(control_bytes_len);

                Sse41::encode_quads(&nums[0..4 * control_bytes_len], control_bytes, num_bytes)
            };

            let control_bytes_written = nums_encoded / 4;

            assert_eq!(
                cumulative_encoded_len(&encoded[0..control_bytes_written]),
                bytes_written
            );

            // the last control byte written may not have populated all 16 output bytes with encoded
            // nums, depending on the length required. Any unused trailing bytes will have had 0
            // written, but nothing beyond that 16 should be touched.

            let length_before_final_control_byte =
                cumulative_encoded_len(&encoded[0..control_bytes_written.saturating_sub(1)]);

            let bytes_written_for_final_control_byte =
                bytes_written - length_before_final_control_byte;
            let trailing_zero_len = if control_bytes_written > 0 {
                16 - bytes_written_for_final_control_byte
            } else {
                0
            };

            assert!(&encoded[control_bytes_len + bytes_written
                         ..control_bytes_len + bytes_written
                             + trailing_zero_len]
                .iter()
                .all(|&i| i == 0));
            assert!(&encoded[control_bytes_len + bytes_written
                         + trailing_zero_len..]
                .iter()
                .all(|&i| i == 0xFF));
        }
    }
}
