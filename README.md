[![](https://img.shields.io/crates/v/stream-vbyte.svg)](https://crates.io/crates/stream-vbyte) [![](https://docs.rs/stream-vbyte/badge.svg)](https://docs.rs/stream-vbyte/)


A port of Stream VByte to Rust.

Stream VByte is a variable-length unsigned int encoding designed to make SIMD processing more efficient.

See https://lemire.me/blog/2017/09/27/stream-vbyte-breaking-new-speed-records-for-integer-compression/ and https://arxiv.org/pdf/1709.08990.pdf for details on the format. The reference C implementation is https://github.com/lemire/streamvbyte.

# Using SIMD

There is one SIMD-accelerated `Decoder` implementation: `x86::Ssse3`, available when you enable the `x86_ssse3` feature for this crate. [SSSE3](https://en.wikipedia.org/wiki/SSSE3) has been around since Core-era Intel CPUs, so any modern `x86_64` system should have it. Unless you're writing a service for specific hardware that you know has the feature, you may need to do some runtime detection and decide at runtime whether or not to use the SSSE3 decoder. Something like [raw-cpuid](https://crates.io/crates/raw-cpuid) will probably be useful for that.

Currently, SIMD support relies on nightly-only rust features. You'll also need to add some compiler flags, namely:

```
RUSTFLAGS='-C target-feature=+ssse3'
```

# Maintainers

To generate the lookup tables:

```
cargo run --example generate_decode_table > tmp/tables.rs && mv tmp/tables.rs src/tables.rs
```

To run the tests:

```
RUSTFLAGS='-C target-feature=+ssse3' rustup run nightly cargo test --features x86_ssse3
```

To run the benchmarks:

```
RUSTFLAGS='-C target-feature=+ssse3' rustup run nightly cargo bench --features x86_ssse3
```
