[![crates.io](https://img.shields.io/crates/v/lz11?logo=rust&style=flat-square)](https://crates.io/crates/lz11) \
[![docs.rs](https://img.shields.io/badge/docs-lz11-1370D3?style=flat-square&logo=rust)](https://docs.rs/lz11/latest/lz11/) \
![Crates.io License](https://img.shields.io/crates/l/lz11?style=flat-square) \
![CI Status](https://github.com/mulbruk/lz11/actions/workflows/ci.yml/badge.svg)

## What

A library and command-line tool for working with Nintendo's LZ10 and LZ11 compression formats.

## Why

Existing Rust libraries for working with LZ10/LZ11 were slow, and produced compressed output larger than the input file.

## How to use

Decompress a file

```bash
lz11 decompress 00000001.app WLME.dol
```

Compress a file

```bash
lz11 compress WLME.dol 00000001.app
```

Compress a file using LZ10

```bash
lz11 compress --format lz10 WLME.dol 00000001.app
```

Compress a file using a specific compression level

```bash
lz11 compress -o9 WLME.dol 00000001.app
```

## Installing

### Direct Download

Download the latest binary for your system (Windows/Mac/Linux) from the [releases page](https://github.com/mulbruk/lz11/releases)

### Via Cargo

```bash
cargo install lz11
```

### From Source

Ensure that you have the [Rust toolchain](https://rustup.rs/) installed on your computer, and then run:

```bash
git clone https://github.com/mulbruk/lz11.git
cd lz11
cargo build --release --features="cli" --bin lz11
```

Copy the `lz11` executable from `target/release/` to your location of choice.

## License

`lz11` is made available under the terms of either the MIT License or the Apache License 2.0, at your option.

See the LICENSE-APACHE and LICENSE-MIT files for license details.
