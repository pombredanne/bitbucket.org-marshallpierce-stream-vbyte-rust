[![](https://img.shields.io/crates/v/stream-vbyte.svg)](https://crates.io/crates/stream-vbyte) [![](https://docs.rs/stream-vbyte/badge.svg)](https://docs.rs/stream-vbyte/)

A port of Stream VByte to Rust.

Stream VByte is a variable-length unsigned int encoding designed to make SIMD processing more efficient.

See https://lemire.me/blog/2017/09/27/stream-vbyte-breaking-new-speed-records-for-integer-compression/ and https://arxiv.org/pdf/1709.08990.pdf for details on the format. The reference C implementation is https://github.com/lemire/streamvbyte.

# Using it in your code

There are two traits, `Encoder` and `Decoder`, that allow you to choose what logic to use in the inner hot loops. If you know which hardware you'll be running on, or you add runtime detection of CPU features, you can choose to use one that takes advantage of your hardware. Something like [raw-cpuid](https://crates.io/crates/raw-cpuid) will probably be useful for runtime detection.

Encoders:
- For now, the only encoder is `Scalar`, which uses plain old scalar (i.e. non vectorized) code. It works on all platforms. On an E5-1650v3, it encodes about 140 million random `u32`s per second.

Decoders:
- `Scalar` is also a decoder, with about the same performance as it has for encodes.
- `x86::Ssse3` decodes about 2.7 billion random `u32`s per second.

# Using SIMD

- `x86::Ssse3`
    - Enable the `x86_ssse3` feature for this crate.
    - [SSSE3](https://en.wikipedia.org/wiki/SSSE3) has been around since Core-era Intel CPUs, so any modern `x86_64` system should have it.
    - Currently, SIMD support relies on nightly-only rust features. You'll also need to add some compiler flags, namely `RUSTFLAGS='-C target-feature=+ssse3'`

# Play with the CLI example

There's a `cli.rs` example provided that demonstrates encoding and decoding.

To encode some numbers, provide numbers (one per line) to stdin, and the encoded result will be written to stdout.

Example using `jot` to produce the numbes `1` to `100`: `jot 100 | cargo run --example cli -- enc | base64`

Output, with cargo build output removed (the "Encoded ..." is on stderr for human convenience):

```
Encoded 100 numbers
AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8g
ISIjJCUmJygpKissLS4vMDEyMzQ1Njc4OTo7PD0+P0BBQkNERUZHSElKS0xNTk9QUVJTVFVWV1hZ
WltcXV5fYGFiY2Q=
```

There's a corresponding decode mode that reads the encoded format on stdin and emits the contents, one number per line. Here, we encode some numbers then decode them again: `jot 10 | cargo run --example cli -- enc | cargo run --example cli -- dec -c 10`

```
Encoded 10 numbers
1
2
3
4
5
6
7
8
9
10
Decoded 10 numbers

```

# Maintainers

To generate the lookup tables:

```
cargo run --example generate_decode_table > tmp/tables.rs && mv tmp/tables.rs src/tables.rs
```

To run the tests:

```
RUSTFLAGS='-C target-feature=+ssse3' cargo +nightly test --features x86_ssse3
```

To run the benchmarks:

```
RUSTFLAGS='-C target-feature=+ssse3' cargo +nightly bench --features x86_ssse3
```
