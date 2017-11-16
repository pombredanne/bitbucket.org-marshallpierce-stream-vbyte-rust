//! Encode `u32`s to bytes and decode them back again with the Stream VByte format.
//!
//! To encode all your numbers to a `&[u8]`, or decode all your bytes to a `&[u32]`, see `encode()`
//! and `decode()` respectively. For more sophisticated decoding functionality, see `DecodeCursor`.
//!
//! There are two traits, `Encoder` and `Decoder`, that allow you to choose what logic to use in the
//! inner hot loops.
//!
//! A terminology note - Stream VByte groups encoded numbers into clusters of four, which are
//! referred to as "quads" in this project.
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
//! [raw-cpuid](https://crates.io/crates/raw-cpuid) or [auxv](https://crates.io/crates/auxv) will
//! probably be useful for runtime CPU feature detection.
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
//! # Examples
//!
//! Encode some numbers to bytes, then decode them in different ways.
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
//! let count = cursor.decode_slice::<Scalar>(&mut decoded_nums);
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
//! SIMD code uses unsafe internally because many of the SIMD intrinsics are unsafe. However, SIMD
//! intrinsics are used only on appropriately sized slices to essentially manually apply
//! slice index checking before use.
//!
//! Since this is human-maintained code, it could do the bounds checking incorrectly, of course. To
//! mitigate those risks, there are various forms of randomized testing in the test suite to shake
//! out any lurking bugs.
//!
//! The `Scalar` codec does not use unsafe.

extern crate byteorder;

mod tables;

mod scalar;
pub use scalar::Scalar;

pub mod x86;

mod encode;
pub use encode::{encode, Encoder};

mod decode;
pub use decode::{decode, DecodeQuadSink, DecodeSingleSink, Decoder, SliceDecodeSink};
pub use decode::cursor::DecodeCursor;

#[derive(Debug, PartialEq)]
struct EncodedShape {
    control_bytes_len: usize,
    complete_control_bytes_len: usize,
    leftover_numbers: usize,
}

fn encoded_shape(count: usize) -> EncodedShape {
    EncodedShape {
        control_bytes_len: (count + 3) / 4,
        complete_control_bytes_len: count / 4,
        leftover_numbers: count % 4,
    }
}

fn cumulative_encoded_len(control_bytes: &[u8]) -> usize {
    // sum could only overflow with an invalid encoding because the sum can be no larger than
    // the complete length of the encoded data, which fits in a usize
    control_bytes
        .iter()
        .map({
            |&b| tables::DECODE_LENGTH_PER_QUAD_TABLE[b as usize] as usize
        })
        .sum()
}

#[cfg(test)]
mod tests;
