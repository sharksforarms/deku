# Deku

[![Latest Version](https://img.shields.io/crates/v/deku.svg)](https://crates.io/crates/deku)
[![Rust Documentation](https://docs.rs/deku/badge.svg)](https://docs.rs/deku)
[![Actions Status](https://github.com/sharksforarms/deku/workflows/CI/badge.svg)](https://github.com/sharksforarms/deku/actions)
[![codecov](https://codecov.io/gh/sharksforarms/deku/branch/master/graph/badge.svg)](https://codecov.io/gh/sharksforarms/deku)
[![discord](https://img.shields.io/discord/771034826119053363?label=discord&logo=discord)](https://discord.gg/3ANfVS)

Declarative binary reading and writing

This crate provides bit-level, symmetric, serialization/deserialization
implementations for structs and enums

## Why use Deku

**Productivity**: Deku will generate symmetric reader/writer functions for your type!
Avoid the requirement of writing redundant, error-prone parsing and writing code
for binary structs or network headers

## Usage

```toml
[dependencies]
deku = "0.10"
```

no_std:
```toml
[dependencies]
deku = { version = "0.10", default-features = false, features = ["alloc"] }
```

## Example

See [documentation](https://docs.rs/deku) or
[examples](https://github.com/sharksforarms/deku/tree/master/examples) folder for more!

Read big-endian data into a struct, modify a value, and write it

```rust
use deku::prelude::*;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
struct DekuTest {
    #[deku(bits = "4")]
    field_a: u8,
    #[deku(bits = "4")]
    field_b: u8,
    field_c: u16,
}

fn main() {
    let data: Vec<u8> = vec![0b0110_1001, 0xBE, 0xEF];
    let (_rest, mut val) = DekuTest::from_bytes((data.as_ref(), 0)).unwrap();
    assert_eq!(DekuTest {
        field_a: 0b0110,
        field_b: 0b1001,
        field_c: 0xBEEF,
    }, val);

    val.field_c = 0xC0FE;

    let data_out = val.to_bytes().unwrap();
    assert_eq!(vec![0b0110_1001, 0xC0, 0xFE], data_out);
}
```

## Changelog

See [CHANGELOG.md](https://github.com/sharksforarms/deku/blob/master/CHANGELOG.md)
