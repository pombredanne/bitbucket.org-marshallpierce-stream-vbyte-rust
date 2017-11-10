extern crate rand;

use self::rand::Rng;

#[path = "random_varint.rs"]
pub mod random_varint;

use self::random_varint::*;

use ::*;
use cumulative_encoded_len;
use decode::decode_num_scalar;
use encode::encode_num_scalar;

#[test]
fn encode_decode_roundtrip_random() {
    let mut buf = [0; 4];
    for num in RandomVarintEncodedLengthIter::new(rand::weak_rng()).take(100_000) {
        let len = encode_num_scalar(num, &mut buf);
        let decoded = decode_num_scalar(len, &buf);

        assert_eq!(num, decoded);
    }
}

#[test]
fn encoded_shape_len_0() {
    let shape = encoded_shape(0);
    let expected = EncodedShape {
        control_bytes_len: 0,
        complete_control_bytes_len: 0,
        leftover_numbers: 0,
    };

    assert_eq!(expected, shape);
}

#[test]
fn encoded_shape_len_1() {
    let shape = encoded_shape(1);
    let expected = EncodedShape {
        control_bytes_len: 1,
        complete_control_bytes_len: 0,
        leftover_numbers: 1,
    };

    assert_eq!(expected, shape);
}

#[test]
fn encoded_shape_len_3() {
    let shape = encoded_shape(3);
    let expected = EncodedShape {
        control_bytes_len: 1,
        complete_control_bytes_len: 0,
        leftover_numbers: 3,
    };

    assert_eq!(expected, shape);
}

#[test]
fn encoded_shape_len_4() {
    let shape = encoded_shape(4);
    let expected = EncodedShape {
        control_bytes_len: 1,
        complete_control_bytes_len: 1,
        leftover_numbers: 0,
    };

    assert_eq!(expected, shape);
}

#[test]
fn encoded_shape_len_5() {
    let shape = encoded_shape(5);
    let expected = EncodedShape {
        control_bytes_len: 2,
        complete_control_bytes_len: 1,
        leftover_numbers: 1,
    };

    assert_eq!(expected, shape);
}

#[test]
fn cumulative_encoded_len_accurate_complete_quad() {
    let mut nums: Vec<u32> = Vec::new();
    let mut encoded = Vec::new();
    let mut rng = rand::weak_rng();

    for _ in 0..1_000 {
        nums.clear();
        encoded.clear();

        // must use complete quads since calculating encoded length is only valid in that case
        let count = rng.gen_range(0, 250) * 4;

        for i in RandomVarintEncodedLengthIter::new(rand::weak_rng()).take(count) {
            nums.push(i);
        }

        encoded.resize(count * 5, 0xFF);

        let encoded_len = encode::<Scalar>(&nums, &mut encoded);

        let shape = encoded_shape(count);

        assert_eq!(
            encoded_len - shape.control_bytes_len,
            cumulative_encoded_len(&encoded[0..shape.control_bytes_len])
        );
    }
}
