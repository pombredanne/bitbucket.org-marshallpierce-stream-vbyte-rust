//! Encode and decode `u32`s with the Stream VByte format.
//!
//! ```
//! use stream_vbyte::*;
//!
//! let nums: Vec<u32> = (0..12345).collect();
//! let mut encoded_data = Vec::new();
//! // make some space to encode into
//! encoded_data.resize(5 * nums.len(), 0x0);
//!
//! // use Scalar implementation that works on any hardware
//! let encoded_len = encode::<Scalar>(&nums, &mut encoded_data);
//! println!("Encoded {} u32s into {} bytes", nums.len(), encoded_len);
//!
//! let mut decoded_nums = Vec::new();
//! decoded_nums.resize(nums.len(), 0);
//! // now decode
//! decode::<Scalar>(&encoded_data, nums.len(), &mut decoded_nums);
//!
//! assert_eq!(nums, decoded_nums);
//!
//! ```
//!
extern crate byteorder;

use std::cmp;
use byteorder::{ByteOrder, LittleEndian};

mod tables;

#[cfg(feature = "x86_ssse3")]
pub mod x86;

pub trait Encoder {
    /// Encode all input numbers that are in groups of 4.
    /// `control_bytes` will be exactly as long as the number of complete 4-number quads in `input`.
    /// Control bytes are written to `control_bytes` and encoded numbers to `encoded_nums`.
    /// Returns the total bytes written to `encoded_nums`.
    fn encode_quads(input: &[u32], control_bytes: &mut [u8], encoded_nums: &mut [u8]) -> usize;
}

pub trait Decoder {
    /// Decode encoded numbers in groups of 4 only.
    /// `control_bytes` will be exactly as long as the number of complete 4-number quads in `input`.
    /// Returns the number of numbers decoded and the total number of bytes read from
    /// `encoded_nums`.
    fn decode_quads(control_bytes: &[u8], encoded_nums: &[u8], output: &mut [u32]) -> (usize, usize);
}

/// Regular ol' byte shuffling.
/// Works on every platform, but it's not the quickest.
pub struct Scalar;

impl Encoder for Scalar {
    fn encode_quads(input: &[u32], control_bytes: &mut [u8], encoded_nums: &mut [u8]) -> usize {
        let mut bytes_written = 0;
        let mut nums_encoded = 0;

        for quads_encoded in 0..control_bytes.len() {
            let num0 = input[nums_encoded];
            let num1 = input[nums_encoded + 1];
            let num2 = input[nums_encoded + 2];
            let num3 = input[nums_encoded + 3];

            let len0 = encode_num_scalar(num0, &mut encoded_nums[bytes_written..]);
            let len1 = encode_num_scalar(num1, &mut encoded_nums[bytes_written + len0..]);
            let len2 = encode_num_scalar(num2, &mut encoded_nums[bytes_written + len0 + len1..]);
            let len3 = encode_num_scalar(num3, &mut encoded_nums[bytes_written + len0 + len1 + len2..]);

            // this is a few percent faster in my testing than using control_bytes.iter_mut()
            control_bytes[quads_encoded] = ((len0 - 1) | (len1 - 1) << 2 | (len2 - 1) << 4 | (len3 - 1) << 6) as u8;

            bytes_written += len0 + len1 + len2 + len3;
            nums_encoded += 4;
        }

        bytes_written
    }
}

impl Decoder for Scalar {
    fn decode_quads(_control_bytes: &[u8], _encoded_nums: &[u8], _output: &mut [u32]) -> (usize, usize) {
        // let the scalar loop decode the whole thing
        (0, 0)
    }
}

/// Encode the input slice into the output slice. The worst-case encoded length is 4 bytes per `u32`
/// plus another byte for every 4 `u32`s, including a trailing partial 4-some.
/// Returns the number of bytes written to the `output` slice.
pub fn encode<T: Encoder>(input: &[u32], output: &mut [u8]) -> usize {
    if input.len() == 0 {
        return 0;
    }

    let complete_quads = input.len() / 4;
    let leftover_numbers = input.len() % 4;
    let control_bytes_len = (input.len() + 3) / 4;

    let (control_bytes, encoded_bytes) = output.split_at_mut(control_bytes_len);

    let mut num_bytes_written = T::encode_quads(&input[..],
                                                &mut control_bytes[0..complete_quads],
                                                &mut encoded_bytes[..]);

    // last control byte, if there were leftovers
    if leftover_numbers > 0 {
        debug_assert!(leftover_numbers < 4);

        let mut control_byte = 0;
        let mut nums_encoded = complete_quads * 4;

        for i in 0..leftover_numbers {
            let num = input[nums_encoded];
            let len = encode_num_scalar(num, &mut encoded_bytes[num_bytes_written..]);

            control_byte |= ((len - 1) as u8) << (i * 2);

            num_bytes_written += len;
            nums_encoded += 1;
        }
        control_bytes[complete_quads] = control_byte;
    }

    control_bytes.len() + num_bytes_written
}

/// Decode `count` numbers from `input`, appending them to `output`. The `count` must be the same
/// as the number of items originally encoded.
/// Returns the number of bytes read from `input`.
pub fn decode<T: Decoder>(input: &[u8], count: usize, output: &mut [u32]) -> usize {
    // 4 numbers to decode per control byte
    let complete_quads = count / 4;
    let leftover_numbers = count % 4;
    let control_bytes_len = (count + 3) / 4;
    let control_bytes = &input[0..control_bytes_len];
    // data immediately follows control bytes
    let encoded_nums = &input[control_bytes_len..];

    let (mut nums_decoded, mut bytes_read) = T::decode_quads(&control_bytes[0..complete_quads],
                                                         &encoded_nums[..],
                                                         &mut output[..]);

    let control_bytes_decoded = nums_decoded / 4;

    // handle any remaining full quads
    for &control_byte in control_bytes[control_bytes_decoded..complete_quads].iter() {
        let (len0, len1, len2, len3) = tables::SCALAR_DECODE_TABLE[control_byte as usize];
        let len0 = len0 as usize;
        let len1 = len1 as usize;
        let len2 = len2 as usize;
        let len3 = len3 as usize;

        output[nums_decoded] = decode_num_scalar(len0, &encoded_nums[bytes_read..]);
        output[nums_decoded + 1] = decode_num_scalar(len1, &encoded_nums[bytes_read + len0..]);
        output[nums_decoded + 2] = decode_num_scalar(len2, &encoded_nums[bytes_read + len0 + len1..]);
        output[nums_decoded + 3] = decode_num_scalar(len3, &encoded_nums[bytes_read + len0 + len1 + len2..]);

        bytes_read += len0 + len1 + len2 + len3;
        nums_decoded += 4;
    }

    // incomplete quad, if any
    if leftover_numbers > 0 {
        debug_assert!(leftover_numbers < 4);

        let control_byte = control_bytes[complete_quads];
        let mut nums_decoded = 4 * complete_quads;

        for i in 0..leftover_numbers {
            let bitmask = 0x03 << (i * 2);
            let len = ((control_byte & bitmask) >> (i * 2)) as usize + 1;
            output[nums_decoded] = decode_num_scalar(len, &encoded_nums[bytes_read..]);
            nums_decoded += 1;
            bytes_read += len;
        }
    }

    control_bytes.len() + bytes_read
}

fn encode_num_scalar(num: u32, output: &mut [u8]) -> usize {
    // this will calculate 0_u32 as taking 0 bytes, so ensure at least 1 byte
    let len = cmp::max(1_usize, 4 - num.leading_zeros() as usize / 8);
    let mut buf = [0_u8; 4];
    LittleEndian::write_u32(&mut buf, num);

    for i in 0..len {
        output[i] = buf[i];
    }

    len
}

fn decode_num_scalar(len: usize, input: &[u8]) -> u32 {
    let mut buf = [0_u8; 4];
    &buf[0..len].copy_from_slice(&input[0..len]);

    LittleEndian::read_u32(&buf)
}

#[cfg(test)]
mod tests {
    extern crate rand;

    use super::*;
    use self::rand::Rng;

    #[test]
    fn encode_num_zero() {
        let mut buf = [0; 4];

        assert_eq!(1, encode_num_scalar(0, &mut buf));
        assert_eq!(&[0x00_u8, 0x00_u8, 0x00_u8, 0x00_u8], &buf);
    }

    #[test]
    fn encode_num_bottom_two_bytes() {
        let mut buf = [0; 4];

        assert_eq!(2, encode_num_scalar((1 << 16) - 1, &mut buf));
        assert_eq!(&[0xFF_u8, 0xFF_u8, 0x00_u8, 0x00_u8], &buf);
    }

    #[test]
    fn encode_num_middleish() {
        let mut buf = [0; 4];

        assert_eq!(3, encode_num_scalar((1 << 16) + 3, &mut buf));
        assert_eq!(&[0x03_u8, 0x00_u8, 0x01_u8, 0x00_u8], &buf);
    }

    #[test]
    fn encode_num_u32_max() {
        let mut buf = [0; 4];

        assert_eq!(4, encode_num_scalar(u32::max_value(), &mut buf));
        assert_eq!(&[0xFF_u8, 0xFF_u8, 0xFF_u8, 0xFF_u8], &buf);
    }

    #[test]
    fn decode_num_zero() {
        assert_eq!(0, decode_num_scalar(1, &vec![0, 0, 0, 0]));
    }

    #[test]
    fn decode_num_u32_max() {
        assert_eq!(u32::max_value(), decode_num_scalar(4, &vec![0xFF, 0xFF, 0xFF, 0xFF]));
    }

    #[test]
    fn decode_num_4_byte() {
        // 0x04030201
        assert_eq!((4 << 24) + (3 << 16) + (2 << 8) + 1, decode_num_scalar(4, &vec![1, 2, 3, 4]));
    }

    #[test]
    fn decode_num_3_byte() {
        // 0x04030201
        assert_eq!((3 << 16) + (2 << 8) + 1, decode_num_scalar(3, &vec![1, 2, 3]));
    }

    #[test]
    fn decode_num_2_byte() {
        // 0x04030201
        assert_eq!((2 << 8) + 1, decode_num_scalar(2, &vec![1, 2]));
    }

    #[test]
    fn decode_num_1_byte() {
        // 0x04030201
        assert_eq!(1, decode_num_scalar(1, &vec![1]));
    }

    #[test]
    fn encode_decode_roundtrip_random() {
        let mut rng = rand::weak_rng();

        let mut buf = [0; 4];
        for _ in 0..100_000 {
            let num: u32 = rng.gen();
            let len = encode_num_scalar(num, &mut buf);
            let decoded = decode_num_scalar(len, &buf);

            assert_eq!(num, decoded);
        }
    }
}
