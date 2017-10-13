//! Encode and decode `u32`s with the Stream VByte format.
//!
//! There are two traits, `Encoder` and `Decoder`, that allow you to choose what logic to use in the
//! inner hot loops.
//! 
//! # The simple, pretty fast way
//! 
//! Use `Scalar` for your `Encoder` and `Decoder`. It will work on all hardware, and is fast enough
//! that most people will probably never notice the time taken to encode/decode.
//! 
//! # The more complex, really fast way
//! 
//! If you can use nightly Rust (currently needed for SIMD) and you know which hardware you'll be
//! running on, or you can add runtime detection of CPU features, you can choose to use an
//! implementation that takes advantage of your hardware. Something like
//! [raw-cpuid](https://crates.io/crates/raw-cpuid) will probably be useful for runtime detection.
//! 
//! Performance numbers are calculated on an E5-1650v3 on encoding/decoding 1 million random numbers
//! at a time. You can run the benchmarks yourself to see how your hardware does.
//! 
//! Both `feature`s and `target_feature`s are used because `target_feature` is not in stable Rust
//! yet and this library should remain usable by stable Rust, so non-stable-friendly things are
//! hidden behind `feature`s.
//! 
//! ## Encoders
//! 
//! | Type           | Performance    | Hardware                                 | `target_feature` | `feature`   |
//! | -------------- | ---------------| ---------------------------------------- | ---------------- | ----------- |
//! | `Scalar`       | ≈140 million/s | All                                      | none             | none        |
//! | `x86::Sse41`   | ≈1 billion/s   | x86 with SSE4.1 (Penryn and above, 2008) | `sse4.1`         | `x86_sse41` |
//! 
//! ## Decoders
//! 
//! | Type           | Performance    | Hardware                                   | `target_feature` | `feature`   |
//! | -------------- | ---------------| ------------------------------------------ | ---------------- | ----------- |
//! | `Scalar`       | ≈140 million/s | All                                        | none             | none        |
//! | `x86::Ssse3`   | ≈2.7 billion/s | x86 with SSSE3 (Woodcrest and above, 2006) | `ssse3`          | `x86_ssse3` |
//! 
//! If you have a modern x86 and you want to use the all SIMD accelerated versions, you would use
//! `target_feature` in a compiler invocation like this:
//! 
//! ```sh
//! RUSTFLAGS='-C target-feature=+ssse3,+sse4.1' cargo ...
//! ```
//! 
//! Meanwhile, `feature`s for your dependency on this crate are specified
//! [in your project's Cargo.toml](http://doc.crates.io/manifest.html#the-features-section).
//! 
//! # Example
//!
//! ```
//! use stream_vbyte::*;
//!
//! let nums: Vec<u32> = (0..12_345).collect();
//! let mut encoded_data = Vec::new();
//! // make some space to encode into
//! encoded_data.resize(5 * nums.len(), 0x0);
//!
//! // use Scalar implementation that works on any hardware
//! let encoded_len = encode::<Scalar>(&nums, &mut encoded_data);
//! println!("Encoded {} u32s into {} bytes", nums.len(), encoded_len);
//!
//! // decode all the numbers at once
//! let mut decoded_nums = Vec::new();
//! decoded_nums.resize(nums.len(), 0);
//! let bytes_decoded = decode::<Scalar>(&encoded_data, nums.len(), &mut decoded_nums);
//! assert_eq!(nums, decoded_nums);
//! assert_eq!(encoded_len, bytes_decoded);
//!
//! // or maybe you want to skip some of the numbers while decoding
//! decoded_nums.clear();
//! decoded_nums.resize(nums.len(), 0);
//! let mut cursor = DecodeCursor::new(&encoded_data, nums.len());
//! cursor.skip(10_000);
//! let count = cursor.decode::<Scalar>(&mut decoded_nums);
//! assert_eq!(12_345 - 10_000, count);
//! assert_eq!(&nums[10_000..], &decoded_nums[0..count]);
//! assert_eq!(encoded_len, cursor.input_consumed());
//! ```
//!
//! # Panics
//!
//! If you use undersized slices (e.g. encoding 10 numbers into 5 bytes), you will get the normal
//! slice bounds check panics.
//!
//! # Safety
//!
//! SIMD code uses unsafe internally because many of the SIMD intrinsics are unsafe.
//!
//! The `Scalar` codec does not use unsafe.
//!
//!
extern crate byteorder;

use std::cmp;
use byteorder::{ByteOrder, LittleEndian};

mod tables;
mod cursor;

pub use cursor::DecodeCursor;

mod scalar;

pub use scalar::Scalar;

#[path = "x86/x86.rs"]
pub mod x86;

/// Encode numbers to bytes.
pub trait Encoder {
    /// Encode complete quads of input numbers.
    ///
    /// `control_bytes` will be exactly as long as the number of complete 4-number quads in `input`.
    ///
    /// Control bytes are written to `control_bytes` and encoded numbers to `output`.
    ///
    /// Implementations may choose to encode fewer than the full provided input, but any writes done
    /// must be for full quads.
    ///
    /// Implementations must not write to `output` outside of the area that will be populated by
    /// encoded numbers when all control bytes are processed..
    ///
    /// Returns the number of numbers encoded and the number of bytes written to `output`.
    fn encode_quads(input: &[u32], control_bytes: &mut [u8], output: &mut [u8]) -> (usize, usize);
}

/// Decode bytes to numbers.
pub trait Decoder {
    type DecodedQuad;

    /// Decode encoded numbers in complete quads.
    ///
    /// Only control bytes with 4 corresponding encoded numbers will be provided as input (i.e. no
    /// trailing partial quad).
    ///
    /// `control_bytes` are the control bytes that correspond to `encoded_nums`.
    ///
    /// `control_bytes_to_decode * 4` may be greater than the number of control bytes remaining.
    ///
    /// Implementations may decode at most `control_bytes_to_decode` control bytes, but may decode
    /// fewer.
    ///
    /// Returns the number of numbers decoded and the number of bytes read from
    /// `encoded_nums`.
    fn decode_quads<S: DecodeSink<Self::DecodedQuad>>(control_bytes: &[u8], encoded_nums: &[u8],
                                                      control_bytes_to_decode: usize, sink: &mut S) -> (usize, usize);
}

/// Receives numbers decoded by a Decoder.
pub trait DecodeSink<T> {
    fn on_quad(&mut self, quad: T, nums_decoded: usize);

    fn on_number(&mut self, num: u32, nums_decoded: usize);
}

/// A sink for writing to a slice.
///
/// `output` must be big enough for all complete quads in the input to be written to.
pub struct SliceDecodeSink<'a> {
    output: &'a mut [u32]
}

impl<'a> SliceDecodeSink<'a> {
    fn new(output: &'a mut [u32]) -> SliceDecodeSink<'a> {
        SliceDecodeSink {
            output
        }
    }
}

/// Encode the `input` slice into the `output` slice.
///
/// If you don't have specific knowledge of the input that would let you determine the encoded
/// length ahead of time, make `output` 5x as long as `input`. The worst-case encoded length is 4
/// bytes per `u32` plus another byte for every 4 `u32`s, including any trailing partial 4-some.
///
/// Returns the number of bytes written to the `output` slice.
pub fn encode<E: Encoder>(input: &[u32], output: &mut [u8]) -> usize {
    if input.len() == 0 {
        return 0;
    }

    let shape = encoded_shape(input.len());

    let (control_bytes, encoded_bytes) = output.split_at_mut(shape.control_bytes_len);

    let (nums_encoded, mut num_bytes_written) = E::encode_quads(&input[..],
                                                                &mut control_bytes[0..shape.complete_control_bytes_len],
                                                                &mut encoded_bytes[..]);

    // may be some input left, use Scalar to finish it
    let control_bytes_written = nums_encoded / 4;

    let (more_nums_encoded, more_bytes_written) =
        Scalar::encode_quads(&input[nums_encoded..],
                             &mut control_bytes[control_bytes_written..shape.complete_control_bytes_len],
                             &mut encoded_bytes[num_bytes_written..]);

    num_bytes_written += more_bytes_written;

    debug_assert_eq!(shape.complete_control_bytes_len * 4, nums_encoded + more_nums_encoded);

    // last control byte, if there were leftovers
    if shape.leftover_numbers > 0 {
        let mut control_byte = 0;
        let mut nums_encoded = shape.complete_control_bytes_len * 4;

        for i in 0..shape.leftover_numbers {
            let num = input[nums_encoded];
            let len = encode_num_scalar(num, &mut encoded_bytes[num_bytes_written..]);

            control_byte |= ((len - 1) as u8) << (i * 2);

            num_bytes_written += len;
            nums_encoded += 1;
        }
        control_bytes[shape.complete_control_bytes_len] = control_byte;
    }

    control_bytes.len() + num_bytes_written
}

/// Decode `count` numbers from `input`, writing them to `output`.
///
/// The `count` must be the same as the number of items originally encoded.
///
/// `output` must be at least of size 4, and must be large enough for all `count` numbers.
///
/// Returns the number of bytes read from `input`.
pub fn decode<D: Decoder>(input: &[u8], count: usize, output: &mut [u32]) -> usize
    where for <'a> SliceDecodeSink<'a>: DecodeSink<<D as Decoder>::DecodedQuad> {
    let mut cursor = DecodeCursor::new(&input, count);

    assert_eq!(count, cursor.decode::<D>(output), "output buffer was not large enough");

    cursor.input_consumed()
}

#[derive(Debug, PartialEq)]
struct EncodedShape {
    control_bytes_len: usize,
    complete_control_bytes_len: usize,
    leftover_numbers: usize
}

fn encoded_shape(count: usize) -> EncodedShape {
    EncodedShape {
        control_bytes_len: (count + 3) / 4,
        complete_control_bytes_len: count / 4,
        leftover_numbers: count % 4
    }
}

#[inline]
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

#[inline]
fn decode_num_scalar(len: usize, input: &[u8]) -> u32 {
    let mut buf = [0_u8; 4];
    &buf[0..len].copy_from_slice(&input[0..len]);

    LittleEndian::read_u32(&buf)
}


fn cumulative_encoded_len(control_bytes: &[u8]) -> usize {
    // sum could only overflow with an invalid encoding because the sum can be no larger than
    // the complete length of the encoded data, which fits in a usize
    control_bytes.iter()
            .map({ |&b| tables::DECODE_LENGTH_PER_QUAD_TABLE[b as usize] as usize })
            .sum()
}

#[cfg(test)]
mod tests;
