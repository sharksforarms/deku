//! Instruction counts for instruction forms that a binary analysis tool reads
//! from machine code: one 16- or 32-bit word per read, many times over.
//!
//! The forms come from hereticus, which reads PowerPC words most significant
//! bit first through a `bit_field` for each flag of a branch, and reads and
//! rewrites the ARM and Thumb calls of the xz branch filters as little-endian
//! words in `Lsb0`, with an `assert_eq` on each opcode. The ARM64 `.xdata`
//! header of a PE file is a little-endian `Lsb0` word with no assertion.
//!
//! Each bench moves `WORDS` words, so divide by `WORDS` for the cost of one.
//! Reads go through `from_bytes`, which is how a tool reads a word of a
//! larger image, or through a reader of the word's own bytes, as a decoder
//! that takes one word at a time does. Writes go through `to_slice`, which
//! rewrites a word in place. See `benches/bebits_callgrind.rs` for how to
//! read these counts.
use std::hint::black_box;

use deku::prelude::*;
use deku::reader::Reader;
use gungraun::{library_benchmark, library_benchmark_group, main};
use no_std_io::io::{Read, Seek};

/// The words each bench reads or writes.
const WORDS: usize = 1024;

/// The PowerPC D form, as `addi` and `stwu` hold it: four fields that fill
/// one big-endian word.
#[derive(Clone, Copy, Debug, PartialEq, Eq, DekuRead)]
#[deku(endian = "big")]
struct DForm {
    #[deku(bits = 6)]
    opcode: u8,
    #[deku(bits = 5)]
    rt: u8,
    #[deku(bits = 5)]
    ra: u8,
    #[deku(bits = 16)]
    d: u16,
}

/// One bit of the BO field of a PowerPC conditional branch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, DekuRead, DekuBitField)]
#[deku(id_type = "u8", bits = 1, endian = "big", ctx = "_: deku::ctx::Endian")]
enum Flag {
    #[deku(id = 0)]
    Clear,
    #[deku(id = 1)]
    Set,
}

/// The PowerPC B form of a conditional branch, with the four flags of its
/// BO field read as enums.
#[derive(Clone, Copy, Debug, PartialEq, Eq, DekuRead)]
#[deku(endian = "big")]
struct ConditionalBranch {
    #[deku(bits = 6)]
    opcode: u8,
    #[deku(bit_field)]
    bit: Flag,
    #[deku(bit_field)]
    when: Flag,
    #[deku(bit_field)]
    count: Flag,
    #[deku(bit_field)]
    until: Flag,
    #[deku(bits = 1)]
    hint: u8,
    #[deku(bits = 5)]
    bi: u8,
    #[deku(bits = 16)]
    rest: u16,
}

/// The word that opens the `.xdata` of an ARM64 entry of a PE exception
/// directory: six little-endian fields in `Lsb0`, none of them checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "little", bit_order = "lsb")]
struct Arm64XdataHeader {
    #[deku(bits = 18)]
    function_length: u32,
    #[deku(bits = 2)]
    version: u8,
    #[deku(bits = 1)]
    has_exception_data: u8,
    #[deku(bits = 1)]
    packs_one_epilog: u8,
    #[deku(bits = 5)]
    epilog_count: u8,
    #[deku(bits = 5)]
    code_words: u8,
}

/// The top byte of an ARM `bl` that always runs.
const ARM_BL: u8 = 0xeb;

/// An ARM `bl`: a 24-bit target in words, then the opcode in the top byte of
/// the little-endian word.
#[derive(Clone, Copy, Debug, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "little", bit_order = "lsb")]
struct ArmCall {
    #[deku(bits = 24)]
    offset: u32,
    #[deku(assert_eq = "ARM_BL")]
    opcode: u8,
}

/// The prefixes of the two halfwords of a Thumb `bl`.
const THUMB_BL_HIGH: u8 = 0b11110;
const THUMB_BL_LOW: u8 = 0b11111;

/// A Thumb `bl`: two little-endian halfwords, each with 11 bits of the target
/// and a 5-bit prefix.
#[derive(Clone, Copy, Debug, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "little", bit_order = "lsb")]
struct ThumbCall {
    #[deku(bits = 11)]
    high: u16,
    #[deku(bits = 5, assert_eq = "THUMB_BL_HIGH")]
    high_prefix: u8,
    #[deku(bits = 11)]
    low: u16,
    #[deku(bits = 5, assert_eq = "THUMB_BL_LOW")]
    low_prefix: u8,
}

/// `WORDS` big-endian words, each `opcode` in its top 6 bits over a pattern
/// that changes from word to word.
fn powerpc_words(opcode: u32) -> Vec<u8> {
    (0..WORDS as u32)
        .flat_map(|i| ((opcode << 26) | (i.wrapping_mul(0x9e37_79b9) >> 6)).to_be_bytes())
        .collect()
}

fn d_forms() -> Vec<u8> {
    // `addi`.
    powerpc_words(14)
}

fn conditional_branches() -> Vec<u8> {
    // `bc`.
    powerpc_words(16)
}

fn xdata_headers() -> Vec<u8> {
    (0..WORDS as u32)
        .flat_map(|i| i.wrapping_mul(0x9e37_79b9).to_le_bytes())
        .collect()
}

fn arm_calls() -> Vec<u8> {
    (0..WORDS as u32)
        .flat_map(|i| ((u32::from(ARM_BL) << 24) | (i * 3)).to_le_bytes())
        .collect()
}

fn thumb_calls() -> Vec<u8> {
    (0..WORDS as u16)
        .flat_map(|i| {
            let high = (u16::from(THUMB_BL_HIGH) << 11) | (i & 0x7ff);
            let low = (u16::from(THUMB_BL_LOW) << 11) | (i.wrapping_mul(7) & 0x7ff);
            [high.to_le_bytes(), low.to_le_bytes()]
        })
        .flatten()
        .collect()
}

/// A form folded into one number, so that a read cannot be optimized out.
trait Fold {
    fn fold(&self) -> u64;
}

impl Fold for DForm {
    fn fold(&self) -> u64 {
        u64::from(self.opcode) + u64::from(self.rt) + u64::from(self.ra) + u64::from(self.d)
    }
}

impl Fold for ConditionalBranch {
    fn fold(&self) -> u64 {
        let flags = [self.bit, self.when, self.count, self.until];
        let set = flags.iter().filter(|&&flag| flag == Flag::Set).count() as u64;
        set + u64::from(self.opcode)
            + u64::from(self.hint)
            + u64::from(self.bi)
            + u64::from(self.rest)
    }
}

impl Fold for Arm64XdataHeader {
    fn fold(&self) -> u64 {
        u64::from(self.function_length)
            + u64::from(self.version)
            + u64::from(self.has_exception_data)
            + u64::from(self.packs_one_epilog)
            + u64::from(self.epilog_count)
            + u64::from(self.code_words)
    }
}

impl Fold for ArmCall {
    fn fold(&self) -> u64 {
        u64::from(self.offset) + u64::from(self.opcode)
    }
}

impl Fold for ThumbCall {
    fn fold(&self) -> u64 {
        u64::from(self.high)
            + u64::from(self.low)
            + u64::from(self.high_prefix)
            + u64::from(self.low_prefix)
    }
}

/// Reads each `T` of `words`, one word at a time, as a scan of an image
/// does.
///
/// Callgrind toggles its count where a function whose name matches the
/// bench's begins and ends. A closure of the bench puts the bench's name in
/// the name of this function, which then turns the count off, so `T` folds
/// itself.
fn read_each<T>(words: &[u8]) -> u64
where
    T: for<'a> DekuContainerRead<'a> + Fold,
{
    let mut sum: u64 = 0;
    for word in words.as_chunks::<4>().0 {
        let (_, value) =
            T::from_bytes((black_box(&word[..]), 0)).expect("each word holds the form");
        sum = sum.wrapping_add(value.fold());
    }
    sum
}

/// Reads each `T` of `words` through a reader of its own 4 bytes, which
/// `reader` makes, as a decoder that takes one word at a time does.
fn read_each_through<'w, T, R>(words: &'w [u8], reader: fn(&'w [u8]) -> R) -> u64
where
    T: for<'a> DekuReader<'a> + Fold,
    R: Read + Seek,
{
    let mut sum: u64 = 0;
    for word in words.as_chunks::<4>().0 {
        let mut bytes = reader(black_box(&word[..]));
        let mut reader = Reader::new(&mut bytes);
        let value = T::from_reader_with_ctx(&mut reader, ()).expect("each word holds the form");
        sum = sum.wrapping_add(value.fold());
    }
    sum
}

fn cursor(word: &[u8]) -> no_std_io::io::Cursor<&[u8]> {
    no_std_io::io::Cursor::new(word)
}

#[library_benchmark]
#[bench::words(d_forms())]
fn powerpc_d_form_cursor_x1024(words: Vec<u8>) -> u64 {
    black_box(read_each_through::<DForm, _>(black_box(&words), cursor))
}

#[library_benchmark]
#[bench::words(conditional_branches())]
fn powerpc_conditional_branch_cursor_x1024(words: Vec<u8>) -> u64 {
    black_box(read_each_through::<ConditionalBranch, _>(
        black_box(&words),
        cursor,
    ))
}

#[library_benchmark]
#[bench::words(d_forms())]
fn powerpc_d_form_x1024(words: Vec<u8>) -> u64 {
    black_box(read_each::<DForm>(black_box(&words)))
}

#[library_benchmark]
#[bench::words(conditional_branches())]
fn powerpc_conditional_branch_x1024(words: Vec<u8>) -> u64 {
    black_box(read_each::<ConditionalBranch>(black_box(&words)))
}

#[library_benchmark]
#[bench::words(xdata_headers())]
fn arm64_xdata_header_x1024(words: Vec<u8>) -> u64 {
    black_box(read_each::<Arm64XdataHeader>(black_box(&words)))
}

#[library_benchmark]
#[bench::words(arm_calls())]
fn arm_call_x1024(words: Vec<u8>) -> u64 {
    black_box(read_each::<ArmCall>(black_box(&words)))
}

#[library_benchmark]
#[bench::words(thumb_calls())]
fn thumb_call_x1024(words: Vec<u8>) -> u64 {
    black_box(read_each::<ThumbCall>(black_box(&words)))
}

#[library_benchmark]
#[bench::words(xdata_headers())]
fn arm64_xdata_header_write_x1024(mut words: Vec<u8>) -> Vec<u8> {
    for (i, word) in words.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let header = Arm64XdataHeader {
            function_length: (i as u32).wrapping_mul(5) & 0x3_ffff,
            version: 0,
            has_exception_data: (i & 1) as u8,
            packs_one_epilog: 1,
            epilog_count: (i & 0x1f) as u8,
            code_words: (i >> 5 & 0x1f) as u8,
        };
        black_box(&header)
            .to_slice(word)
            .expect("a header fills its word");
    }
    black_box(words)
}

// The writes of the calls rewrite the target of each call in place, as the
// undo of an xz branch filter does.
#[library_benchmark]
#[bench::words(arm_calls())]
fn arm_call_write_x1024(mut words: Vec<u8>) -> Vec<u8> {
    for (i, word) in words.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let call = ArmCall {
            offset: (i as u32).wrapping_mul(5) & 0x00ff_ffff,
            opcode: ARM_BL,
        };
        black_box(&call)
            .to_slice(word)
            .expect("a call fills its word");
    }
    black_box(words)
}

#[library_benchmark]
#[bench::words(thumb_calls())]
fn thumb_call_write_x1024(mut words: Vec<u8>) -> Vec<u8> {
    for (i, word) in words.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let target = (i as u16).wrapping_mul(5);
        let call = ThumbCall {
            high: target & 0x7ff,
            high_prefix: THUMB_BL_HIGH,
            low: target.rotate_left(3) & 0x7ff,
            low_prefix: THUMB_BL_LOW,
        };
        black_box(&call)
            .to_slice(word)
            .expect("a call fills its word");
    }
    black_box(words)
}

library_benchmark_group!(
    name = read_group,
    benchmarks = [
        powerpc_d_form_x1024,
        powerpc_conditional_branch_x1024,
        powerpc_d_form_cursor_x1024,
        powerpc_conditional_branch_cursor_x1024,
        arm_call_x1024,
        thumb_call_x1024,
        arm64_xdata_header_x1024,
    ]
);

library_benchmark_group!(
    name = write_group,
    benchmarks = [
        arm_call_write_x1024,
        thumb_call_write_x1024,
        arm64_xdata_header_write_x1024
    ]
);

main!(library_benchmark_groups = read_group, write_group);
