<p align="center">
  <img src=".github/assets/deku-mascot.png" alt="Deku mascot transforming a bit stream into structured data" width="320">
</p>

<p align="center">
  <a href="https://crates.io/crates/deku"><img src="https://img.shields.io/crates/v/deku.svg" alt="Latest Version"></a>
  <a href="https://docs.rs/deku"><img src="https://docs.rs/deku/badge.svg" alt="Rust Documentation"></a>
  <a href="https://github.com/sharksforarms/deku/actions/workflows/main.yml"><img src="https://github.com/sharksforarms/deku/actions/workflows/main.yml/badge.svg" alt="CI Status"></a>
  <a href="https://codecov.io/gh/sharksforarms/deku"><img src="https://codecov.io/gh/sharksforarms/deku/branch/master/graph/badge.svg" alt="codecov"></a>
</p>


Deku provides declarative binary reading and writing for Rust


Describe a binary layout with annotated structs and enums, then derive bit-level reading and
writing implementations from the same definition. It is designed for network
protocols, file formats, and embedded data.

- Symmetric reading and writing from a single type definition
- Bit-sized fields with configurable byte and bit order
- Tagged enums, variable-length collections, and conditional fields
- Validation, context-aware types, and custom readers and writers
- Byte slices and `Read`/`Write` streams
- `std`, `no_std`, and fixed-size, no-allocation workflows with `DekuSize`

## Usage

*Compiler support: requires rustc 1.88+*

```toml
[dependencies]
deku = "0.20"
```

## Example

This example reads a red pixel in the 16-bit RGB565 format, adds blue, then
writes the resulting magenta pixel back to bytes.

```rust
use deku::prelude::*;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
struct Rgb565 {
    #[deku(bits = 5)]
    red: u8,
    #[deku(bits = 6)]
    green: u8,
    #[deku(bits = 5)]
    blue: u8,
}

fn main() {
    let input = [0xf8, 0x00];
    let (_, mut pixel) = Rgb565::from_bytes((&input, 0)).unwrap();

    assert_eq!(pixel, Rgb565 { red: 31, green: 0, blue: 0 });

    pixel.blue = 31;

    let output = pixel.to_bytes().unwrap();
    assert_eq!(output, [0xf8, 0x1f]);
}
```

The attributes describe the binary layout; `DekuRead` and `DekuWrite` generate
the matching operations in both directions.

See the [API documentation](https://docs.rs/deku),
[`#[deku]` attribute reference](https://docs.rs/deku/latest/deku/attributes/),
and [examples](https://github.com/sharksforarms/deku/tree/master/examples) for
complete protocol layouts and more advanced use cases.

## Cargo features

The default feature set selects `std`, `bits`, and `descriptive-errors`.
`alloc` is enabled by both `std` and `descriptive-errors`.

| Feature | Default | Description |
| --- | --- | --- |
| `std` | Yes | Standard library integration; also enables `alloc` |
| `alloc` | Indirectly | Heap-backed types such as `Vec` and allocating APIs such as `to_bytes` |
| `bits` | Yes | Bit-sized and non-byte-aligned fields, including `#[deku(bits = ...)]` |
| `descriptive-errors` | Yes | Detailed, dynamically formatted error messages; requires `alloc` |
| `logging` | No | Trace-level read and write diagnostics through the `log` crate |

Deku supports `no_std` with or without allocation. Disable the default features
for a minimal no-allocation build:

```toml
[dependencies]
deku = { version = "0.20", default-features = false }
```

Features can be combined as needed. For example, enable `bits` for bit-level
formats and `alloc` for heap-backed types:

```toml
[dependencies]
deku = { version = "0.20", default-features = false, features = ["bits", "alloc"] }
```

## Development

Development commands are defined in the `Justfile` and run in Docker. To run
the build, test, example, no_std, and WebAssembly checks used by CI (stable is
the default):

```sh
docker compose build
docker compose run --rm build just ci
```

Formatting and Clippy checks run with stable Rust:

```sh
docker compose run --rm build just lint
```

Select another pipeline toolchain with `DEKU_TOOLCHAIN=msrv` or
`DEKU_TOOLCHAIN=beta`:

```sh
docker compose run --rm -e DEKU_TOOLCHAIN=msrv build just ci
```

To list available commands:

```sh
docker compose run --rm build just
```

## License

Licensed under either the
[MIT license](https://github.com/sharksforarms/deku/blob/master/LICENSE-MIT) or
[Apache License 2.0](https://github.com/sharksforarms/deku/blob/master/LICENSE-APACHE).
