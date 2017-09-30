#![feature(test)]

extern crate stream_vbyte;
extern crate rand;
extern crate test;

use self::test::Bencher;

use self::rand::Rng;
use self::rand::distributions::{IndependentSample, Range};

use std::iter;

use stream_vbyte::*;

#[bench]
fn encode_rand_1k(b: &mut Bencher) {
    do_encode_bench(b, RandomVarintEncodedLengthIter::new(rand::weak_rng()).take(1024));
}

#[bench]
fn encode_rand_1m(b: &mut Bencher) {
    do_encode_bench(b, RandomVarintEncodedLengthIter::new(rand::weak_rng()).take(1024 * 1024));
}

#[bench]
fn encode_zeros_1k(b: &mut Bencher) {
    do_encode_bench(b, iter::repeat(0).take(1024));
}

#[bench]
fn encode_zeros_1m(b: &mut Bencher) {
    do_encode_bench(b, iter::repeat(0).take(1024 * 1024));
}

#[bench]
fn decode_scalar_rand_1k(b: &mut Bencher) {
    do_decode_bench(b, RandomVarintEncodedLengthIter::new(rand::weak_rng()).take(1024), Scalar);
}

#[cfg(feature = "x86_ssse3")]
#[bench]
fn decode_ssse3_rand_1k(b: &mut Bencher) {
    do_decode_bench(b, RandomVarintEncodedLengthIter::new(rand::weak_rng()).take(1024), x86::Ssse3);
}

#[bench]
fn decode_scalar_rand_1m(b: &mut Bencher) {
    do_decode_bench(b, RandomVarintEncodedLengthIter::new(rand::weak_rng()).take(1024 * 1024), Scalar);
}

#[cfg(feature = "x86_ssse3")]
#[bench]
fn decode_ssse3_rand_1m(b: &mut Bencher) {
    do_decode_bench(b, RandomVarintEncodedLengthIter::new(rand::weak_rng()).take(1024 * 1024), x86::Ssse3);
}

#[bench]
fn decode_scalar_zeros_1k(b: &mut Bencher) {
    do_decode_bench(b, iter::repeat(0).take(1024), Scalar);
}

#[cfg(feature = "x86_ssse3")]
#[bench]
fn decode_ssse3_zeros_1k(b: &mut Bencher) {
    do_decode_bench(b, iter::repeat(0).take(1024), x86::Ssse3);
}

#[bench]
fn decode_scalar_zeros_1m(b: &mut Bencher) {
    do_decode_bench(b, iter::repeat(0).take(1024 * 1024), Scalar);
}

#[cfg(feature = "x86_ssse3")]
#[bench]
fn decode_ssse3_zeros_1m(b: &mut Bencher) {
    do_decode_bench(b, iter::repeat(0).take(1024 * 1024), x86::Ssse3);
}

fn do_encode_bench<I: Iterator<Item=u32>>(b: &mut Bencher, iter: I) {
    let mut nums: Vec<u32> = Vec::new();
    let mut encoded = Vec::new();

    for i in iter {
        nums.push(i);
    }

    encoded.resize(nums.len() * 5, 0);

    b.iter(|| {
        let _ = stream_vbyte::encode::<Scalar>(&nums, &mut encoded);
    });
}

// take a decoder param to save us some typing -- type inference won't work if you only specify some
// of the generic types
fn do_decode_bench<I: Iterator<Item=u32>, D: Decoder>(b: &mut Bencher, iter: I, _decoder: D) {
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

// copied from tests because it's handy here too
struct RandomVarintEncodedLengthIter<R: Rng> {
    ranges: [Range<u32>; 4],
    range_for_picking_range: Range<usize>,
    rng: R
}

impl<R: Rng> RandomVarintEncodedLengthIter<R> {
    fn new(rng: R) -> RandomVarintEncodedLengthIter<R> {
        RandomVarintEncodedLengthIter {
            ranges: [
                Range::new(0, 1 << 8),
                Range::new(1 << 8, 1 << 16),
                Range::new(1 << 16, 1 << 24),
                Range::new(1 << 24, u32::max_value()) // this won't ever emit the max value, sadly
            ],
            range_for_picking_range: Range::new(0, 4),
            rng
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
