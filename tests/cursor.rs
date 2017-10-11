extern crate stream_vbyte;
extern crate rand;

use std::cmp;

use self::rand::Rng;

use stream_vbyte::*;

#[path = "../src/random_varint.rs"]
mod random_varint;

use random_varint::*;

const MIN_DECODE_BUFFER_LEN: usize = 4;

#[test]
fn decode_cursor_random_decode_len_scalar() {
    do_decode_cursor_random_decode_len::<Scalar>();
}

#[cfg(feature = "x86_ssse3")]
#[test]
fn decode_cursor_random_decode_len_ssse3() {
    do_decode_cursor_random_decode_len::<x86::Ssse3>();
}

#[test]
fn decode_cursor_every_decode_len_scalar() {
    do_decode_cursor_every_decode_len::<Scalar>()
}

#[cfg(feature = "x86_ssse3")]
#[test]
fn decode_cursor_every_decode_len_ssse3() {
    do_decode_cursor_every_decode_len::<x86::Ssse3>()
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

fn do_decode_cursor_every_decode_len<D: Decoder>() {
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

                if cursor.has_more() {
                    // if we're in the middle somewhere, we shouldn't fall short by any more than 3
                    // (partial quad size)
                    assert!(decode_len - nums_decoded <= 3, "{} - {} = {}",
                            decode_len, nums_decoded, decode_len - nums_decoded);


                }
                // the chunk is correct
                assert_eq!(&nums[decoded_accum.len()..(decoded_accum.len() + nums_decoded)],
                           &decoded[0..nums_decoded]);
                // beyond the chunk wasn't overwritten
                for (i, &n) in decoded[nums_decoded..(count + extra_slots)].iter().enumerate() {
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

fn do_decode_cursor_random_decode_len<D: Decoder>() {
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

fn do_decode_cursor_skip_every_allowable_len_from_start<D: Decoder>() {
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
        for skip_len in (0..(count / 4 + 1)).map(|i| i * 4) {
            let mut cursor = DecodeCursor::new(&encoded[0..encoded_len], count);

            cursor.skip(skip_len);

            let garbage = rng.gen();
            decoded.clear();
            decoded.resize(count + extra_slots, garbage);
            let nums_decoded = cursor.decode::<D>(&mut decoded);

            assert!(!cursor.has_more());
            // decoded all the remaining numbers
            assert_eq!(count - skip_len, nums_decoded);
            // the chunk is correct
            assert_eq!(&nums[skip_len..], &decoded[0..nums_decoded]);
            // beyond the chunk wasn't overwritten
            for (i, &n) in decoded[nums_decoded..(count + extra_slots)].iter().enumerate() {
                assert_eq!(garbage as u32, n, "index {}", i);
            }

            assert_eq!(encoded_len, cursor.input_consumed());
        }
    }
}

fn do_decode_cursor_skip_every_allowable_len_between_decodes<D: Decoder>() {
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

        'dec1: for initial_decode_len in 4..(count + 1) {
            // skips must be divisible by 4
            'skip: for skip_len in (0..count / 4).map(|i| i * 4) {
                'dec2: for final_decode_len in 4..count {
                    decoded.clear();

                    let garbage = rng.gen();
                    let extra_slots = 100;
                    decoded.resize(count + extra_slots, garbage);

                    // 1: decode a bit
                    let mut cursor = DecodeCursor::new(&encoded[0..encoded_len], count);
                    let initial_decoded_nums = cursor.decode::<D>(&mut decoded[0..initial_decode_len]);
                    assert_eq!(&nums[0..initial_decoded_nums], &decoded[0..initial_decoded_nums]);
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

                    if final_decode_len > nums_after_skip {
                        // final_decode_len has gotten too big; go to next enclosing loop
                        break 'dec2;
                    }

                    // 3: decode some more
                    let garbage = rng.gen();
                    decoded.clear();
                    decoded.resize(count + extra_slots, garbage);
                    let final_decoded_nums = cursor.decode::<D>(&mut decoded[0..final_decode_len]);

                    // the chunk is correct
                    assert_eq!(&nums[(initial_decoded_nums + skip_len)..(initial_decoded_nums + skip_len + final_decoded_nums)],
                               &decoded[0..final_decoded_nums]);
                    // beyond the chunk wasn't overwritten
                    for (i, &n) in decoded[final_decoded_nums..(count + extra_slots)].iter().enumerate() {
                        assert_eq!(garbage as u32, n, "index {}", i);
                    }
                }
            }
        }
    }
}
