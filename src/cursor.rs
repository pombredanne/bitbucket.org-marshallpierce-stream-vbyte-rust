use std::cmp;

use super::{Decoder, Scalar, encoded_shape, EncodedShape, decode_num_scalar, cumulative_encoded_len};

const MIN_DECODE_BUFFER_LEN: usize = 4;

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

    /// Skip `to_skip` numbers. `to_skip` must be a multiple of 4.
    fn skip(&mut self, to_skip: usize) {
        // TODO skip control bytes?
        assert_eq!(to_skip % 4, 0, "Must be a multiple of 4");

        // TODO what if skipping part of last quad when it's a partial?
        // (Not possible until we allow non-multiples-of-4 skips)

        let control_bytes_to_skip = to_skip / 4;

        // sum could only overflow with an invalid encoding because the sum can be no larger than
        // the complete length of the encoded data, which fits in a usize
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
        debug_assert!(output.len() >= MIN_DECODE_BUFFER_LEN);
        let start_nums_decoded = self.nums_decoded;

        // decode complete quads
        let complete_control_bytes = &self.control_bytes[self.control_bytes_read..self.encoded_shape.complete_control_bytes_len];
        // decode as much as we can fit
        let control_bytes_to_decode = output.len() / 4;

        let (primary_nums_decoded, primary_bytes_read) = D::decode_quads(complete_control_bytes,
                                                                         &self.encoded_nums[self.encoded_bytes_read..],
                                                                         output,
                                                                         control_bytes_to_decode);

        self.encoded_bytes_read += primary_bytes_read;
        self.control_bytes_read += primary_nums_decoded / 4;
        self.nums_decoded += primary_nums_decoded;

        let mut remaining_output = &mut output[primary_nums_decoded..];
        // handle any remaining full quads if the provided Decoder did not finish them all
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

#[cfg(test)]
mod tests {
    extern crate rand;

    use self::rand::Rng;

    use super::super::*;
    use super::super::tests::random_varint::RandomVarintEncodedLengthIter;
    use super::*;

    #[test]
    fn decode_cursor_random_decode_len_scalar() {
        decode_in_chunks_random_decode_len::<Scalar>();
    }

    #[cfg(feature = "x86_ssse3")]
    #[test]
    fn decode_cursor_random_decode_len_ssse3() {
        decode_in_chunks_random_decode_len::<x86::Ssse3>();
    }

    #[test]
    fn decode_cursor_every_decode_len_scalar() {
        decode_in_chunks_every_decode_len::<Scalar>()
    }

    #[cfg(feature = "x86_ssse3")]
    #[test]
    fn decode_cursor_every_decode_len_ssse3() {
        decode_in_chunks_every_decode_len::<x86::Ssse3>()
    }

    fn decode_in_chunks_every_decode_len<D: Decoder>() {
        let mut nums: Vec<u32> = Vec::new();
        let mut encoded = Vec::new();
        let mut decoded = Vec::new();
        let mut decoded_accum = Vec::new();
        let mut rng = rand::weak_rng();

        // 54 isn't magic, it's just a non-multiple-of-4
        for count in 0..100 {
            nums.clear();
            encoded.clear();
            decoded.clear();

            for i in RandomVarintEncodedLengthIter::new(rand::weak_rng()).take(count) {
                nums.push(i);
            }

            encoded.resize(count * 5, 0);
            let encoded_len = encode::<Scalar>(&nums, &mut encoded);

            // decode in several chunks, copying to accumulator
            let extra_slots = 100;

            // try every legal decode length (must be at least 16)
            for decode_len in MIN_DECODE_BUFFER_LEN..cmp::max(MIN_DECODE_BUFFER_LEN + 1, count + 1) {
                decoded_accum.clear();
                let mut cursor = DecodeCursor::new(&encoded[0..encoded_len], count);
                while cursor.has_more() {
                    let garbage = rng.gen();
                    decoded.clear();
                    decoded.resize(count + extra_slots, garbage);
                    let nums_decoded = cursor.decode::<D>(&mut decoded[0..decode_len]);
                    // the chunk is correct
                    assert_eq!(&nums[decoded_accum.len()..(decoded_accum.len() + nums_decoded)],
                               &decoded[0..nums_decoded]);
                    // beyond the chunk wasn't overwritten
                    for (i, &n) in decoded[nums_decoded..(count + extra_slots)].iter().enumerate() {
                        assert_eq!(garbage as u32, n, "index {}", i);
                    }

                    // accumulate for later comparison
                    for &n in &decoded[0..nums_decoded] {
                        decoded_accum.push(n);
                    }
                }

                assert_eq!(count, decoded_accum.len());
                assert_eq!(&nums[..], &decoded_accum[0..count]);
            }
        }
    }

    fn decode_in_chunks_random_decode_len<D: Decoder>() {
        let mut nums: Vec<u32> = Vec::new();
        let mut encoded = Vec::new();
        let mut decoded = Vec::new();
        let mut decoded_accum = Vec::new();
        let mut rng = rand::weak_rng();
        for _ in 0..100 {
            nums.clear();
            encoded.clear();
            decoded.clear();
            decoded_accum.clear();

            let count = rng.gen_range(0, 500);

            for i in RandomVarintEncodedLengthIter::new(rand::weak_rng()).take(count) {
                nums.push(i);
            }

            encoded.resize(count * 5, 0);
            let encoded_len = encode::<Scalar>(&nums, &mut encoded);

            // decode in several chunks, copying to accumulator
            let extra_slots = 100;
            let mut cursor = DecodeCursor::new(&encoded[0..encoded_len], count);
            while cursor.has_more() {
                let garbage = rng.gen();
                decoded.clear();
                decoded.resize(count + extra_slots, garbage);
                let decode_len: usize = rng.gen_range(MIN_DECODE_BUFFER_LEN, cmp::max(MIN_DECODE_BUFFER_LEN + 1, count / 3));
                let nums_decoded = cursor.decode::<D>(&mut decoded[0..decode_len]);
                // the chunk is correct
                assert_eq!(&nums[decoded_accum.len()..(decoded_accum.len() + nums_decoded)],
                           &decoded[0..nums_decoded]);
                // beyond the chunk wasn't overwritten
                for (i, &n) in decoded[nums_decoded..(count + extra_slots)].iter().enumerate() {
                    assert_eq!(garbage as u32, n, "index {}", i);
                }

                // accumulate for later comparison
                for &n in &decoded[0..nums_decoded] {
                    decoded_accum.push(n);
                }
            }

            assert_eq!(count, decoded_accum.len());

            assert_eq!(&nums[..], &decoded_accum[0..count]);
        }
    }
}
