extern crate stream_vbyte;
extern crate rand;

use self::rand::Rng;

use stream_vbyte::*;

#[path="../src/random_varint.rs"]
mod random_varint;
use random_varint::*;

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

        assert_eq!(encoded_len, decode::<D>(&encoded[0..encoded_len], count, &mut decoded[0..count]));
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
            assert_ne!(garbage, num);

            // 1 byte for each number, + 1 control byte for every 4 zeroes
            let control_byte_len = (count + 3) / 4;
            let encoded_len = count + control_byte_len;

            // make the vecs a little oversized so we can tell if something clobbers the bytes
            // following what should be written to
            let extra_slots = 1000;
            encoded.resize(encoded_len + extra_slots, garbage);
            decoded.resize(count + extra_slots, garbage as u32);

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

            assert_eq!(encoded_len, decode::<D>(&encoded[0..encoded_len], count, &mut decoded[0..count]));
            // extra u32s in decoded were not touched
            for (i, &n) in decoded[count..(count + extra_slots)].iter().enumerate() {
                assert_eq!(garbage as u32, n, "index {}", i);
            }

            assert_eq!(&nums[..], &decoded[0..count]);
        }
    }
}

