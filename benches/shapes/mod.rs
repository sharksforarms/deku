//! Shapes shared by the `deku` benches, which measure the same types under two
//! harnesses: criterion for wall time, gungraun/callgrind for instruction
//! counts. The definitions live here so that the two harnesses cannot drift
//! apart and report numbers for different types.

// Each bench target compiles this module on its own and uses a part of it, so
// the items the other target uses look dead here.
#![allow(dead_code)]

use deku::prelude::*;

/// `DekuBits` needs the `bits` feature, which the criterion target does not
/// require.
#[cfg(feature = "bits")]
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
pub struct DekuBits {
    #[deku(bits = 1)]
    pub data_01: u8,
    #[deku(bits = 2)]
    pub data_02: u8,
    #[deku(bits = 5)]
    pub data_03: u8,
}

/// The next three types are a matched set: the same 1/7/56 or 1/31 layout, with
/// only the endianness and the container width different. Read them together.
/// A difference between them is a difference in the bit path, not in the shape.
#[cfg(feature = "bits")]
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
pub struct DekuBitsBeU64 {
    #[deku(bits = 1)]
    pub data_01: u64,
    #[deku(bits = 7)]
    pub data_02: u64,
    #[deku(bits = 56)]
    pub data_03: u64,
}

#[cfg(feature = "bits")]
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
pub struct DekuBitsBeU32 {
    #[deku(bits = 1)]
    pub data_01: u32,
    #[deku(bits = 31)]
    pub data_02: u32,
}

#[cfg(feature = "bits")]
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct DekuBitsLeU64 {
    #[deku(bits = 1)]
    pub data_01: u64,
    #[deku(bits = 7)]
    pub data_02: u64,
    #[deku(bits = 56)]
    pub data_03: u64,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
pub struct DekuBytes {
    pub data_00: u8,
    pub data_01: u16,
    pub data_02: u32,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(id_type = "u8")]
pub enum DekuEnum {
    #[deku(id = "0x01")]
    VariantA(u8),
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
pub struct DekuVec {
    pub count: u8,
    #[deku(count = "count")]
    pub data: Vec<u8>,
}

/// Reads to the end of the input.
#[derive(DekuRead, DekuWrite)]
pub struct AllWrapper {
    #[deku(read_all)]
    pub data: Vec<u8>,
}

/// A count of `u8`, which deku reads with a specialized path.
#[derive(DekuRead, DekuWrite)]
pub struct CountWrapper {
    #[deku(count = "1500")]
    pub data: Vec<u8>,
}

/// A count of `u16`, which has no specialized path. The pair with
/// [`CountWrapper`] shows what the specialization is worth.
#[derive(DekuRead, DekuWrite)]
pub struct CountNonSpecialize {
    #[deku(count = "(1500/2)")]
    pub data: Vec<u16>,
}
