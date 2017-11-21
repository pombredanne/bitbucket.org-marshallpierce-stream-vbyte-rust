extern crate rand;
extern crate stream_vbyte;

#[cfg(feature = "x86_ssse3")]
extern crate stdsimd;

use std::cmp;

use self::rand::Rng;

use stream_vbyte::*;

#[path = "../src/random_varint.rs"]
mod random_varint;

use random_varint::*;

const QUAD_LEN: usize = 4;

#[test]
fn decode_cursor_random_decode_len_scalar() {
    do_decode_cursor_slice_random_decode_len::<Scalar>();
}

#[cfg(feature = "x86_ssse3")]
#[test]
fn decode_cursor_random_decode_len_ssse3() {
    do_decode_cursor_slice_random_decode_len::<x86::Ssse3>();
}

#[test]
fn decode_cursor_every_decode_len_scalar() {
    do_decode_cursor_slice_every_decode_len::<Scalar>()
}

#[cfg(feature = "x86_ssse3")]
#[test]
fn decode_cursor_every_decode_len_ssse3() {
    do_decode_cursor_slice_every_decode_len::<x86::Ssse3>()
}

#[test]
fn decode_cursor_skip_from_start_scalar() {
    do_decode_cursor_skip_every_allowable_len_from_start::<Scalar>();
}

#[cfg(feature = "x86_ssse3")]
#[test]
fn decode_cursor_skip_from_start_ssse3() {
    do_decode_cursor_skip_every_allowable_len_from_start::<x86::Ssse3>();
}

#[test]
fn decode_cursor_skip_every_allowable_len_between_decodes_scalar() {
    do_decode_cursor_skip_every_allowable_len_between_decodes::<Scalar>();
}

#[cfg(feature = "x86_ssse3")]
#[test]
fn decode_cursor_skip_every_allowable_len_between_decodes_ssse3() {
    do_decode_cursor_skip_every_allowable_len_between_decodes::<x86::Ssse3>();
}

#[test]
fn decode_cursor_slice_input_only_partial_quad_decodes_all_scalar() {
    do_decode_cursor_slice_input_only_partial_quad_decodes_all::<Scalar>()
}

#[cfg(feature = "x86_ssse3")]
#[test]
fn decode_cursor_slice_input_only_partial_quad_decodes_all_ssse3() {
    do_decode_cursor_slice_input_only_partial_quad_decodes_all::<x86::Ssse3>()
}

#[test]
fn decode_cursor_sink_decode_entire_input_emits_entire_input_including_trailing_partial_quad_scalar(
) {
    do_decode_cursor_sink_decode_entire_input_emits_entire_input_including_trailing_partial_quad::<
        Scalar,
    >()
}

#[cfg(feature = "x86_ssse3")]
#[test]
fn decode_cursor_sink_decode_entire_input_emits_entire_input_including_trailing_partial_quad_ssse3()
{
    do_decode_cursor_sink_decode_entire_input_emits_entire_input_including_trailing_partial_quad::<
        x86::Ssse3,
    >()
}

#[test]
fn decode_cursor_sink_decode_partial_input_from_beginning_emits_complete_quads_only_scalar() {
    do_decode_cursor_sink_decode_partial_input_from_beginning_emits_complete_quads_only::<Scalar>()
}

#[cfg(feature = "x86_ssse3")]
#[test]
fn decode_cursor_sink_decode_partial_input_from_beginning_emits_complete_quads_only_ssse3() {
    do_decode_cursor_sink_decode_partial_input_from_beginning_emits_complete_quads_only::<
        x86::Ssse3,
    >()
}

#[test]
fn decode_cursor_sink_decode_in_chunks_emits_complete_quads_until_end_scalar() {
    do_decode_cursor_sink_decode_in_chunks_emits_complete_quads_until_end::<Scalar>()
}

#[cfg(feature = "x86_ssse3")]
#[test]
fn decode_cursor_sink_decode_in_chunks_emits_complete_quads_until_end_ssse3() {
    do_decode_cursor_sink_decode_in_chunks_emits_complete_quads_until_end::<x86::Ssse3>()
}

#[test]
fn decode_cursor_sink_decode_in_chunks_smaller_than_first_quad_decodes_0_nums_scalar() {
    do_decode_cursor_sink_decode_in_chunks_smaller_than_first_quad_decodes_0_nums::<Scalar>()
}

#[cfg(feature = "x86_ssse3")]
#[test]
fn decode_cursor_sink_decode_in_chunks_smaller_than_first_quad_decodes_0_nums_ssse3() {
    do_decode_cursor_sink_decode_in_chunks_smaller_than_first_quad_decodes_0_nums::<x86::Ssse3>()
}

#[test]
fn decode_cursor_sink_decode_final_chunk_partially_includes_leftovers_decodes_only_complete_quads_scalar(
) {
    do_decode_cursor_sink_decode_final_chunk_partially_includes_leftovers_decodes_only_complete_quads::<Scalar>()
}

#[cfg(feature = "x86_ssse3")]
#[test]
fn decode_cursor_sink_decode_final_chunk_partially_includes_leftovers_decodes_only_complete_quads_ssse3(
) {
    do_decode_cursor_sink_decode_final_chunk_partially_includes_leftovers_decodes_only_complete_quads::<x86::Ssse3>()
}

#[test]
fn decode_cursor_sink_decode_after_finishing_input_decodes_0_numbers_scalar() {
    do_decode_cursor_sink_decode_after_finishing_input_decodes_0_numbers::<Scalar>()
}

#[cfg(feature = "x86_ssse3")]
#[test]
fn decode_cursor_sink_decode_after_finishing_input_decodes_0_numbers_ssse3() {
    do_decode_cursor_sink_decode_after_finishing_input_decodes_0_numbers::<x86::Ssse3>()
}

fn do_decode_cursor_slice_every_decode_len<D: Decoder>()
where
    for<'a> SliceDecodeSink<'a>: DecodeQuadSink<<D as Decoder>::DecodedQuad>,
{
    let mut nums: Vec<u32> = Vec::new();
    let mut encoded = Vec::new();
    let mut decoded = Vec::new();
    let mut decoded_accum = Vec::new();
    let mut rng = rand::weak_rng();

    for count in 0..100 {
        nums.clear();
        encoded.clear();
        decoded.clear();

        for i in RandomVarintEncodedLengthIter::new(rand::weak_rng()).take(count) {
            nums.push(i);
        }

        encoded.resize(count * 5, 0);
        let encoded_len = encode::<Scalar>(&nums, &mut encoded);

        let extra_slots = 100;

        for decode_len in cmp::min(count, QUAD_LEN)..(count + 1) {
            decoded_accum.clear();
            let mut cursor = DecodeCursor::new(&encoded[0..encoded_len], count);
            while cursor.has_more() {
                let garbage = rng.gen();
                decoded.clear();
                decoded.resize(count + extra_slots, garbage);
                let nums_decoded = cursor.decode_slice::<D>(&mut decoded[0..decode_len]);

                if cursor.has_more() {
                    // if we're in the middle somewhere, we shouldn't fall short by any more than 3
                    // (partial quad size)
                    assert!(
                        decode_len - nums_decoded <= 3,
                        "{} - {} = {}",
                        decode_len,
                        nums_decoded,
                        decode_len - nums_decoded
                    );
                }

                // the chunk is correct
                assert_eq!(
                    &nums[decoded_accum.len()..(decoded_accum.len() + nums_decoded)],
                    &decoded[0..nums_decoded]
                );
                // beyond the chunk wasn't overwritten
                for (i, &n) in decoded[nums_decoded..(count + extra_slots)]
                    .iter()
                    .enumerate()
                {
                    assert_eq!(garbage, n, "index {}", i);
                }

                // accumulate for later comparison
                for &n in &decoded[0..nums_decoded] {
                    decoded_accum.push(n);
                }
            }

            assert_eq!(encoded_len, cursor.input_consumed());
            assert_eq!(count, decoded_accum.len());
            assert_eq!(&nums, &decoded_accum);
        }
    }
}

fn do_decode_cursor_slice_random_decode_len<D: Decoder>()
where
    for<'a> SliceDecodeSink<'a>: DecodeQuadSink<<D as Decoder>::DecodedQuad>,
{
    let mut nums: Vec<u32> = Vec::new();
    let mut encoded = Vec::new();
    let mut decoded = Vec::new();
    let mut decoded_accum = Vec::new();
    let mut rng = rand::weak_rng();
    for _ in 0..10_000 {
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
            let decode_len: usize = rng.gen_range(QUAD_LEN, cmp::max(QUAD_LEN + 1, count + 1));
            let nums_decoded = cursor.decode_slice::<D>(&mut decoded[0..decode_len]);
            // the chunk is correct
            assert_eq!(
                &nums[decoded_accum.len()..(decoded_accum.len() + nums_decoded)],
                &decoded[0..nums_decoded]
            );
            // beyond the chunk wasn't overwritten
            for (i, &n) in decoded[nums_decoded..(count + extra_slots)]
                .iter()
                .enumerate()
            {
                assert_eq!(garbage as u32, n, "index {}", i);
            }

            // accumulate for later comparison
            for &n in &decoded[0..nums_decoded] {
                decoded_accum.push(n);
            }
        }

        assert_eq!(encoded_len, cursor.input_consumed());
        assert_eq!(count, decoded_accum.len());
        assert_eq!(&nums[..], &decoded_accum[0..count]);
    }
}

fn do_decode_cursor_skip_every_allowable_len_from_start<D: Decoder>()
where
    for<'a> SliceDecodeSink<'a>: DecodeQuadSink<<D as Decoder>::DecodedQuad>,
{
    let mut nums: Vec<u32> = Vec::new();
    let mut encoded = Vec::new();
    let mut decoded = Vec::new();
    let mut rng = rand::weak_rng();

    for count in 0..100_usize {
        nums.clear();
        encoded.clear();
        decoded.clear();

        for i in 0..count {
            nums.push(i as u32);
        }

        encoded.resize(count * 5, 0);
        let encoded_len = encode::<Scalar>(&nums, &mut encoded);

        let extra_slots = 100;

        // skips must be divisible by 4
        for skip_len in (0..(count / QUAD_LEN + 1)).map(|i| i * QUAD_LEN) {
            let mut cursor = DecodeCursor::new(&encoded[0..encoded_len], count);

            cursor.skip(skip_len);

            let garbage = rng.gen();
            decoded.clear();
            decoded.resize(count + extra_slots, garbage);
            let nums_decoded = cursor.decode_slice::<D>(&mut decoded);

            assert!(!cursor.has_more());
            // decoded all the remaining numbers
            assert_eq!(count - skip_len, nums_decoded);
            // the chunk is correct
            assert_eq!(&nums[skip_len..], &decoded[0..nums_decoded]);
            // beyond the chunk wasn't overwritten
            for (i, &n) in decoded[nums_decoded..(count + extra_slots)]
                .iter()
                .enumerate()
            {
                assert_eq!(garbage as u32, n, "index {}", i);
            }

            assert_eq!(encoded_len, cursor.input_consumed());
        }
    }
}

fn do_decode_cursor_slice_input_only_partial_quad_decodes_all<D: Decoder>()
where
    for<'a> SliceDecodeSink<'a>: DecodeQuadSink<<D as Decoder>::DecodedQuad>,
{
    let mut nums: Vec<u32> = Vec::new();
    let mut encoded = Vec::new();
    let mut decoded = Vec::new();
    let mut rng = rand::weak_rng();

    for count in 0..4 {
        nums.clear();
        encoded.clear();
        decoded.clear();

        let encoded_len = prepare_offset_nums(count, 1000, &mut nums, &mut encoded);

        let extra_slots = 100;

        let garbage = rng.gen();
        decoded.clear();
        decoded.resize(count + extra_slots, garbage);

        let mut cursor = DecodeCursor::new(&encoded, count);
        let nums_decoded = cursor.decode_slice::<D>(&mut decoded[0..count]);

        assert!(!cursor.has_more());
        assert_eq!(count, nums_decoded);
        // the chunk is correct
        assert_eq!(&nums[..], &decoded[0..nums_decoded]);
        // beyond the chunk wasn't overwritten
        for (i, &n) in decoded[nums_decoded..(count + extra_slots)]
            .iter()
            .enumerate()
        {
            assert_eq!(garbage as u32, n, "index {}", i);
        }

        assert_eq!(encoded_len, cursor.input_consumed());
    }
}


fn do_decode_cursor_skip_every_allowable_len_between_decodes<D: Decoder>()
where
    for<'a> SliceDecodeSink<'a>: DecodeQuadSink<<D as Decoder>::DecodedQuad>,
{
    let mut nums: Vec<u32> = Vec::new();
    let mut encoded = Vec::new();
    let mut decoded = Vec::new();
    let mut rng = rand::weak_rng();

    for count in 0..50_usize {
        nums.clear();
        encoded.clear();
        decoded.clear();

        for i in 0..count {
            nums.push(i as u32);
        }

        encoded.resize(count * 5, 0);
        let encoded_len = encode::<Scalar>(&nums, &mut encoded);

        // The looping here is a little weird because we don't want to try to predict how many nums
        // will get decoded when we request, say 5 numbers. Will that be 4 because that 5th number
        // is part of a complete quad, or 5 because there are only 5 numbers so we do the last one
        // as part of the normal partial trailing quad handling.
        // Also, we can't roll cursors back (yet) so we have to do each decode-skip-decode triple
        // in one shot rather than re-trying various skips after the first decode.

        'dec1: for initial_decode_len in 0..(count + 1) {
            // skips must be divisible by 4
            'skip: for skip_len in (0..count / 4).map(|i| i * QUAD_LEN) {
                'dec2: for final_decode_len in 0..count {
                    decoded.clear();

                    let garbage = rng.gen();
                    let extra_slots = 100;
                    decoded.resize(count + extra_slots, garbage);

                    // 1: decode a bit
                    let mut cursor = DecodeCursor::new(&encoded[0..encoded_len], count);
                    let initial_decoded_nums =
                        cursor.decode_slice::<D>(&mut decoded[0..initial_decode_len]);
                    assert_eq!(
                        &nums[0..initial_decoded_nums],
                        &decoded[0..initial_decoded_nums]
                    );
                    for (i, &n) in decoded[initial_decoded_nums..].iter().enumerate() {
                        assert_eq!(garbage, n, "index {}", i);
                    }

                    if initial_decoded_nums + skip_len > count {
                        // skip has gotten too big; go to next enclosing loop
                        break 'skip;
                    }

                    // 2: skip a bit
                    cursor.skip(skip_len);

                    // won't underflow because we checked above
                    let nums_after_skip = count - initial_decoded_nums - skip_len;

                    // 3: decode some more
                    let garbage = rng.gen();
                    decoded.clear();
                    decoded.resize(count + extra_slots, garbage);
                    let final_decoded_nums =
                        cursor.decode_slice::<D>(&mut decoded[0..final_decode_len]);

                    // the count of what was decoded was correct

                    if final_decode_len >= nums_after_skip {
                        // should have decoded all
                        assert_eq!(nums_after_skip, final_decoded_nums)
                    } else if final_decode_len < QUAD_LEN {
                        // smaller than nums_after_skip, and a partial quad, so decode nothing
                        assert_eq!(0, final_decoded_nums);
                    } else {
                        // couldn't decode everything, but at least QUAD_LEN, so should be
                        // n * QUAD_LEN
                        assert_eq!(
                            final_decode_len - final_decode_len % QUAD_LEN,
                            final_decoded_nums
                        );
                    }

                    // the decoded data is correct
                    assert_eq!(
                        &nums[(initial_decoded_nums + skip_len)
                                  ..(initial_decoded_nums + skip_len + final_decoded_nums)],
                        &decoded[0..final_decoded_nums]
                    );
                    // beyond the chunk wasn't overwritten
                    for (i, &n) in decoded[final_decoded_nums..(count + extra_slots)]
                        .iter()
                        .enumerate()
                    {
                        assert_eq!(garbage as u32, n, "index {}", i);
                    }
                }
            }
        }
    }
}

fn do_decode_cursor_sink_decode_entire_input_emits_entire_input_including_trailing_partial_quad<
    D: Decoder,
>()
where
    TupleSink: DecodeQuadSink<<D as Decoder>::DecodedQuad>,
{
    let mut nums: Vec<u32> = Vec::new();
    let mut encoded = Vec::new();
    let mut expected = Vec::new();

    for len in 0..100 {
        nums.clear();
        encoded.clear();
        expected.clear();

        for num in 0..len {
            expected.push((num as usize, num as u32 + 1000));
        }

        prepare_offset_nums(len, 1000, &mut nums, &mut encoded);

        let mut cursor = DecodeCursor::new(&encoded, len);

        let mut sink = TupleSink::new();
        let nums_decoded = cursor.decode_sink::<D, _>(&mut sink, len);

        assert_eq!(len, nums_decoded);
        assert_eq!(expected, sink.tuples);
    }
}

fn do_decode_cursor_sink_decode_partial_input_from_beginning_emits_complete_quads_only<D: Decoder>()
where
    TupleSink: DecodeQuadSink<<D as Decoder>::DecodedQuad>,
{
    let mut nums: Vec<u32> = Vec::new();
    let mut encoded = Vec::new();
    let mut expected = Vec::new();

    for len in 0..100 {
        nums.clear();
        encoded.clear();

        prepare_offset_nums(len, 1000, &mut nums, &mut encoded);

        for partial_len in 0..len {
            expected.clear();

            let complete_quad_len = partial_len - (partial_len % QUAD_LEN);

            for num in 0..complete_quad_len {
                expected.push((num as usize, num as u32 + 1000));
            }

            let mut cursor = DecodeCursor::new(&encoded, len);

            let mut sink = TupleSink::new();
            let nums_decoded = cursor.decode_sink::<D, _>(&mut sink, partial_len);

            assert_eq!(
                complete_quad_len,
                nums_decoded,
                "len {} partial len {}",
                len,
                partial_len
            );
            assert_eq!(expected, sink.tuples);
            if partial_len % QUAD_LEN == 0 {
                assert_eq!(partial_len, nums_decoded);
            }
        }
    }
}

fn do_decode_cursor_sink_decode_in_chunks_emits_complete_quads_until_end<D: Decoder>()
where
    TupleSink: DecodeQuadSink<<D as Decoder>::DecodedQuad>,
{
    let mut nums: Vec<u32> = Vec::new();
    let mut encoded = Vec::new();
    let mut expected = Vec::new();

    for len in QUAD_LEN..100 {
        nums.clear();
        encoded.clear();

        prepare_offset_nums(len, 1000, &mut nums, &mut encoded);

        for chunk_len in QUAD_LEN..len {
            let complete_quad_len = chunk_len - (chunk_len % QUAD_LEN);
            let mut cursor = DecodeCursor::new(&encoded, len);
            let mut total_nums_decoded = 0;

            while cursor.has_more() {
                expected.clear();

                let remaining_to_decode = len - total_nums_decoded;
                let expected_decode_len = if remaining_to_decode <= chunk_len {
                    remaining_to_decode
                } else {
                    complete_quad_len
                };

                for num in 0..expected_decode_len {
                    expected.push((
                        num as usize,
                        total_nums_decoded as u32 + num as u32 + 1000,
                    ));
                }

                let mut sink = TupleSink::new();
                let nums_decoded = cursor.decode_sink::<D, _>(&mut sink, chunk_len);

                assert_eq!(
                    expected_decode_len,
                    nums_decoded,
                    "len {} chunk len {}",
                    len,
                    chunk_len
                );
                assert_eq!(expected, sink.tuples);

                total_nums_decoded += nums_decoded;
            }

            assert_eq!(
                len,
                total_nums_decoded,
                "len {} chunk len {}",
                len,
                chunk_len
            );
        }
    }
}

fn do_decode_cursor_sink_decode_in_chunks_smaller_than_first_quad_decodes_0_nums<D: Decoder>()
where
    TupleSink: DecodeQuadSink<<D as Decoder>::DecodedQuad>,
{
    let mut nums: Vec<u32> = Vec::new();
    let mut encoded = Vec::new();

    let len = 17;
    prepare_offset_nums(len, 1000, &mut nums, &mut encoded);

    for decode_len in 0..QUAD_LEN {
        let mut cursor = DecodeCursor::new(&encoded, len);
        let mut sink = TupleSink::new();
        let nums_decoded = cursor.decode_sink::<D, _>(&mut sink, decode_len);

        assert_eq!(0, nums_decoded);
        let expected = Vec::<(usize, u32)>::new();
        assert_eq!(expected, sink.tuples);
    }
}

fn do_decode_cursor_sink_decode_final_chunk_partially_includes_leftovers_decodes_only_complete_quads<
    D: Decoder,
>()
where
    TupleSink: DecodeQuadSink<<D as Decoder>::DecodedQuad>,
{
    let mut nums: Vec<u32> = Vec::new();
    let mut encoded = Vec::new();

    // enough that SIMD decoders can kick in (SSE3 needs at least 16 for instance)
    prepare_offset_nums(43, 1000, &mut nums, &mut encoded);

    for decode_len in 40..43 {
        let mut cursor = DecodeCursor::new(&encoded, 43);
        let mut sink = TupleSink::new();
        let nums_decoded = cursor.decode_sink::<D, _>(&mut sink, decode_len);

        assert_eq!(40, nums_decoded);
        let expected_suffix = vec![(35, 1035), (36, 1036), (37, 1037), (38, 1038), (39, 1039)];
        assert_eq!(&expected_suffix[..], &sink.tuples[35..]);
    }

    // but asking for all 11 gets the last partial quad
    let mut cursor = DecodeCursor::new(&encoded, 43);
    let mut sink = TupleSink::new();
    let nums_decoded = cursor.decode_sink::<D, _>(&mut sink, 43);

    assert_eq!(43, nums_decoded);
    let expected = vec![
        (35, 1035),
        (36, 1036),
        (37, 1037),
        (38, 1038),
        (39, 1039),
        (40, 1040),
        (41, 1041),
        (42, 1042),
    ];
    assert_eq!(expected, &sink.tuples[35..]);
}

fn do_decode_cursor_sink_decode_after_finishing_input_decodes_0_numbers<D: Decoder>()
where
    TupleSink: DecodeQuadSink<<D as Decoder>::DecodedQuad>,
{
    let mut nums: Vec<u32> = Vec::new();
    let mut encoded = Vec::new();

    for len in 0..100 {
        nums.clear();
        encoded.clear();

        prepare_offset_nums(100, 1000, &mut nums, &mut encoded);

        // read the whole thing

        let mut cursor = DecodeCursor::new(&encoded, len);
        {
            let mut sink = TupleSink::new();
            let nums_decoded = cursor.decode_sink::<D, _>(&mut sink, len);

            assert_eq!(len, nums_decoded);
            assert_eq!(len, sink.tuples.len());
        }

        // try keep decoding
        {
            let mut sink = TupleSink::new();
            let nums_decoded = cursor.decode_sink::<D, _>(&mut sink, 1000);

            assert_eq!(0, nums_decoded);
            assert_eq!(0, sink.tuples.len());
        }
    }
}

/// Prepare some input
fn prepare_offset_nums(
    count: usize,
    offset: u32,
    nums: &mut Vec<u32>,
    encoded: &mut Vec<u8>,
) -> usize {
    for num in 0..count {
        nums.push(num as u32 + offset);
    }

    encoded.resize(count * 5, 0);
    let encoded_len = encode::<Scalar>(&nums, encoded);
    encoded.truncate(encoded_len);

    encoded_len
}

struct TupleSink {
    tuples: Vec<(usize, u32)>,
}

impl TupleSink {
    fn new() -> TupleSink {
        TupleSink { tuples: Vec::new() }
    }
}

impl DecodeQuadSink<()> for TupleSink {
    fn on_quad(&mut self, _: (), _: usize) {
        unimplemented!()
    }
}

#[cfg(feature = "x86_ssse3")]
impl DecodeQuadSink<stdsimd::simd::u8x16> for TupleSink {
    fn on_quad(&mut self, quad: stdsimd::simd::u8x16, nums_decoded: usize) {
        let u32s = stdsimd::simd::u32x4::from(quad);
        self.tuples.push((nums_decoded, u32s.extract(0)));
        self.tuples.push((nums_decoded + 1, u32s.extract(1)));
        self.tuples.push((nums_decoded + 2, u32s.extract(2)));
        self.tuples.push((nums_decoded + 3, u32s.extract(3)));
    }
}

impl DecodeSingleSink for TupleSink {
    fn on_number(&mut self, num: u32, nums_decoded: usize) {
        self.tuples.push((nums_decoded, num))
    }
}
