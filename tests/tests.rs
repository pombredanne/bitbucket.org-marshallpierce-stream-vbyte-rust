extern crate stream_vbyte;
extern crate rand;

use std::fs::File;
use std::io::Read;
use std::cmp;

use self::rand::Rng;

use stream_vbyte::*;

#[path="../src/random_varint.rs"]
mod random_varint;
use random_varint::*;

const MIN_DECODE_BUFFER_LEN: usize = 4;

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

#[test]
fn partial_final_quad_roundtrip() {
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
fn encode_compare_reference_impl() {
    let ref_nums: Vec<u32> = (0..5000).map(|x| x * 100).collect();
    let mut ref_data = Vec::new();
    File::open("tests/data/data.bin").unwrap().read_to_end(&mut ref_data).unwrap();
    let ref_data = ref_data;

    let mut rust_encoded_data = Vec::new();
    rust_encoded_data.resize(ref_nums.len() * 5, 0);
    let bytes_written = encode::<Scalar>(&ref_nums, &mut rust_encoded_data);
    rust_encoded_data.truncate(bytes_written);

    assert_eq!(ref_data.len(), bytes_written);
    assert_eq!(ref_data, rust_encoded_data);
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
        let decoded_len = cmp::max(4, count);

        let garbage: u8 = rng.gen();
        encoded.resize(count * 5 + extra_slots, garbage);
        decoded.resize(decoded_len + extra_slots, garbage as u32);

        let encoded_len = encode::<E>(&nums, &mut encoded);
        // extra bytes in encoded were not touched
        for (i, &b) in encoded[encoded_len..(encoded_len + extra_slots)].iter().enumerate() {
            assert_eq!(garbage, b, "index {}", i);
        }

        assert_eq!(encoded_len, decode::<D>(&encoded[0..encoded_len], count, &mut decoded[0..decoded_len]));
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
            // create something distinct from `num` so we can tell if it gets overwritten
            let garbage = num.overflowing_add(1).0;
            assert_ne!(garbage, num);

            // 1 byte for each number, + 1 control byte for every 4 zeroes
            let control_byte_len = (count + 3) / 4;
            let encoded_len = count + control_byte_len;

            // make the vecs a little oversized so we can tell if something clobbers the bytes
            // following what should be written to
            let extra_slots = 1000;
            let decoded_len = cmp::max(4, count);

            encoded.resize(encoded_len + extra_slots, garbage);
            decoded.resize(decoded_len + extra_slots, garbage as u32);

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

            assert_eq!(encoded_len, decode::<D>(&encoded[0..encoded_len], count, &mut decoded[0..decoded_len]));
            // extra u32s in decoded were not touched
            for (i, &n) in decoded[count..(count + extra_slots)].iter().enumerate() {
                assert_eq!(garbage as u32, n, "index {}", i);
            }

            assert_eq!(&nums[..], &decoded[0..count]);
        }
    }
}

fn decode_in_chunks_every_decode_len<D: Decoder>() {
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

        // try every legal decode length
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

            assert_eq!(encoded_len, cursor.input_consumed());
            assert_eq!(count, decoded_accum.len());
            assert_eq!(&nums, &decoded_accum);
        }
    }
}

fn decode_in_chunks_random_decode_len<D: Decoder>() {
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
            let decode_len: usize = rng.gen_range(MIN_DECODE_BUFFER_LEN,
                                                  cmp::max(MIN_DECODE_BUFFER_LEN + 1, count + 1));
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

        assert_eq!(encoded_len, cursor.input_consumed());
        assert_eq!(count, decoded_accum.len());
        assert_eq!(&nums[..], &decoded_accum[0..count]);
    }
}
