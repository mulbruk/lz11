## What

A library and command-line tool for working with Nintendo's LZ10 and LZ11 compression formats.

## Why

I was using the [nintendo-lz](https://crates.io/crates/nintendo-lz) crate for a project and noticed that:
1. It was very slow
2. It produced compressed outputs that were larger than the input files

So I ended up having to write my own tool to replace it.

`lz11`:
1. Is fast, unless you run it with `-o9`
2. Achieves good compression ratios

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

## License

`lz11` is made available under the terms of either the MIT License or the Apache License 2.0, at your option.

See the LICENSE-APACHE and LICENSE-MIT files for license details.
