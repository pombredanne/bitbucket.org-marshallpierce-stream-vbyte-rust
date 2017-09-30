[![](https://img.shields.io/crates/v/stream-vbyte.svg)](https://crates.io/crates/stream-vbyte) [![](https://docs.rs/stream-vbyte/badge.svg)](https://docs.rs/stream-vbyte/)


A port of Stream VByte to Rust.

Stream VByte is a variable-length unsigned int encoding designed to make SIMD processing more efficient.

See https://lemire.me/blog/2017/09/27/stream-vbyte-breaking-new-speed-records-for-integer-compression/ and https://arxiv.org/pdf/1709.08990.pdf for details on the format. The reference C implementation is https://github.com/lemire/streamvbyte.

To generate the lookup tables:

```
cargo run --example generate_decode_table > tmp/tables.rs && mv tmp/tables.rs src/tables.rs
```
