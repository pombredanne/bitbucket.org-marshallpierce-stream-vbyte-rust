use std::cmp;

use byteorder::{ByteOrder, LittleEndian};

use encoded_shape;

#[cfg(feature = "x86_sse41")]
pub mod sse41;

/// Encode numbers to bytes.
pub trait Encoder {
    type EncodedQuad;

    /// Encode complete quads of input numbers.
    ///
    /// `control_bytes` will be exactly as long as the number of complete 4-number quads in `input`.
    ///
    /// Control bytes are written to `control_bytes` and encoded numbers to `output`.
    ///
    /// The provided `transformer` will be used on all input numbers prior to encoding.
    ///
    /// Implementations may choose to encode fewer than the full provided input, but any writes done
    /// must be for full quads.
    ///
    /// Implementations must not write to `output` outside of the area that will be populated by
    /// encoded numbers when all control bytes are processed..
    ///
    /// Returns the number of numbers encoded, the number of bytes written to `output`, and the
    /// `EncodeSingleTransformer` to use for the rest of the input.
    fn encode_quads<T: EncodeQuadTransformer<Self::EncodedQuad>>(
        input: &[u32],
        control_bytes: &mut [u8],
        output: &mut [u8],
    ) -> (usize, usize);
}

/// Transform numbers at encode time.
///
/// These are intended to be single-use: one transformer for each complete input.
///
/// This is not meant to be implemented exernally, but must be public because it is used in
/// `Encoder`.
#[doc(hidden)]
pub trait EncodeQuadTransformer<Q> {
    type SingleTransformer: EncodeSingleTransformer;
    /// Transform a quad of numbers.
    fn transform_quad(&mut self, quad: Q) -> Q;

    /// Once this is called, no more invocations of `transform_quad()` will be made.
    fn into_single_transformer(self) -> Self::SingleTransformer;
}

#[doc(hidden)]
pub trait EncodeSingleTransformer {
    /// Transform a single number.
    fn transform(&mut self, num: u32) -> u32;
}

/// Don't transform the input at all.
pub struct IdentityTransformer;

impl<T> EncodeQuadTransformer<T> for IdentityTransformer {
    type SingleTransformer = IdentityTransformer;

    #[inline]
    fn transform_quad(&mut self, quad: T) -> T {
        quad
    }

    #[inline]
    fn into_single_transformer(self) -> Self::SingleTransformer {
        IdentityTransformer
    }
}

impl EncodeSingleTransformer for IdentityTransformer {
    #[inline]
    fn transform(&mut self, num: u32) -> u32 {
        num
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
    encode_transformed::<E, IdentityTransformer>(input, output)
}

fn encode_transformed<E, T>(input: &[u32], output: &mut [u8]) -> usize
where
    E: Encoder,
    T: EncodeQuadTransformer<E::EncodedQuad>,
{
    if input.len() == 0 {
        return 0;
    }

    let shape = encoded_shape(input.len());

    let (control_bytes, encoded_bytes) = output.split_at_mut(shape.control_bytes_len);

    let (nums_encoded, mut num_bytes_written) = E::encode_quads::<IdentityTransformer>(
        &input[..],
        &mut control_bytes[0..shape.complete_control_bytes_len],
        &mut encoded_bytes[..],
    );

    // may be some input left, use Scalar to finish it
    let control_bytes_written = nums_encoded / 4;

    let (more_nums_encoded, more_bytes_written) = ::scalar::do_encode_quads(
        &input[nums_encoded..],
        &mut control_bytes[control_bytes_written..shape.complete_control_bytes_len],
        &mut encoded_bytes[num_bytes_written..],
    );

    num_bytes_written += more_bytes_written;

    debug_assert_eq!(
        shape.complete_control_bytes_len * 4,
        nums_encoded + more_nums_encoded
    );

    // last control byte, if there were leftovers
    if shape.leftover_numbers > 0 {
        let mut control_byte = 0;
        let mut nums_encoded = shape.complete_control_bytes_len * 4;

        for i in 0..shape.leftover_numbers {
            // TODO apply transformer
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

#[inline]
pub fn encode_num_scalar(num: u32, output: &mut [u8]) -> usize {
    // this will calculate 0_u32 as taking 0 bytes, so ensure at least 1 byte
    let len = cmp::max(1_usize, 4 - num.leading_zeros() as usize / 8);
    let mut buf = [0_u8; 4];
    LittleEndian::write_u32(&mut buf, num);

    for i in 0..len {
        output[i] = buf[i];
    }

    len
}

#[cfg(test)]
mod tests {
    use super::*;

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

}
