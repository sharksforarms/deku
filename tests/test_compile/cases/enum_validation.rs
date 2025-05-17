use deku::prelude::*;

// test required attributes
#[derive(DekuRead)]
enum Test1 {}

// test conflict `type` and `id_type`
#[derive(DekuRead)]
#[deku(id_type = "u8", id = "test")]
enum Test2 {}

// test conflict `id_type` and `id_pat`
#[derive(DekuRead)]
#[deku(id_type = "u8")]
enum Test3 {
    #[deku(id = "1", id_pat = "2..=3")]
    A(u8),
}

// test `id_type` only allowed on enum
#[derive(DekuRead)]
#[deku(id_type = "u8")]
struct Test4 {
    a: u8,
}

// test `bits` only allowed on enum
#[derive(DekuRead)]
#[deku(bits = 1)]
struct Test5 {
    a: u8,
}

// test `bytes` only allowed on enum
#[derive(DekuRead)]
#[deku(bits = 1)]
struct Test6 {
    a: u8,
}

// test `id_type` only allowed on enum
#[derive(DekuRead)]
#[deku(id_type = "test")]
struct Test7 {
    a: u8,
}

// test `bits` cannot be used with `id_type`
#[derive(DekuRead)]
#[deku(id_type = "test", bits = 4)]
enum Test8 {
    A,
}

// test `bytes` cannot be used with `id_type`
#[derive(DekuRead)]
#[deku(id_type = "test", bytes = 4)]
enum Test9 {
    A,
}

// test `type_id` cannot be `_`
#[derive(DekuRead)]
#[deku(id_type = "u8")]
enum Test10 {
    #[deku(id = "_")]
    A,
}

// test missing `id_type`
#[derive(DekuRead)]
#[deku(id_type = "u8")]
enum Test11 {
    #[deku(id = "1")]
    A,
    B(u8),
}

// Test id_pat id storage must not have attributes
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(id_type = "u8", bits = "2")]
pub enum Test12 {
    #[deku(id_pat = "_")]
    B(#[deku(bits = 7)] u8, #[deku(bits = 6)] u8),
}

// Test id_pat id storage must not have attributes
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(id_type = "u32", bytes = "2")]
pub enum Test13 {
    #[deku(id_pat = "_")]
    B(#[deku(bytes = 3)] u32, #[deku(bits = 6)] u8),
}

// Test id_pat id storage must not have attributes
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(id_type = "u32", bytes = "2", id_endian = "little")]
pub enum Test14 {
    #[deku(id_pat = "_")]
    B(#[deku(bytes = 2, endian = "big")] u32, #[deku(bits = 6)] u8),
}

// Test id_pat id storage must have matching types
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(id_type = "u32", bytes = "2")]
pub enum Test15 {
    #[deku(id_pat = "_")]
    B(u8, #[deku(bits = 6)] u8),
}

// Test id_pat id storage must have matching types
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(id_type = "[u8; 32]")]
pub enum Test16 {
    #[deku(id_pat = "_")]
    B(u16, #[deku(bits = 6)] u8),
}

// Test id_pat id storage must exist (read)
#[derive(PartialEq, Debug, DekuRead)]
#[deku(id_type = "u8")]
pub enum Test17 {
    #[deku(id_pat = "_")]
    B,
}

// Test cannot determine id write
#[derive(PartialEq, Debug, DekuWrite)]
#[deku(id_type = "u8")]
pub enum Test18 {
    #[deku(id_pat = "_")]
    B,
}

// Test #[repr(inttype)]
#[derive(PartialEq, Debug, DekuWrite)]
#[deku(id_type = "u8")]
pub enum Test19 {
    A = 0,
    #[deku(id_pat = "_")]
    B(u8),
}

// Require #[repr(u8)]
#[derive(Debug, Clone, DekuRead)]
#[deku(id_type = "u8")]
pub enum CommandU8 {
    Base = 0x00,
}

// id_type must match repr
#[derive(Debug, Clone, DekuRead)]
#[repr(u16)]
#[deku(id_type = "u8")]
pub enum CommandU8NonMatchingRead {
    Base = 0x00,
}

// Require #[repr(u16)]
#[derive(Debug, Clone, DekuRead)]
#[deku(id_type = "u16")]
pub enum CommandU16 {
    Base = 0x00,
}

// Require #[repr(u32)]
#[derive(Debug, Clone, DekuRead)]
#[deku(id_type = "u32")]
pub enum CommandU32 {
    Base = 0x00,
}

// Require #[repr(i32)]
#[derive(Debug, Clone, DekuRead)]
#[deku(id_type = "i32")]
pub enum CommandI32 {
    Base = 0x00,
}

// Require #[repr(u8)]
#[derive(Debug, Clone, DekuWrite)]
#[deku(id_type = "u8")]
pub enum CommandWU8 {
    Base = 0x00,
}

// id_type must match repr
#[derive(Debug, Clone, DekuWrite)]
#[repr(u16)]
#[deku(id_type = "u8")]
pub enum CommandU8NonMatchingWrite {
    Base = 0x00,
}

// Require #[repr(u16)]
#[derive(Debug, Clone, DekuWrite)]
#[deku(id_type = "u16")]
pub enum CommandWU16 {
    Base = 0x00,
}

// Require #[repr(u32)]
#[derive(Debug, Clone, DekuWrite)]
#[deku(id_type = "u32")]
pub enum CommandWU32 {
    Base = 0x00,
}

// Require #[repr(i32)]
#[derive(Debug, Clone, DekuWrite)]
#[deku(id_type = "i32")]
pub enum CommandWI32 {
    Base = 0x00,
}

fn main() {}
