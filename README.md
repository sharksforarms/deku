# Deku

[![Latest Version](https://img.shields.io/crates/v/deku.svg)](https://crates.io/crates/deku)
[![Rust Documentation](https://docs.rs/deku/badge.svg)](https://docs.rs/deku)
[![Actions Status](https://github.com/sharksforarms/deku/workflows/CI/badge.svg)](https://github.com/sharksforarms/deku/actions)
[![codecov](https://codecov.io/gh/sharksforarms/deku/branch/master/graph/badge.svg)](https://codecov.io/gh/sharksforarms/deku)

Deku provides bit level serialization/deserialization proc-macros for structs

Under the hood, it uses [nom](https://crates.io/crates/nom) as the consumer or “Reader” and [bitvec](https://crates.io/crates/bitvec) as the “Writer”

## Usage

```toml
[dependencies]
deku = "0.1"
```

## Example

```rust
use deku::prelude::*;
use std::convert::TryFrom;

/// DekuTest Struct
//   0                   1                   2                   3
//   0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3
//  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//  |    field_a    |   field_b   |c|            field_d            | e |
//  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
// #[deku(endian = "little")] // By default it uses the system endianess, but can be overwritten
struct DekuTest {
    field_a: u8,
    #[deku(bits = "7")]
    field_b: u8,
    #[deku(bits = "1")]
    field_c: u8,
    #[deku(endian = "big")]
    field_d: u16,
    #[deku(bits = "2")]
    field_e: u8,
}

fn main() {
    let test_data: &[u8] = [0xAB, 0b1010010_1, 0xAB, 0xCD, 0b1100_0000].as_ref();

    let test_deku = DekuTest::try_from(test_data).unwrap();

    assert_eq!(
        test_deku,
        DekuTest {
            field_a: 0xAB,
            field_b: 0b0_1010010,
            field_c: 0b0000000_1,
            field_d: 0xCDAB,
            field_e: 0b0000_0011,
        }
    );

    let test_deku: Vec<u8> = test_deku.into();
    assert_eq!(test_data.to_vec(), test_deku);
}
```
