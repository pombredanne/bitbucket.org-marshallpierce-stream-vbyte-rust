//! Encode and decode `u32`s with the Stream VByte format.

extern crate byteorder;

use std::cmp;
use byteorder::{ByteOrder, BigEndian};

mod tables;

pub trait Encoder {
    /// Encode all input numbers that are in groups of 4.
    /// `control_bytes` will be exactly as long as the number of complete 4-number quads in `input`.
    /// Control bytes are written to `control_bytes` and encoded numbers to `encoded_nums`.
    /// Returns the total bytes written to `encoded_nums`.
    fn encode_quads(input: &[u32], control_bytes: &mut [u8], encoded_nums: &mut [u8]) -> usize;
}

pub trait Decoder {
    /// Decode all encoded numbers that are in groups of 4.
    /// `control_bytes` will be exactly as long as the number of complete 4-number quads in `input`.
    /// Returns the total number of bytes read from `encoded_nums`.
    fn decode_quads(control_bytes: &[u8], encoded_nums: &[u8], output: &mut [u32]) -> usize;
}

/// Regular ol' byte shuffling.
/// Works on every platform, but it's not the quickest.
pub struct GenericCodec;

impl Encoder for GenericCodec {
    fn encode_quads(input: &[u32], control_bytes: &mut [u8], encoded_nums: &mut [u8]) -> usize {
        let mut bytes_written = 0;
        let mut nums_encoded = 0;

        for quads_encoded in 0..control_bytes.len() {
            let num0 = input[nums_encoded];
            let num1 = input[nums_encoded + 1];
            let num2 = input[nums_encoded + 2];
            let num3 = input[nums_encoded + 3];

            let len0 = encode_num(num0, &mut encoded_nums[bytes_written..]);
            let len1 = encode_num(num1, &mut encoded_nums[bytes_written + len0..]);
            let len2 = encode_num(num2, &mut encoded_nums[bytes_written + len0 + len1..]);
            let len3 = encode_num(num3, &mut encoded_nums[bytes_written + len0 + len1 + len2..]);

            // this is a few percent faster in my testing than using control_bytes.iter_mut()
            control_bytes[quads_encoded] = ((len0 - 1) << 6 | (len1 - 1) << 4 | (len2 - 1) << 2 | (len3 - 1)) as u8;

            bytes_written += len0 + len1 + len2 + len3;
            nums_encoded += 4;
        }

        bytes_written
    }
}

impl Decoder for GenericCodec {
    fn decode_quads(control_bytes: &[u8], encoded_nums: &[u8], output: &mut [u32]) -> usize {
        let mut bytes_read = 0;
        let mut nums_decoded = 0;

        for &control_byte in control_bytes.iter() {
            let (len0, len1, len2, len3) = tables::DECODE_TABLE[control_byte as usize];
            let len0 = len0 as usize;
            let len1 = len1 as usize;
            let len2 = len2 as usize;
            let len3 = len3 as usize;

            output[nums_decoded] = decode_num(len0, &encoded_nums[bytes_read..]);
            output[nums_decoded + 1] = decode_num(len1, &encoded_nums[bytes_read + len0..]);
            output[nums_decoded + 2] = decode_num(len2, &encoded_nums[bytes_read + len0 + len1..]);
            output[nums_decoded + 3] = decode_num(len3, &encoded_nums[bytes_read + len0 + len1 + len2..]);

            bytes_read += len0 + len1 + len2 + len3;
            nums_decoded += 4;
        }

        bytes_read
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
            let len = encode_num(num, &mut encoded_bytes[num_bytes_written..]);

            control_byte |= ((len - 1) as u8) << (6 - i * 2);

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

    let mut bytes_read = T::decode_quads(&control_bytes[0..complete_quads],
                                         &encoded_nums[..],
                                         &mut output[..]);

    if leftover_numbers > 0 {
        debug_assert!(leftover_numbers < 4);

        let control_byte = control_bytes[complete_quads];
        let mut nums_decoded = 4 * complete_quads;

        for i in 0..leftover_numbers {
            let bitmask = 0xC0 >> (i * 2);
            let len = ((control_byte & bitmask) >> (6 - i * 2)) as usize + 1;
            output[nums_decoded] = decode_num(len, &encoded_nums[bytes_read..]);
            nums_decoded += 1;
            bytes_read += len;
        }
    }

    control_bytes.len() + bytes_read
}

fn encode_num(num: u32, output: &mut [u8]) -> usize {
    // this will calculate 0_u32 as taking 0 bytes, so ensure at least 1 byte
    let len = cmp::max(1_usize, 4 - num.leading_zeros() as usize / 8);
    let mut buf = [0_u8; 4];
    BigEndian::write_u32(&mut buf, num);
    let start_index_in_buf = 4 - len;

    for i in 0..len {
        output[i] = buf[start_index_in_buf + i];
    }

    len
}

fn decode_num(len: usize, input: &[u8]) -> u32 {
    let mut num: u32 = input[0] as u32;
    
    for &b in input[1..len].iter() {
        num <<= 8;
        num |= b as u32;
    }

    num
}

#[cfg(test)]
mod tests {
    extern crate rand;

    use super::*;
    use self::rand::Rng;

    #[test]
    fn encode_num_zero() {
        let mut buf = [0; 4];

        assert_eq!(1, encode_num(0, &mut buf));
        assert_eq!(&[0x00_u8, 0x00_u8, 0x00_u8, 0x00_u8], &buf);
    }

    #[test]
    fn encode_num_bottom_two_bytes() {
        let mut buf = [0; 4];

        assert_eq!(2, encode_num((1 << 16) - 1, &mut buf));
        assert_eq!(&[0xFF_u8, 0xFF_u8, 0x00_u8, 0x00_u8], &buf);
    }

    #[test]
    fn encode_num_middleish() {
        let mut buf = [0; 4];

        assert_eq!(3, encode_num((1 << 16) + 1, &mut buf));
        assert_eq!(&[0x01_u8, 0x00_u8, 0x01_u8, 0x00_u8], &buf);
    }

    #[test]
    fn encode_num_u32_max() {
        let mut buf = [0; 4];

        assert_eq!(4, encode_num(u32::max_value(), &mut buf));
        assert_eq!(&[0xFF_u8, 0xFF_u8, 0xFF_u8, 0xFF_u8], &buf);
    }

    #[test]
    fn decode_num_zero() {
        assert_eq!(0, decode_num(1, &vec![0, 0, 0, 0]));
    }

    #[test]
    fn decode_num_u32_max() {
        assert_eq!(u32::max_value(), decode_num(4, &vec![0xFF, 0xFF, 0xFF, 0xFF]));
    }

    #[test]
    fn encode_decode_roundtrip_random() {
        let mut rng = rand::weak_rng();

        let mut buf = [0; 4];
        for _ in 0..100_000 {
            let num: u32 = rng.gen();
            let len = encode_num(num, &mut buf);
            let decoded = decode_num(len, &buf);

            assert_eq!(num, decoded);
        }
    }
}
