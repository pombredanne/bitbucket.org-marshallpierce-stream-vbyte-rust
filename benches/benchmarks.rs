#![feature(test)]

extern crate rand;
extern crate stream_vbyte;
extern crate test;

#[cfg(feature = "x86_ssse3")]
extern crate x86intrin;

use self::test::Bencher;

use self::rand::Rng;
use self::rand::distributions::{IndependentSample, Range};

use std::iter;

use stream_vbyte::*;

#[bench]
fn encode_scalar_rand_1k(b: &mut Bencher) {
    do_encode_bench(
        b,
        RandomVarintEncodedLengthIter::new(rand::weak_rng()).take(1000),
        Scalar,
    );
}

#[bench]
fn encode_scalar_rand_1m(b: &mut Bencher) {
    do_encode_bench(
        b,
        RandomVarintEncodedLengthIter::new(rand::weak_rng()).take(1000 * 1000),
        Scalar,
    );
}

#[cfg(feature = "x86_sse41")]
#[bench]
fn encode_sse41_rand_1k(b: &mut Bencher) {
    do_encode_bench(
        b,
        RandomVarintEncodedLengthIter::new(rand::weak_rng()).take(1000),
        x86::Sse41,
    );
}

#[cfg(feature = "x86_sse41")]
#[bench]
fn encode_sse41_rand_1m(b: &mut Bencher) {
    do_encode_bench(
        b,
        RandomVarintEncodedLengthIter::new(rand::weak_rng()).take(1000 * 1000),
        x86::Sse41,
    );
}

#[bench]
fn encode_scalar_zeros_1k(b: &mut Bencher) {
    do_encode_bench(b, iter::repeat(0).take(1000), Scalar);
}

#[bench]
fn encode_scalar_zeros_1m(b: &mut Bencher) {
    do_encode_bench(b, iter::repeat(0).take(1_000_000), Scalar);
}

#[cfg(feature = "x86_sse41")]
#[bench]
fn encode_sse41_zeros_1k(b: &mut Bencher) {
    do_encode_bench(b, iter::repeat(0).take(1000), x86::Sse41);
}

#[cfg(feature = "x86_sse41")]
#[bench]
fn encode_sse41_zeros_1m(b: &mut Bencher) {
    do_encode_bench(b, iter::repeat(0).take(1_000_000), x86::Sse41);
}

#[bench]
fn decode_scalar_rand_1k(b: &mut Bencher) {
    do_decode_bench(
        b,
        RandomVarintEncodedLengthIter::new(rand::weak_rng()).take(1000),
        Scalar,
    );
}

#[cfg(feature = "x86_ssse3")]
#[bench]
fn decode_ssse3_rand_1k(b: &mut Bencher) {
    do_decode_bench(
        b,
        RandomVarintEncodedLengthIter::new(rand::weak_rng()).take(1000),
        x86::Ssse3,
    );
}

#[bench]
fn decode_scalar_rand_1m(b: &mut Bencher) {
    do_decode_bench(
        b,
        RandomVarintEncodedLengthIter::new(rand::weak_rng()).take(1_000_000),
        Scalar,
    );
}

#[cfg(feature = "x86_ssse3")]
#[bench]
fn decode_ssse3_rand_1m(b: &mut Bencher) {
    do_decode_bench(
        b,
        RandomVarintEncodedLengthIter::new(rand::weak_rng()).take(1_000_000),
        x86::Ssse3,
    );
}

#[bench]
fn decode_cursor_slice_scalar_rand_1k(b: &mut Bencher) {
    do_decode_cursor_slice_bench(
        b,
        RandomVarintEncodedLengthIter::new(rand::weak_rng()).take(1000),
        Scalar,
    );
}

#[cfg(feature = "x86_ssse3")]
#[bench]
fn decode_cursor_slice_ssse3_rand_1k(b: &mut Bencher) {
    do_decode_cursor_slice_bench(
        b,
        RandomVarintEncodedLengthIter::new(rand::weak_rng()).take(1000),
        x86::Ssse3,
    );
}

#[bench]
fn decode_cursor_slice_scalar_rand_1m(b: &mut Bencher) {
    do_decode_cursor_slice_bench(
        b,
        RandomVarintEncodedLengthIter::new(rand::weak_rng()).take(1_000_000),
        Scalar,
    );
}

#[cfg(feature = "x86_ssse3")]
#[bench]
fn decode_cursor_slice_ssse3_rand_1m(b: &mut Bencher) {
    do_decode_cursor_slice_bench(
        b,
        RandomVarintEncodedLengthIter::new(rand::weak_rng()).take(1_000_000),
        x86::Ssse3,
    );
}

#[bench]
fn decode_cursor_sink_no_op_scalar_rand_1m(b: &mut Bencher) {
    do_decode_cursor_sink_no_op_bench(
        b,
        RandomVarintEncodedLengthIter::new(rand::weak_rng()).take(1_000_000),
        Scalar,
    );
}

#[cfg(feature = "x86_ssse3")]
#[bench]
fn decode_cursor_sink_no_op_ssse3_rand_1m(b: &mut Bencher) {
    do_decode_cursor_sink_no_op_bench(
        b,
        RandomVarintEncodedLengthIter::new(rand::weak_rng()).take(1_000_000),
        x86::Ssse3,
    );
}

#[bench]
fn decode_scalar_zeros_1k(b: &mut Bencher) {
    do_decode_bench(b, iter::repeat(0).take(1000), Scalar);
}

#[cfg(feature = "x86_ssse3")]
#[bench]
fn decode_ssse3_zeros_1k(b: &mut Bencher) {
    do_decode_bench(b, iter::repeat(0).take(1000), x86::Ssse3);
}

#[bench]
fn decode_scalar_zeros_1m(b: &mut Bencher) {
    do_decode_bench(b, iter::repeat(0).take(1_000_000), Scalar);
}

#[cfg(feature = "x86_ssse3")]
#[bench]
fn decode_ssse3_zeros_1m(b: &mut Bencher) {
    do_decode_bench(b, iter::repeat(0).take(1_000_000), x86::Ssse3);
}

#[bench]
fn skip_all_1m(b: &mut Bencher) {
    let mut nums: Vec<u32> = Vec::new();
    let mut encoded = Vec::new();
    let mut decoded = Vec::new();
    let count = 1_000_000;

    for i in RandomVarintEncodedLengthIter::new(rand::weak_rng()).take(count) {
        nums.push(i);
    }

    encoded.resize(nums.len() * 5, 0);
    let bytes_written = stream_vbyte::encode::<Scalar>(&nums, &mut encoded);

    decoded.resize(nums.len(), 0);
    b.iter(|| {
        DecodeCursor::new(&encoded[0..bytes_written], count).skip(count);
    });
}

fn do_encode_bench<I: Iterator<Item = u32>, E: Encoder>(b: &mut Bencher, iter: I, _encoder: E) {
    let mut nums: Vec<u32> = Vec::new();
    let mut encoded = Vec::new();

    for i in iter {
        nums.push(i);
    }

    encoded.resize(nums.len() * 5, 0);

    b.iter(|| {
        let _ = stream_vbyte::encode::<E>(&nums, &mut encoded);
    });
}

// take a decoder param to save us some typing -- type inference won't work if you only specify some
// of the generic types
fn do_decode_bench<I: Iterator<Item = u32>, D: Decoder>(b: &mut Bencher, iter: I, _decoder: D)
where
    for<'a> SliceDecodeSink<'a>: DecodeQuadSink<<D as Decoder>::DecodedQuad>,
{
    let mut nums: Vec<u32> = Vec::new();
    let mut encoded = Vec::new();
    let mut decoded = Vec::new();

    for i in iter {
        nums.push(i);
    }

    encoded.resize(nums.len() * 5, 0);
    let bytes_written = stream_vbyte::encode::<Scalar>(&nums, &mut encoded);

    decoded.resize(nums.len(), 0);
    b.iter(|| {
        stream_vbyte::decode::<D>(&encoded[0..bytes_written], nums.len(), &mut decoded);
    });
}

fn do_decode_cursor_slice_bench<I: Iterator<Item = u32>, D: Decoder>(
    b: &mut Bencher,
    iter: I,
    _decoder: D,
) where
    for<'a> SliceDecodeSink<'a>: DecodeQuadSink<<D as Decoder>::DecodedQuad>,
{
    let mut nums: Vec<u32> = Vec::new();
    let mut encoded = Vec::new();
    let mut decoded = Vec::new();

    for i in iter {
        nums.push(i);
    }

    encoded.resize(nums.len() * 5, 0);
    let _ = stream_vbyte::encode::<Scalar>(&nums, &mut encoded);

    decoded.resize(nums.len(), 0);
    b.iter(|| {
        let mut cursor = DecodeCursor::new(&encoded, nums.len());
        cursor.decode_slice::<D>(&mut decoded);
    })
}

fn do_decode_cursor_sink_no_op_bench<I: Iterator<Item = u32>, D: Decoder>(
    b: &mut Bencher,
    iter: I,
    _decoder: D,
) where
    NoOpSink: DecodeQuadSink<<D as Decoder>::DecodedQuad>,
{
    let mut nums: Vec<u32> = Vec::new();
    let mut encoded = Vec::new();

    for i in iter {
        nums.push(i);
    }

    encoded.resize(nums.len() * 5, 0);
    let _ = stream_vbyte::encode::<Scalar>(&nums, &mut encoded);

    b.iter(|| {
        let mut cursor = DecodeCursor::new(&encoded, nums.len());
        let mut sink = NoOpSink;
        cursor.decode_sink::<D, _>(&mut sink, nums.len());
    })
}

// copied from tests because it's handy here too
struct RandomVarintEncodedLengthIter<R: Rng> {
    ranges: [Range<u32>; 4],
    range_for_picking_range: Range<usize>,
    rng: R,
}

impl<R: Rng> RandomVarintEncodedLengthIter<R> {
    fn new(rng: R) -> RandomVarintEncodedLengthIter<R> {
        RandomVarintEncodedLengthIter {
            ranges: [
                Range::new(0, 1 << 8),
                Range::new(1 << 8, 1 << 16),
                Range::new(1 << 16, 1 << 24),
                Range::new(1 << 24, u32::max_value()), // this won't ever emit the max value, sadly
            ],
            range_for_picking_range: Range::new(0, 4),
            rng,
        }
    }
}

impl<R: Rng> Iterator for RandomVarintEncodedLengthIter<R> {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        // pick the range we'll use
        let value_range = self.ranges[self.range_for_picking_range.ind_sample(&mut self.rng)];

        Some(value_range.ind_sample(&mut self.rng))
    }
}

struct NoOpSink;

impl DecodeSingleSink for NoOpSink {
    fn on_number(&mut self, _num: u32, _nums_decoded: usize) {}
}

impl DecodeQuadSink<()> for NoOpSink {
    fn on_quad(&mut self, _quad: (), _nums_decoded: usize) {}
}

#[cfg(feature = "x86_ssse3")]
impl DecodeQuadSink<x86intrin::m128i> for NoOpSink {
    fn on_quad(&mut self, _quad: x86intrin::m128i, _nums_decoded: usize) {}
}
