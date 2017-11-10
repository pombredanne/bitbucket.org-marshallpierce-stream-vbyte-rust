extern crate rand;

use self::rand::Rng;

use {cumulative_encoded_len, encode, Scalar};

use super::*;

#[path = "../random_varint.rs"]
mod random_varint;
use self::random_varint::*;

#[test]
fn decode_num_zero() {
    assert_eq!(0, decode_num_scalar(1, &vec![0, 0, 0, 0]));
}

#[test]
fn decode_num_u32_max() {
    assert_eq!(
        u32::max_value(),
        decode_num_scalar(4, &vec![0xFF, 0xFF, 0xFF, 0xFF])
    );
}

#[test]
fn decode_num_4_byte() {
    // 0x04030201
    assert_eq!(
        (4 << 24) + (3 << 16) + (2 << 8) + 1,
        decode_num_scalar(4, &vec![1, 2, 3, 4])
    );
}

#[test]
fn decode_num_3_byte() {
    // 0x04030201
    assert_eq!(
        (3 << 16) + (2 << 8) + 1,
        decode_num_scalar(3, &vec![1, 2, 3])
    );
}

#[test]
fn decode_num_2_byte() {
    // 0x04030201
    assert_eq!((2 << 8) + 1, decode_num_scalar(2, &vec![1, 2]));
}

#[test]
fn decode_num_1_byte() {
    // 0x04030201
    assert_eq!(1, decode_num_scalar(1, &vec![1]));
}


#[test]
fn decoder_honors_nums_to_decode_scalar() {
    // scalar should be able to decode all control bytes regardless of remaining input
    decoder_honors_nums_to_decode::<Scalar>(0);
}

#[cfg(feature = "x86_ssse3")]
#[test]
fn decoder_honors_nums_to_decode_ssse3() {
    // Sse3 reads 16 bytes at a time, so it cannot handle the last 3 control bytes in case their
    // encoded nums are <16 bytes
    decoder_honors_nums_to_decode::<::x86::Ssse3>(3);
}

fn decoder_honors_nums_to_decode<D: Decoder>(control_byte_limit_fudge_factor: usize)
where
    for<'a> SliceDecodeSink<'a>: DecodeQuadSink<<D as Decoder>::DecodedQuad>,
{
    let mut nums: Vec<u32> = Vec::new();
    let mut encoded = Vec::new();
    let mut decoded = Vec::new();
    let mut rng = rand::weak_rng();

    let count = 1000;

    for control_bytes_to_decode in 0..(count / 4 - control_byte_limit_fudge_factor) {
        nums.clear();
        encoded.clear();
        decoded.clear();

        for i in RandomVarintEncodedLengthIter::new(rand::weak_rng()).take(count) {
            nums.push(i);
        }

        // make the vecs a little oversized so we can tell if something clobbers them
        let extra_slots = 1000;
        let garbage: u8 = rng.gen();
        encoded.resize(count * 5 + extra_slots, garbage);
        decoded.resize(count + extra_slots, garbage as u32);

        let encoded_len = encode::<Scalar>(&nums, &mut encoded);

        // count is a multiple of 4, so no partial quad
        let control_bytes = &encoded[0..count / 4];
        let encoded_nums = &encoded[count / 4..encoded_len];
        let (nums_decoded, bytes_read) = D::decode_quads(
            &control_bytes,
            &encoded_nums,
            control_bytes_to_decode,
            0,
            &mut SliceDecodeSink::new(&mut decoded),
        );

        let nums_to_decode = control_bytes_to_decode * 4;
        assert_eq!(nums_to_decode, nums_decoded);
        assert_eq!(
            bytes_read,
            cumulative_encoded_len(&control_bytes[0..control_bytes_to_decode])
        );

        // extra u32s in decoded were not touched
        for (i, &n) in decoded[nums_to_decode..].iter().enumerate() {
            assert_eq!(garbage as u32, n, "index {}", i);
        }

        assert_eq!(&nums[0..nums_to_decode], &decoded[0..nums_to_decode]);
    }
}
