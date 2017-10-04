extern crate stream_vbyte;
extern crate rand;

use std::fs::File;
use std::io::{Read, Write};

use self::rand::Rng;
use self::rand::distributions::{IndependentSample, Range};

use stream_vbyte::*;

#[test]
fn random_roundtrip_scalar_scalar() {
    do_random_roundtrip::<Scalar, Scalar>();
}

#[cfg(feature = "x86_ssse3")]
#[test]
fn random_roundtrip_scalar_ssse3() {
    do_random_roundtrip::<Scalar, x86::Ssse3>();
}

#[test]
fn all_same_single_byte_scalar_scalar() {
    do_all_same_single_byte::<Scalar, Scalar>();
}

#[cfg(feature = "x86_ssse3")]
#[test]
fn all_same_single_byte_scalar_ssse3() {
    do_all_same_single_byte::<Scalar, x86::Ssse3>();
}

fn do_random_roundtrip<E: Encoder, D: Decoder>() {
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

        // make the vecs a little oversized so we can tell if something clobbers them
        let extra_slots = 1000;
        let garbage: u8 = rng.gen();
        encoded.resize(count * 5 + extra_slots, garbage);
        decoded.resize(count + extra_slots, garbage as u32);

        let encoded_len = encode::<E>(&nums, &mut encoded);
        // extra bytes in encoded were not touched
        for (i, &b) in encoded[encoded_len..(encoded_len + extra_slots)].iter().enumerate() {
            assert_eq!(garbage, b, "index {}", i);
        }

        assert_eq!(encoded_len, decode::<D>(&encoded[0..encoded_len], count, &mut decoded));
        // extra u32s in decoded were not touched
        for (i, &n) in decoded[count..(count + extra_slots)].iter().enumerate() {
            assert_eq!(garbage as u32, n, "index {}", i);
        }

        assert_eq!(&nums[..], &decoded[0..count]);
    }
}

fn do_all_same_single_byte<E: Encoder, D: Decoder>() {
    let mut nums: Vec<u32> = Vec::new();
    let mut encoded: Vec<u8> = Vec::new();
    let mut decoded: Vec<u32> = Vec::new();

    // for a bunch of lengths
    for count in (0..100).map(|l| l * 7) {
        // for every possible single byte
        for num in (0..256).map(|b| b as u8) {
            nums.clear();
            encoded.clear();
            decoded.clear();
            // create something distinct from `num` so we can tell when it gets overwritten
            let garbage = num.overflowing_add(1).0;
            assert!(garbage != num);

            // 1 byte for each number, + 1 control byte for every 4 zeroes
            let control_byte_len = (count + 3) / 4;
            let encoded_len = count + control_byte_len;

            // make the vecs a little oversized so we can tell if something clobbers the bytes
            // following what should be written to
            let extra_slots = 1000;
            encoded.resize(encoded_len + extra_slots, garbage);
            decoded.resize(count + extra_slots, garbage as u32);

            println!("count {}", count);

            for _ in 0..count {
                nums.push(num as u32);
            }

            assert_eq!(encoded_len, encode::<E>(&nums, &mut encoded));
            for (i, &b) in encoded[0..control_byte_len].iter().enumerate() {
                assert_eq!(0, b, "index {}", i);
            }
            for (i, &b) in encoded[control_byte_len..encoded_len].iter().enumerate() {
                assert_eq!(num, b, "index {}", i);
            }
            // extra bytes in encoded were not touched
            for (i, &b) in encoded[encoded_len..(encoded_len + extra_slots)].iter().enumerate() {
                assert_eq!(garbage, b, "index {}", i);
            }

            assert_eq!(encoded_len, decode::<D>(&encoded[0..encoded_len], count, &mut decoded));
            // extra u32s in decoded were not touched
            for (i, &n) in decoded[count..(count + extra_slots)].iter().enumerate() {
                assert_eq!(garbage as u32, n, "index {}", i);
            }

            assert_eq!(&nums[..], &decoded[0..count]);
        }
    }
}

#[test]
fn partial_final_quad() {
    // easily recognizable bit patterns
    let nums = vec![0, 1 << 8, 3 << 16, 7 << 24, 2 << 8, 4 << 16];
    let mut encoded = Vec::new();
    encoded.resize(nums.len() * 5, 0xFF);

    // 2 control bytes, 10 for first quad, 4 for second
    let encoded_len = 2 + 10 + 5;
    assert_eq!(encoded_len, encode::<Scalar>(&nums, &mut encoded[..]));
    for (i, &b) in encoded[encoded_len..].iter().enumerate() {
        assert_eq!(0xFF, b, "index {}", i);
    }

    let expected = vec![0xE4, 0x09,
                        0x00,
                        0x00, 0x01,
                        0x00, 0x00, 0x03,
                        0x00, 0x00, 0x00, 0x07,
                        0x00, 0x02,
                        0x00, 0x00, 0x04];
    assert_eq!(&expected[..], &encoded[0..encoded_len]);

    let mut decoded = Vec::new();
    decoded.resize(nums.len(), 0);
    decode::<Scalar>(&encoded[..], nums.len(), &mut decoded);
    assert_eq!(nums, decoded);
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
