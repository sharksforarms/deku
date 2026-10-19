#![cfg(all(feature = "alloc", feature = "bits"))]
//! `bit_field`: folding `DekuBitField` fields into a neighbouring bit-field run.
//!
//! The contract under test is that batching is invisible: a struct with
//! `bit_field` fields must read and write exactly what the same struct without
//! them does.

use deku::prelude::*;

/// 1-bit flag, the shape bit-packed headers are full of. Endianness comes from
/// context so the flag can sit in an `endian = "big"` struct.
#[derive(Copy, Clone, Debug, PartialEq, DekuRead, DekuWrite, DekuBitField)]
#[deku(
    id_type = "u8",
    bits = 1,
    endian = "endian",
    ctx = "endian: deku::ctx::Endian"
)]
enum Flag {
    #[deku(id = 0b0)]
    Off,
    #[deku(id = 0b1)]
    On,
}

/// 2 bits, and deliberately not every id is assigned: `0b10` must stay an error.
#[derive(Copy, Clone, Debug, PartialEq, DekuRead, DekuWrite, DekuBitField)]
#[deku(
    id_type = "u8",
    bits = 2,
    endian = "endian",
    ctx = "endian: deku::ctx::Endian"
)]
enum Mode {
    #[deku(id = 0b00)]
    Zero,
    #[deku(id = 0b01)]
    One,
    #[deku(id = 0b11)]
    Three,
}

/// A 32-bit id, wide enough to push a run past 64 bits.
#[derive(Copy, Clone, Debug, PartialEq, DekuRead, DekuWrite, DekuBitField)]
#[deku(id_type = "u32", endian = "big", ctx = "_endian: deku::ctx::Endian")]
enum Word {
    #[deku(id = 0xDEAD_BEEF)]
    Dead,
    #[deku(id = 0x0000_0001)]
    One,
}

#[test]
fn bit_field_is_derived_for_flag_enums() {
    assert_eq!(<Flag as DekuBitField>::BITS, 1);
    assert_eq!(<Mode as DekuBitField>::BITS, 2);
    assert_eq!(<Word as DekuBitField>::BITS, 32);

    assert_eq!(Flag::from_bit_run(0b1).unwrap(), Flag::On);
    assert_eq!(Flag::On.to_bit_run().unwrap(), 0b1);

    // Bits above the field's own are not its business.
    assert_eq!(Flag::from_bit_run(0b1111_1110).unwrap(), Flag::Off);

    assert_eq!(Mode::from_bit_run(0b11).unwrap(), Mode::Three);
    assert_eq!(Mode::Three.to_bit_run().unwrap(), 0b11);
}

/// An unassigned id fails the same way through the trait as through the reader.
#[test]
fn unassigned_id_is_the_readers_error() {
    let via_trait = Mode::from_bit_run(0b10).unwrap_err();
    let mut cursor = deku::no_std_io::Cursor::new([0b1000_0000u8]);
    let mut reader = deku::reader::Reader::new(&mut cursor);
    let via_reader = Mode::from_reader_with_ctx(&mut reader, deku::ctx::Endian::Big).unwrap_err();
    assert_eq!(format!("{via_trait:?}"), format!("{via_reader:?}"));
}

macro_rules! header {
    ($name:ident $(, $bf:ident)?) => {
        #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
        #[deku(endian = "big")]
        struct $name {
            #[deku(bits = 2)]
            version: u8,
            #[deku(bits = 10)]
            id: u16,
            #[deku(bits = 3)]
            channel: u8,
            $(#[deku($bf)])?
            ocf: Flag,
            counter: u8,
            vc_counter: u8,
            $(#[deku($bf)])?
            secondary: Flag,
            $(#[deku($bf)])?
            sync: Flag,
            $(#[deku($bf)])?
            order: Flag,
            $(#[deku($bf)])?
            mode: Mode,
            #[deku(bits = 11)]
            pointer: u16,
        }
    };
}

// Same fields, once batched and once not, so the two can be compared directly.
header!(Batched, bit_field);
header!(Plain);

/// Every bit pattern of the 6-octet header, sampled across the whole space, must
/// decode the same batched and not, and re-encode to the input.
#[test]
fn batched_matches_unbatched() {
    let mut x: u32 = 0x1234_5678;
    for _ in 0..20_000 {
        let mut wire = [0u8; 6];
        for b in wire.iter_mut() {
            x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            *b = (x >> 24) as u8;
        }

        let batched = Batched::from_bytes((&wire, 0));
        let plain = Plain::from_bytes((&wire, 0));

        match (batched, plain) {
            (Ok(((b_rest, b_off), b)), Ok(((p_rest, p_off), p))) => {
                assert_eq!(b_rest, p_rest);
                assert_eq!(b_off, p_off);
                assert_eq!(b.version, p.version);
                assert_eq!(b.id, p.id);
                assert_eq!(b.channel, p.channel);
                assert_eq!(b.ocf, p.ocf);
                assert_eq!(b.counter, p.counter);
                assert_eq!(b.vc_counter, p.vc_counter);
                assert_eq!(b.secondary, p.secondary);
                assert_eq!(b.sync, p.sync);
                assert_eq!(b.order, p.order);
                assert_eq!(b.mode, p.mode);
                assert_eq!(b.pointer, p.pointer);

                assert_eq!(b.to_bytes().unwrap(), wire);
                assert_eq!(p.to_bytes().unwrap(), wire);
            }
            // `Mode` rejects `0b10`, so both sides must reject the same inputs.
            (Err(b), Err(p)) => assert_eq!(format!("{b:?}"), format!("{p:?}")),
            (b, p) => panic!("batched and unbatched disagree on {wire:02x?}: {b:?} vs {p:?}"),
        }
    }
}

/// A run that ends on a `bit_field` still leaves the cursor where the unbatched
/// reads would have, so what follows it lines up.
#[test]
fn run_followed_by_unbatchable_field() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(endian = "big")]
    struct Trailing {
        #[deku(bits = 4)]
        head: u8,
        #[deku(bit_field)]
        a: Flag,
        #[deku(bit_field)]
        b: Flag,
        #[deku(bit_field)]
        mode: Mode,
        #[deku(count = "2")]
        rest: Vec<u8>,
    }

    let wire = [0b1010_1101u8, 0xAA, 0xBB];
    let (_, v) = Trailing::from_bytes((&wire, 0)).unwrap();
    assert_eq!(
        v,
        Trailing {
            head: 0b1010,
            a: Flag::On,
            b: Flag::On,
            mode: Mode::One,
            rest: vec![0xAA, 0xBB],
        }
    );
    assert_eq!(v.to_bytes().unwrap(), wire);
}

/// A run that both starts and ends on a primitive, with the enums in the middle.
#[test]
fn enums_between_primitives() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(endian = "big")]
    struct Sandwich {
        #[deku(bits = 3)]
        head: u8,
        #[deku(bit_field)]
        a: Flag,
        #[deku(bit_field)]
        b: Flag,
        #[deku(bits = 3)]
        tail: u8,
    }

    // head 101, a 1, b 0, tail 110
    let wire = [0b1011_0110u8];
    let (_, v) = Sandwich::from_bytes((&wire, 0)).unwrap();
    assert_eq!(
        v,
        Sandwich {
            head: 0b101,
            a: Flag::On,
            b: Flag::Off,
            tail: 0b110,
        }
    );
    assert_eq!(v.to_bytes().unwrap(), wire);
}

/// A run of enums alone, with no primitive to anchor it.
#[test]
fn all_enum_run() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(endian = "big")]
    struct Flags {
        #[deku(bit_field)]
        a: Flag,
        #[deku(bit_field)]
        b: Flag,
        #[deku(bit_field)]
        c: Flag,
        #[deku(bit_field)]
        d: Flag,
        #[deku(bit_field)]
        e: Flag,
        #[deku(bit_field)]
        f: Flag,
        #[deku(bit_field)]
        g: Flag,
        #[deku(bit_field)]
        h: Flag,
    }

    let wire = [0b1100_1010u8];
    let (_, v) = Flags::from_bytes((&wire, 0)).unwrap();
    assert_eq!(
        v,
        Flags {
            a: Flag::On,
            b: Flag::On,
            c: Flag::Off,
            d: Flag::Off,
            e: Flag::On,
            f: Flag::Off,
            g: Flag::On,
            h: Flag::Off,
        }
    );
    assert_eq!(v.to_bytes().unwrap(), wire);
}

/// A wider enum id, where byte order does matter and so must be explicit.
#[test]
fn wide_id_enum() {
    #[derive(Copy, Clone, Debug, PartialEq, DekuRead, DekuWrite, DekuBitField)]
    #[deku(
        id_type = "u16",
        bits = 12,
        endian = "big",
        ctx = "_endian: deku::ctx::Endian"
    )]
    enum Wide {
        #[deku(id = 0x123)]
        A,
        #[deku(id = 0xABC)]
        B,
    }

    assert_eq!(<Wide as DekuBitField>::BITS, 12);

    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(endian = "big")]
    struct Holder {
        #[deku(bits = 4)]
        head: u8,
        #[deku(bit_field)]
        wide: Wide,
    }

    let wire = [0x5Au8, 0xBC];
    let (_, v) = Holder::from_bytes((&wire, 0)).unwrap();
    assert_eq!(
        v,
        Holder {
            head: 0x5,
            wide: Wide::B,
        }
    );
    assert_eq!(v.to_bytes().unwrap(), wire);
}

/// Fields that are not bit fields sit next to a run without joining it: only a
/// field marked `bit_field` has to implement the trait.
#[test]
fn other_types_do_not_join_the_run() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(endian = "big", ctx = "_endian: deku::ctx::Endian")]
    struct Inner {
        x: u8,
    }

    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(endian = "big")]
    struct Mixed {
        #[deku(bits = 7)]
        head: u8,
        #[deku(bit_field)]
        flag: Flag,
        signed: i16,
        inner: Inner,
    }

    let wire = [0b1010_1011u8, 0xFF, 0xFE, 0x42];
    let (_, v) = Mixed::from_bytes((&wire, 0)).unwrap();
    assert_eq!(
        v,
        Mixed {
            head: 0b101_0101,
            flag: Flag::On,
            signed: -2,
            inner: Inner { x: 0x42 },
        }
    );
    assert_eq!(v.to_bytes().unwrap(), wire);
}

/// A run the macro grouped at a guessed width but that is wider than 64 bits
/// once the trait's width is known still works, through the individual reads.
#[test]
fn a_run_too_wide_falls_back() {
    macro_rules! holder {
        ($name:ident $(, $bf:ident)?) => {
            #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
            #[deku(endian = "big")]
            struct $name {
                #[deku(bits = 4)]
                head: u8,
                #[deku(bits = 36)]
                middle: u64,
                // 4 + 36 + 32 = 72 bits.
                $(#[deku($bf)])?
                word: Word,
            }
        };
    }
    holder!(Batched, bit_field);
    holder!(Plain);

    let wire = [0xA1, 0x23, 0x45, 0x67, 0x89, 0xDE, 0xAD, 0xBE, 0xEF];
    let (_, b) = Batched::from_bytes((&wire, 0)).unwrap();
    let (_, p) = Plain::from_bytes((&wire, 0)).unwrap();
    assert_eq!((b.head, b.middle, b.word), (p.head, p.middle, p.word));
    assert_eq!(b.word, Word::Dead);
    assert_eq!(b.to_bytes().unwrap(), wire);
}

/// A whole-byte enum among whole-byte fields keeps the individual reads, which
/// are faster for plain bytes, and still matches the unmarked struct.
#[test]
fn a_whole_byte_run_keeps_the_individual_reads() {
    #[derive(Copy, Clone, Debug, PartialEq, DekuRead, DekuWrite, DekuBitField)]
    #[deku(id_type = "u8", endian = "endian", ctx = "endian: deku::ctx::Endian")]
    enum Kind {
        #[deku(id = 0x01)]
        A,
        #[deku(id = 0x02)]
        B,
    }

    macro_rules! holder {
        ($name:ident $(, $bf:ident)?) => {
            #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
            #[deku(endian = "big")]
            struct $name {
                head: u8,
                $(#[deku($bf)])?
                kind: Kind,
                tail: u16,
            }
        };
    }
    holder!(Batched, bit_field);
    holder!(Plain);

    for wire in [[0x10u8, 0x02, 0xAB, 0xCD], [0x10, 0x03, 0xAB, 0xCD]] {
        match (
            Batched::from_bytes((&wire, 0)),
            Plain::from_bytes((&wire, 0)),
        ) {
            (Ok((_, b)), Ok((_, p))) => {
                assert_eq!((b.head, b.kind, b.tail), (p.head, p.kind, p.tail));
                assert_eq!(b.to_bytes().unwrap(), wire);
            }
            (Err(b), Err(p)) => assert_eq!(format!("{b:?}"), format!("{p:?}")),
            (b, p) => panic!("disagree on {wire:02x?}: {b:?} vs {p:?}"),
        }
    }
}

/// A `bit_field` typed as a parameter of the container, bound by the caller.
#[test]
fn generic_bit_field() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(
        endian = "big",
        bound = "T: DekuBitField",
        read_bound = "for<'a> T: DekuReader<'a, deku::ctx::Endian>",
        write_bound = "T: DekuWriter<deku::ctx::Endian>"
    )]
    struct Generic<T> {
        #[deku(bits = 6)]
        head: u8,
        #[deku(bit_field)]
        mode: T,
    }

    let wire = [0b1111_0111u8];
    let (_, v) = Generic::<Mode>::from_bytes((&wire, 0)).unwrap();
    assert_eq!(
        v,
        Generic {
            head: 0b11_1101,
            mode: Mode::Three,
        }
    );
    assert_eq!(v.to_bytes().unwrap(), wire);
}

/// A nibble with a hand-written impl, whose `to_bit_run` can fail.
#[derive(Copy, Clone, Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "endian", ctx = "endian: deku::ctx::Endian")]
struct Nibble(#[deku(bits = 4)] u8);

impl DekuBitField for Nibble {
    const BITS: usize = 4;

    fn from_bit_run(raw: u64) -> Result<Self, DekuError> {
        Ok(Self((raw & 0xF) as u8))
    }

    fn to_bit_run(&self) -> Result<u64, DekuError> {
        // The same check the derived write of `bits = 4` performs.
        deku::writer::check_bit_size::<false>(u64::from(self.0), 4)?;
        Ok(u64::from(self.0))
    }
}

/// With two bad fields in one run, the batched write reports the first one, as
/// the individual writes do.
#[test]
fn write_reports_the_first_failing_field() {
    macro_rules! holder {
        ($name:ident $(, $bf:ident)?) => {
            #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
            #[deku(endian = "big")]
            struct $name {
                $(#[deku($bf)])?
                nibble: Nibble,
                #[deku(bits = 4)]
                tail: u8,
            }
        };
    }
    holder!(Batched, bit_field);
    holder!(Plain);

    // Both fields overflow 4 bits, by different amounts, so the message tells
    // which one was reported.
    let batched = Batched {
        nibble: Nibble(0x1F),
        tail: 0xFF,
    }
    .to_bytes()
    .unwrap_err();
    let plain = Plain {
        nibble: Nibble(0x1F),
        tail: 0xFF,
    }
    .to_bytes()
    .unwrap_err();
    assert_eq!(format!("{batched:?}"), format!("{plain:?}"));
}

/// A hand-written impl with a width outside 1 to 64 is not batched at all.
#[test]
fn a_zero_width_falls_back() {
    #[derive(Copy, Clone, Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(ctx = "_endian: deku::ctx::Endian")]
    struct Nothing;

    impl DekuBitField for Nothing {
        const BITS: usize = 0;

        fn from_bit_run(_: u64) -> Result<Self, DekuError> {
            Ok(Self)
        }

        fn to_bit_run(&self) -> Result<u64, DekuError> {
            Ok(0)
        }
    }

    // Batched, the two would be one read of zero bits.
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(endian = "big")]
    struct Holder {
        #[deku(bit_field)]
        a: Nothing,
        #[deku(bit_field)]
        b: Nothing,
        signed: i8,
    }

    let wire = [0xFE];
    let (_, v) = Holder::from_bytes((&wire, 0)).unwrap();
    assert_eq!(
        v,
        Holder {
            a: Nothing,
            b: Nothing,
            signed: -2,
        }
    );
    assert_eq!(v.to_bytes().unwrap(), wire);
}

/// A string id, deku's other way to write a literal, and a wide id taking its
/// endian from context, which a big-endian struct fills with `Big`.
#[test]
fn string_ids_and_a_wide_id_from_context() {
    #[derive(Copy, Clone, Debug, PartialEq, DekuRead, DekuWrite, DekuBitField)]
    #[deku(
        id_type = "u16",
        bits = 12,
        endian = "endian",
        ctx = "endian: deku::ctx::Endian"
    )]
    enum Wide {
        #[deku(id = "0x123")]
        A,
        #[deku(id = "0xABC")]
        B,
    }

    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(endian = "big")]
    struct Holder {
        #[deku(bits = 4)]
        head: u8,
        #[deku(bit_field)]
        wide: Wide,
    }

    let wire = [0x51u8, 0x23];
    let (_, v) = Holder::from_bytes((&wire, 0)).unwrap();
    assert_eq!(
        v,
        Holder {
            head: 0x5,
            wide: Wide::A,
        }
    );
    assert_eq!(v.to_bytes().unwrap(), wire);
}

/// A width far past 64 falls back like any other too-wide run, rather than
/// overflowing the sum of the widths at compile time.
#[test]
fn an_absurd_width_falls_back() {
    #[derive(Copy, Clone, Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(endian = "endian", ctx = "endian: deku::ctx::Endian")]
    struct Byte(u8);

    impl DekuBitField for Byte {
        const BITS: usize = usize::MAX;

        fn from_bit_run(raw: u64) -> Result<Self, DekuError> {
            Ok(Self(raw as u8))
        }

        fn to_bit_run(&self) -> Result<u64, DekuError> {
            Ok(self.0.into())
        }
    }

    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(endian = "big")]
    struct Holder {
        #[deku(bits = 4)]
        a: u8,
        #[deku(bit_field)]
        b: Byte,
        #[deku(bits = 4)]
        c: u8,
    }

    let wire = [0xAB, 0xCD];
    let (_, v) = Holder::from_bytes((&wire, 0)).unwrap();
    assert_eq!(
        v,
        Holder {
            a: 0xA,
            b: Byte(0xBC),
            c: 0xD,
        }
    );
    assert_eq!(v.to_bytes().unwrap(), wire);
}

/// The one documented difference: a run reads all of its bits before checking
/// any id, so input too short for the run is `Incomplete` even where its first
/// id alone is unassigned.
#[test]
fn short_input_is_incomplete_before_any_id_is_checked() {
    macro_rules! holder {
        ($name:ident $(, $bf:ident)?) => {
            #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
            #[deku(endian = "big")]
            struct $name {
                $(#[deku($bf)])?
                mode: Mode,
                #[deku(bits = 14)]
                rest: u16,
            }
        };
    }
    holder!(Batched, bit_field);
    holder!(Plain);

    // 0b10 is unassigned, and one byte is two short of the run.
    let wire = [0b1000_0000u8];
    let batched = Batched::from_bytes((&wire, 0)).unwrap_err();
    let plain = Plain::from_bytes((&wire, 0)).unwrap_err();
    assert!(matches!(batched, DekuError::Incomplete(_)), "{batched:?}");
    assert!(matches!(plain, DekuError::Parse(_)), "{plain:?}");

    // With the whole run present, both report the unassigned id.
    let wire = [0b1000_0000u8, 0];
    let batched = Batched::from_bytes((&wire, 0)).unwrap_err();
    let plain = Plain::from_bytes((&wire, 0)).unwrap_err();
    assert_eq!(format!("{batched:?}"), format!("{plain:?}"));
}

/// A hand-written `to_bit_run` returning bits above `BITS` breaks the trait's
/// contract, and the batched write's mask would hide it.
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "DekuBitField::to_bit_run returned bits above BITS")]
fn a_to_bit_run_past_its_width_is_caught_in_debug() {
    #[derive(Copy, Clone, Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(endian = "endian", ctx = "endian: deku::ctx::Endian")]
    struct Loose(#[deku(bits = 4)] u8);

    impl DekuBitField for Loose {
        const BITS: usize = 4;

        fn from_bit_run(raw: u64) -> Result<Self, DekuError> {
            Ok(Self((raw & 0xF) as u8))
        }

        fn to_bit_run(&self) -> Result<u64, DekuError> {
            Ok(u64::from(self.0))
        }
    }

    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(endian = "big")]
    struct Holder {
        #[deku(bits = 4)]
        head: u8,
        #[deku(bit_field)]
        loose: Loose,
    }

    let _ = Holder {
        head: 0,
        loose: Loose(0x1F),
    }
    .to_bytes();
}
