use std::cmp;

use super::{Decoder, Scalar, encoded_shape, EncodedShape, decode_num_scalar, cumulative_encoded_len};

#[derive(Debug)]
pub struct DecodeCursor<'a> {
    control_bytes: &'a [u8],
    encoded_nums: &'a [u8],
    encoded_shape: EncodedShape,
    total_nums: usize,
    nums_decoded: usize,
    control_bytes_read: usize,
    encoded_bytes_read: usize
}

impl<'a> DecodeCursor<'a> {
    pub fn new(input: &'a [u8], count: usize) -> DecodeCursor<'a> {
        let shape = encoded_shape(count);

        DecodeCursor {
            control_bytes: &input[0..shape.control_bytes_len],
            encoded_nums: &input[shape.control_bytes_len..],
            encoded_shape: shape,
            total_nums: count,
            nums_decoded: 0,
            control_bytes_read: 0,
            encoded_bytes_read: 0
        }
    }

    /// Skip `to_skip` numbers. `to_skip` must be a multiple of 4, and must not be greater than the
    /// count of remaining numbers that are in complete blocks of 4.
    ///
    /// In other words, if you have 7 numbers remaining (a block of 4 and a partial block of 3), the
    /// only count you can skip is 4.
    fn skip(&mut self, to_skip: usize) {
        // TODO skip control bytes?
        assert_eq!(to_skip % 4, 0, "Must be a multiple of 4");
        let control_bytes_to_skip = to_skip / 4;
        assert!(self.control_bytes_read + control_bytes_to_skip
                        <= self.encoded_shape.complete_control_bytes_len,
                "Can't skip past the end of complete control bytes");

        let slice_to_skip = &self.control_bytes[self.control_bytes_read..(self.control_bytes_read + control_bytes_to_skip)];
        let skipped_encoded_len = cumulative_encoded_len(&slice_to_skip);

        self.control_bytes_read += control_bytes_to_skip;
        self.encoded_bytes_read += skipped_encoded_len;
    }

    /// Decode into the `output` buffer. The buffer must be at least of size 4.
    ///
    /// Returns the number of numbers decoded by this invocation, which may be less than the size
    /// of the buffer.
    pub fn decode<D: Decoder>(&mut self, output: &mut [u32]) -> usize {
        // TODO this is basically the top level `decode` function
        debug_assert!(output.len() >= 4);
        let start_nums_decoded = self.nums_decoded;

        // decode complete quads
        let complete_control_bytes =
            &self.control_bytes[self.control_bytes_read..self.encoded_shape.complete_control_bytes_len];
        // decode as much as we can fit
        let control_bytes_to_decode = output.len() / 4;

        let (primary_nums_decoded, primary_bytes_read) =
            D::decode_quads(complete_control_bytes,
                            &self.encoded_nums[self.encoded_bytes_read..],
                            output,
                            control_bytes_to_decode);

        self.encoded_bytes_read += primary_bytes_read;
        self.control_bytes_read += primary_nums_decoded / 4;
        self.nums_decoded += primary_nums_decoded;

        let mut remaining_output = &mut output[primary_nums_decoded..];
        // handle any remaining full quads if the provided Decoder did not finish the
        // remaining bytes in output buffer, or remaining control bytes, whichever is smaller
        let control_bytes_limit = cmp::min(remaining_output.len() / 4,
                                           self.encoded_shape.complete_control_bytes_len - self.control_bytes_read);
        let (more_nums_decoded, more_bytes_read) = Scalar::decode_quads(
            &self.control_bytes[self.control_bytes_read..self.encoded_shape.complete_control_bytes_len],
            &self.encoded_nums[self.encoded_bytes_read..],
            &mut remaining_output,
            control_bytes_limit);

        self.encoded_bytes_read += more_bytes_read;
        self.control_bytes_read += more_nums_decoded / 4;
        self.nums_decoded += more_nums_decoded;

        let remaining_output = &mut remaining_output[more_nums_decoded..];

        // decode incomplete quad, if we're at the end and there's room
        if self.control_bytes_read == self.encoded_shape.complete_control_bytes_len
                && remaining_output.len() >= self.encoded_shape.leftover_numbers
                && self.encoded_shape.leftover_numbers > 0 {
            debug_assert!(self.encoded_shape.leftover_numbers < 4);
            debug_assert_eq!(self.control_bytes_read, self.encoded_shape.complete_control_bytes_len);

            let control_byte = self.control_bytes[self.encoded_shape.complete_control_bytes_len];

            for i in 0..self.encoded_shape.leftover_numbers {
                // first num's length in low 2 bits, last in high 2 bits
                let bitmask = 0x03 << (i * 2);
                let len = ((control_byte & bitmask) >> (i * 2)) as usize + 1;
                remaining_output[i] = decode_num_scalar(len, &self.encoded_nums[self.encoded_bytes_read..]);
                self.nums_decoded += 1;
                self.encoded_bytes_read += len;
            }
        }

        self.nums_decoded - start_nums_decoded
    }

    /// Returns the total length of input scanned so far: the complete block of control bytes, plus
    /// any encoded numbers decoded.
    pub fn input_consumed(&self) -> usize {
        self.encoded_shape.control_bytes_len + self.encoded_bytes_read
    }

    /// Returns true iff there are more numbers to be decoded.
    pub fn has_more(&self) -> bool {
        self.nums_decoded < self.total_nums
    }
}
