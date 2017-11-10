use byteorder::{ByteOrder, LittleEndian};

pub mod cursor;

#[cfg(feature = "x86_ssse3")]
pub mod ssse3;

#[cfg(test)]
mod tests;

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
    /// `max_control_bytes_to_decode` may be greater than the number of control bytes remaining, in
    /// which case only the remaining control bytes will be decoded.
    ///
    /// Implementations may decode at most `max_control_bytes_to_decode` control bytes, but may decode
    /// fewer.
    ///
    /// `nums_already_decoded` is the number of numbers that have already been decoded in the
    /// `DecodeCursor.decode` invocation.
    ///
    /// Returns a tuple of the number of numbers decoded (always a multiple of 4; at most
    /// `4 * max_control_bytes_to_decode`) and the number of bytes read from `encoded_nums`.
    fn decode_quads<S: DecodeQuadSink<Self::DecodedQuad>>(
        control_bytes: &[u8],
        encoded_nums: &[u8],
        max_control_bytes_to_decode: usize,
        nums_already_decoded: usize,
        sink: &mut S,
    ) -> (usize, usize);
}

/// Receives numbers decoded via a Decoder in `DecodeCursor.decode_sink()`.
///
/// Since stream-vbyte is oriented around groups of 4 numbers, some decoders will expose decoded
/// numbers in some decoder-specific datatype. Or, if that is not applicable for a particular
/// `Decoder` implementation, `()` will be used, and all decoded numbers will instead be passed to
/// `DecodeSingleSink.on_number()`.
pub trait DecodeQuadSink<T>: DecodeSingleSink {
    /// `nums_decoded` is the number of numbers that have already been decoded before this quad
    /// in the current invocation of `DecodeCursor.decode_sink()`.
    fn on_quad(&mut self, quad: T, nums_decoded: usize);
}

/// Receives numbers decoded via a Decoder in `DecodeCursor.decode_sink()` that weren't handed to
/// `DecodeQuadSink.on_quad()`, whether because the `Decoder` implementation doesn't have a natural
/// quad representation, or because the numbers are part of a trailing partial quad.
pub trait DecodeSingleSink {
    /// `nums_decoded` is the number of numbers that have already been decoded before this number
    /// in the current invocation of `DecodeCursor.decode_sink()`.
    fn on_number(&mut self, num: u32, nums_decoded: usize);
}

impl<'a> DecodeSingleSink for SliceDecodeSink<'a> {
    #[inline]
    fn on_number(&mut self, num: u32, nums_decoded: usize) {
        self.output[nums_decoded] = num;
    }
}

/// A sink for writing to a slice.
///
/// Has to be public because it's in trait bounds on `decode()`.
#[doc(hidden)]
pub struct SliceDecodeSink<'a> {
    output: &'a mut [u32],
}

impl<'a> SliceDecodeSink<'a> {
    /// Create a new sink that wraps a slice.
    ///
    /// `output` must be at least as big as the
    fn new(output: &'a mut [u32]) -> SliceDecodeSink<'a> {
        SliceDecodeSink { output }
    }
}

/// Decode `count` numbers from `input`, writing them to `output`.
///
/// The `count` must be the same as the number of items originally encoded.
///
/// `output` must be at least of size 4, and must be large enough for all `count` numbers.
///
/// Returns the number of bytes read from `input`.
pub fn decode<D: Decoder>(input: &[u8], count: usize, output: &mut [u32]) -> usize
where
    for<'a> SliceDecodeSink<'a>: DecodeQuadSink<<D as Decoder>::DecodedQuad>,
{
    let mut cursor = cursor::DecodeCursor::new(&input, count);

    assert_eq!(
        count,
        cursor.decode_slice::<D>(output),
        "output buffer was not large enough"
    );

    cursor.input_consumed()
}

#[inline]
pub fn decode_num_scalar(len: usize, input: &[u8]) -> u32 {
    let mut buf = [0_u8; 4];
    &buf[0..len].copy_from_slice(&input[0..len]);

    LittleEndian::read_u32(&buf)
}
