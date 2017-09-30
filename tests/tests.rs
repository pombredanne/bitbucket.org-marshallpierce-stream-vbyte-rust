extern crate stream_vbyte;
extern crate rand;

use std::fs::File;
use std::io::{Read, Write};

use self::rand::Rng;
use self::rand::distributions::{IndependentSample, Range};

use stream_vbyte::*;

#[test]
fn random_roundtrip() {
    let mut nums: Vec<u32> = Vec::new();
    let mut encoded = Vec::new();
    let mut decoded = Vec::new();
    let mut rng = rand::weak_rng();
    for _ in 0..10_000 {
        nums.clear();
        encoded.clear();
        decoded.clear();

        let count = rng.gen_range(0, 1000);

        for i in RandomVarintEncodedLengthIter::new(rand::weak_rng()).take(count) {
            nums.push(i);
        }

        encoded.resize(count * 5, 0);

        let bytes_written = encode::<Scalar>(&nums, &mut encoded);

        decoded.resize(count, 0);

        assert_eq!(bytes_written, decode::<Scalar>(&encoded[0..bytes_written], count, &mut decoded));

        assert_eq!(nums, decoded);
    }
}

#[test]
fn all_zeros() {
    let mut nums: Vec<u32> = Vec::new();
    let mut encoded = Vec::new();
    let mut decoded = Vec::new();

    for count in 0..1000 {
        nums.clear();

        // 1 byte for each number, + 1 control byte for every 4 zeroes
        let encoded_len = count + (count + 3) / 4;

        encoded.resize(encoded_len, 0xFF);
        decoded.resize(count, 0xFF);

        println!("count {}", count);

        for _ in 0..count {
            nums.push(0);
        }

        assert_eq!(encoded_len, encode::<Scalar>(&nums, &mut encoded));
        for (i, &b) in encoded[0..encoded_len].iter().enumerate() {
            assert_eq!(0, b, "index {}", i);
        }

        assert_eq!(encoded_len, decode::<Scalar>(&encoded[0..encoded_len], count, &mut decoded));

        assert_eq!(nums, decoded);
    }
}

#[test]
fn compare_reference_impl() {
    let ref_nums: Vec<u32> = (0..5000).map(|x| x * 100).collect();
    let mut ref_data = Vec::new();
    File::open("tests/data/data.bin").unwrap().read_to_end(&mut ref_data).unwrap();
    let ref_data = ref_data;

    let mut rust_encoded_data = Vec::new();
    rust_encoded_data.resize(ref_nums.len() * 5, 0);
    let bytes_written = encode::<Scalar>(&ref_nums, &mut rust_encoded_data);
    rust_encoded_data.truncate(bytes_written);
    File::create("tests/data/rust-data.bin").unwrap().write_all(&rust_encoded_data).unwrap();

    assert_eq!(ref_data.len(), bytes_written);
    assert_eq!(ref_data, rust_encoded_data);
}

// Evenly distributed random numbers end up biased heavily towards longer encoded byte lengths:
// there are a lot more large numbers than there are small (duh), but for exercising serialization
// code paths, we'd like many at all byte lengths. This is also arguably more representative of
// real data. This should emit values whose varint lengths are uniformly distributed across the
// whole length range.
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
