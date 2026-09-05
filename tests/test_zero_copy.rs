use deku::prelude::*;
#[cfg(feature = "alloc")]
use std::borrow::Cow;
#[cfg(feature = "std")]
use std::collections::{HashMap, HashSet};

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct Borrowed<'a> {
    count: u8,
    #[deku(count = "count")]
    data: &'a [u8],
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct NestedBorrowed<'a> {
    prefix: u8,
    #[deku(bytes_read = "2")]
    data: &'a [u8],
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[cfg(feature = "alloc")]
struct RepeatedBorrowed<'a> {
    #[deku(count = "2")]
    records: Vec<NestedBorrowed<'a>>,
}

#[derive(Debug, PartialEq, DekuRead)]
#[cfg(feature = "std")]
struct MapValue<'a> {
    #[deku(bytes_read = "2")]
    data: &'a [u8],
}

#[derive(Debug, PartialEq, DekuRead)]
#[cfg(feature = "std")]
struct MapBorrowed<'a> {
    #[deku(count = "2")]
    values: HashMap<u8, MapValue<'a>>,
}

#[derive(Debug, PartialEq, Eq, Hash, DekuRead)]
#[cfg(feature = "std")]
struct SetValue<'a> {
    key: u8,
    #[deku(bytes_read = "2")]
    data: &'a [u8],
}

#[derive(Debug, PartialEq, DekuRead)]
#[cfg(feature = "std")]
struct SetBorrowed<'a> {
    #[deku(count = "2")]
    values: HashSet<SetValue<'a>>,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(id_type = "u8")]
enum BorrowedEnum<'a> {
    #[deku(id = "1")]
    Data(#[deku(bytes_read = "2")] &'a [u8]),
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[cfg(feature = "alloc")]
struct CowBorrowed<'a> {
    count: u8,
    #[deku(count = "count")]
    data: Cow<'a, [u8]>,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[cfg(feature = "alloc")]
struct CowByteSized<'a> {
    #[deku(bytes = "2")]
    data: Cow<'a, [u8]>,
}

#[test]
fn from_bytes_borrows_slice_field() {
    let input = [3, 0xaa, 0xbb, 0xcc, 0xdd];
    let ((rest, bit_offset), value) = Borrowed::from_bytes((&input, 0)).unwrap();

    assert_eq!((rest, bit_offset), (&input[4..], 0));
    assert_eq!(value.data, &input[1..4]);
    assert_eq!(value.data.as_ptr(), input[1..4].as_ptr());

    let mut output = [0; 4];
    assert_eq!(value.to_slice(&mut output).unwrap(), output.len());
    assert_eq!(output, input[..4]);
}

#[test]
fn from_reader_with_ctx_can_borrow_from_source_reader() {
    let input = [0xaa, 0xbb, 0xcc];
    let mut reader = deku::reader::Reader::from_bytes(&input);
    let data = <&[u8]>::from_reader_with_ctx(&mut reader, deku::ctx::ReadExact(2)).unwrap();

    assert_eq!(data, &input[..2]);
    assert_eq!(data.as_ptr(), input.as_ptr());
    assert_eq!(reader.bits_read, 16);
}

#[test]
fn custom_reader_can_borrow_slice_from_source_reader() {
    fn read_bytes<'a, R: deku::no_std_io::Read + deku::no_std_io::Seek>(
        reader: &mut deku::reader::Reader<'a, R>,
    ) -> Result<&'a [u8], DekuError> {
        <&'a [u8]>::from_reader_with_ctx(reader, deku::ctx::ReadExact(2))
    }

    #[derive(Debug, PartialEq, DekuRead)]
    struct CustomBorrowed<'a> {
        #[deku(reader = "read_bytes(deku::reader)")]
        data: &'a [u8],
    }

    let input = [0xaa, 0xbb, 0xcc];
    let ((rest, bit_offset), value) = CustomBorrowed::from_bytes((&input, 0)).unwrap();

    assert_eq!((rest, bit_offset), (&input[2..], 0));
    assert_eq!(value.data, &input[..2]);
    assert_eq!(value.data.as_ptr(), input.as_ptr());
}

#[test]
fn from_bytes_borrows_nested_slice_field() {
    let input = [0x01, 0xaa, 0xbb, 0xcc];
    let ((rest, bit_offset), value) = NestedBorrowed::from_bytes((&input, 0)).unwrap();

    assert_eq!((rest, bit_offset), (&input[3..], 0));
    assert_eq!(value.data, &input[1..3]);
    assert_eq!(value.data.as_ptr(), input[1..3].as_ptr());
}

#[test]
#[cfg(feature = "alloc")]
fn from_bytes_borrows_through_vec() {
    let input = [0x01, 0xaa, 0xbb, 0x02, 0xcc, 0xdd, 0xee];
    let ((rest, bit_offset), value) = RepeatedBorrowed::from_bytes((&input, 0)).unwrap();

    assert_eq!((rest, bit_offset), (&input[6..], 0));
    assert_eq!(value.records[0].data, &input[1..3]);
    assert_eq!(value.records[1].data, &input[4..6]);
    assert_eq!(value.records[0].data.as_ptr(), input[1..3].as_ptr());
    assert_eq!(value.records[1].data.as_ptr(), input[4..6].as_ptr());
}

#[test]
#[cfg(feature = "std")]
fn from_bytes_borrows_through_hashmap() {
    let input = [1, 0xaa, 0xbb, 2, 0xcc, 0xdd, 0xee];
    let ((rest, bit_offset), value) = MapBorrowed::from_bytes((&input, 0)).unwrap();

    assert_eq!((rest, bit_offset), (&input[6..], 0));
    assert_eq!(value.values[&1].data, &input[1..3]);
    assert_eq!(value.values[&2].data, &input[4..6]);
    assert_eq!(value.values[&1].data.as_ptr(), input[1..3].as_ptr());
    assert_eq!(value.values[&2].data.as_ptr(), input[4..6].as_ptr());
}

#[test]
#[cfg(feature = "std")]
fn from_bytes_borrows_through_hashset() {
    let input = [1, 0xaa, 0xbb, 2, 0xcc, 0xdd, 0xee];
    let ((rest, bit_offset), value) = SetBorrowed::from_bytes((&input, 0)).unwrap();

    assert_eq!((rest, bit_offset), (&input[6..], 0));
    let first = value.values.iter().find(|value| value.key == 1).unwrap();
    let second = value.values.iter().find(|value| value.key == 2).unwrap();
    assert_eq!(first.data, &input[1..3]);
    assert_eq!(second.data, &input[4..6]);
    assert_eq!(first.data.as_ptr(), input[1..3].as_ptr());
    assert_eq!(second.data.as_ptr(), input[4..6].as_ptr());
}

#[test]
fn from_bytes_borrows_with_byte_size_context() {
    #[derive(Debug, PartialEq, DekuRead)]
    struct FixedBorrowed<'a> {
        #[deku(bytes = "2")]
        data: &'a [u8],
    }

    let input = [0xaa, 0xbb, 0xcc];
    let ((rest, bit_offset), value) = FixedBorrowed::from_bytes((&input, 0)).unwrap();

    assert_eq!((rest, bit_offset), (&input[2..], 0));
    assert_eq!(value.data, &input[..2]);
    assert_eq!(value.data.as_ptr(), input[..2].as_ptr());
}

#[test]
fn from_bytes_borrows_fixed_bytes_with_context() {
    #[derive(Debug, PartialEq, DekuRead)]
    #[deku(endian = "big", ctx = "a: u8, b: u8", ctx_default = "1, 2")]
    struct FixedContextBorrowed<'a> {
        #[deku(bytes = "2", ctx = "a, b")]
        data: &'a [u8],
    }

    let input = [0xaa, 0xbb, 0xcc];
    let ((rest, bit_offset), value) = FixedContextBorrowed::from_bytes((&input, 0)).unwrap();

    assert_eq!((rest, bit_offset), (&input[2..], 0));
    assert_eq!(value.data, &input[..2]);
    assert_eq!(value.data.as_ptr(), input.as_ptr());
}

#[cfg(feature = "alloc")]
#[test]
fn from_bytes_borrows_cow_fixed_bytes_with_context() {
    #[derive(Debug, PartialEq, DekuRead)]
    #[deku(endian = "big", ctx = "a: u8, b: u8", ctx_default = "1, 2")]
    struct CowContextBorrowed<'a> {
        #[deku(bytes = "2", ctx = "a, b")]
        data: Cow<'a, [u8]>,
    }

    let input = [0xaa, 0xbb, 0xcc];
    let ((rest, bit_offset), value) = CowContextBorrowed::from_bytes((&input, 0)).unwrap();

    assert_eq!((rest, bit_offset), (&input[2..], 0));
    let Cow::Borrowed(data) = value.data else {
        panic!("expected Cow::Borrowed");
    };
    assert_eq!(data, &input[..2]);
    assert_eq!(data.as_ptr(), input.as_ptr());
}

#[cfg(feature = "bits")]
#[test]
fn from_bytes_borrows_fixed_bits_with_context() {
    #[derive(Debug, PartialEq, DekuRead)]
    #[deku(bit_order = "lsb", ctx = "context: u8", ctx_default = "7")]
    struct FixedContextBorrowed<'a> {
        #[deku(bits = "16", ctx = "context")]
        data: &'a [u8],
    }

    let input = [0xaa, 0xbb, 0xcc];
    let ((rest, bit_offset), value) = FixedContextBorrowed::from_bytes((&input, 0)).unwrap();

    assert_eq!((rest, bit_offset), (&input[2..], 0));
    assert_eq!(value.data, &input[..2]);
    assert_eq!(value.data.as_ptr(), input.as_ptr());
}

#[test]
fn from_bytes_borrows_until_and_read_all() {
    #[derive(Debug, PartialEq, DekuRead)]
    struct UntilBorrowed<'a> {
        #[deku(until = "|v: &u8| *v == 0")]
        data: &'a [u8],
    }

    #[derive(Debug, PartialEq, DekuRead)]
    struct AllBorrowed<'a> {
        prefix: u8,
        #[deku(read_all)]
        data: &'a [u8],
    }

    let until_input = [0xaa, 0xbb, 0, 0xcc];
    let ((rest, bit_offset), value) = UntilBorrowed::from_bytes((&until_input, 0)).unwrap();
    assert_eq!((rest, bit_offset), (&until_input[3..], 0));
    assert_eq!(value.data, &until_input[..3]);
    assert_eq!(value.data.as_ptr(), until_input.as_ptr());

    let all_input = [0x01, 0xaa, 0xbb, 0xcc];
    let ((rest, bit_offset), value) = AllBorrowed::from_bytes((&all_input, 0)).unwrap();
    assert_eq!((rest, bit_offset), (&[][..], 0));
    assert_eq!(value.data, &all_input[1..]);
    assert_eq!(value.data.as_ptr(), all_input[1..].as_ptr());
}

#[cfg(feature = "bits")]
#[test]
fn from_bytes_rejects_unaligned_borrow() {
    #[derive(Debug, PartialEq, DekuRead)]
    struct UnalignedBorrowed<'a> {
        #[deku(bits = "1")]
        bit: u8,
        #[deku(bytes_read = "1")]
        data: &'a [u8],
    }

    let error = UnalignedBorrowed::from_bytes((&[0xaa, 0xbb], 0)).unwrap_err();
    assert!(matches!(error, deku::DekuError::InvalidParam(_)));
}

#[test]
fn from_bytes_borrows_enum_slice_field() {
    let input = [1, 0xaa, 0xbb, 0xcc];
    let ((rest, bit_offset), value) = BorrowedEnum::from_bytes((&input, 0)).unwrap();

    assert_eq!((rest, bit_offset), (&input[3..], 0));
    let BorrowedEnum::Data(data) = value;
    assert_eq!(data, &input[1..3]);
    assert_eq!(data.as_ptr(), input[1..3].as_ptr());
}

#[test]
fn try_from_borrows_slice_field() {
    let input = [3, 0xaa, 0xbb, 0xcc];
    let value = Borrowed::try_from(&input[..]).unwrap();

    assert_eq!(value.data, &input[1..]);
    assert_eq!(value.data.as_ptr(), input[1..].as_ptr());
}

#[test]
#[cfg(feature = "alloc")]
fn from_bytes_borrows_cow_slice() {
    let input = [3, 0xaa, 0xbb, 0xcc, 0xdd];
    let ((rest, bit_offset), value) = CowBorrowed::from_bytes((&input, 0)).unwrap();

    assert_eq!((rest, bit_offset), (&input[4..], 0));
    let Cow::Borrowed(data) = value.data else {
        panic!("expected Cow::Borrowed");
    };
    assert_eq!(data, &input[1..4]);
    assert_eq!(data.as_ptr(), input[1..4].as_ptr());
}

#[test]
#[cfg(feature = "alloc")]
fn from_reader_owns_cow_slice() {
    let input = [3, 0xaa, 0xbb, 0xcc, 0xdd];
    let mut cursor = deku::no_std_io::Cursor::new(&input[..]);
    let (_, value) = CowBorrowed::from_reader((&mut cursor, 0)).unwrap();

    let Cow::Owned(data) = value.data else {
        panic!("expected Cow::Owned");
    };
    assert_eq!(data, input[1..4]);
}

#[test]
#[cfg(feature = "alloc")]
fn from_bytes_borrows_cow_with_byte_size() {
    let input = [0xaa, 0xbb, 0xcc];
    let ((rest, bit_offset), value) = CowByteSized::from_bytes((&input, 0)).unwrap();

    assert_eq!((rest, bit_offset), (&input[2..], 0));
    let Cow::Borrowed(data) = value.data else {
        panic!("expected Cow::Borrowed");
    };
    assert_eq!(data, &input[..2]);
    assert_eq!(data.as_ptr(), input.as_ptr());
}

#[test]
fn from_bytes_dispatches_common_field_attributes() {
    fn read_one<R: deku::no_std_io::Read + deku::no_std_io::Seek>(
        reader: &mut deku::reader::Reader<'_, R>,
    ) -> Result<u8, DekuError> {
        u8::from_reader_with_ctx(reader, ())
    }

    #[deku_derive(DekuRead)]
    #[derive(Debug, PartialEq)]
    #[deku(magic = b"Z")]
    struct AttributeBorrowed<'a> {
        #[deku(temp, temp_value = "0")]
        temp: u8,
        #[deku(
            map = "|value: u8| -> Result<_, DekuError> { Ok(value + 1) }",
            assert_eq = "2"
        )]
        mapped: u8,
        #[deku(cond = "*mapped == 2", default = "&[]", bytes_read = "1")]
        conditional: &'a [u8],
        #[deku(skip, default = "&[]")]
        skipped: &'a [u8],
        #[deku(reader = "read_one(deku::reader)")]
        custom: u8,
        #[deku(bytes_read = "1")]
        data: &'a [u8],
    }

    let input = [b'Z', 0, 1, 0xaa, 0xcc, 0xbb];
    let ((rest, bit_offset), value) = AttributeBorrowed::from_bytes((&input, 0)).unwrap();

    assert_eq!((rest, bit_offset), (&[][..], 0));
    assert_eq!(value.mapped, 2);
    assert_eq!(value.conditional, &input[3..4]);
    assert_eq!(value.custom, 0xcc);
    assert_eq!(value.data, &input[5..]);
    assert_eq!(value.data.as_ptr(), input[5..].as_ptr());
}

#[test]
fn from_bytes_dispatches_context_default() {
    #[derive(Debug, PartialEq, DekuRead)]
    #[deku(ctx = "context: u8", ctx_default = "7")]
    struct ContextBorrowed<'a> {
        #[deku(ctx = "context", bytes_read = "2")]
        data: &'a [u8],
    }

    let input = [0xaa, 0xbb, 0xcc];
    let ((rest, bit_offset), value) = ContextBorrowed::from_bytes((&input, 0)).unwrap();

    assert_eq!((rest, bit_offset), (&input[2..], 0));
    assert_eq!(value.data, &input[..2]);
    assert_eq!(value.data.as_ptr(), input.as_ptr());
}

#[test]
fn from_bytes_dispatches_enum_default_and_magic() {
    #[derive(Debug, PartialEq, DekuRead)]
    #[deku(magic = b"Z", id_type = "u8")]
    enum EnumAttributeBorrowed<'a> {
        #[deku(id = "1")]
        Known(#[deku(bytes_read = "2")] &'a [u8]),
        #[deku(id_pat = "_")]
        Unknown {
            id: u8,
            #[deku(bytes_read = "2")]
            data: &'a [u8],
        },
    }

    let known_input = [b'Z', 1, 0xaa, 0xbb];
    let ((rest, bit_offset), known) = EnumAttributeBorrowed::from_bytes((&known_input, 0)).unwrap();
    let EnumAttributeBorrowed::Known(data) = known else {
        panic!("expected known variant");
    };
    assert_eq!((rest, bit_offset), (&[][..], 0));
    assert_eq!(data, &known_input[2..]);
    assert_eq!(data.as_ptr(), known_input[2..].as_ptr());

    let default_input = [b'Z', 9, 0xcc, 0xdd];
    let ((rest, bit_offset), default) =
        EnumAttributeBorrowed::from_bytes((&default_input, 0)).unwrap();
    let EnumAttributeBorrowed::Unknown { id, data } = default else {
        panic!("expected default variant");
    };
    assert_eq!(id, 9);
    assert_eq!((rest, bit_offset), (&[][..], 0));
    assert_eq!(data, &default_input[2..]);
    assert_eq!(data.as_ptr(), default_input[2..].as_ptr());
}

#[test]
fn from_bytes_dispatches_seek_attributes() {
    #[derive(Debug, PartialEq, DekuRead)]
    struct SeekStartBorrowed<'a> {
        #[deku(seek_from_start = "2", bytes_read = "1")]
        data: &'a [u8],
    }

    #[derive(Debug, PartialEq, DekuRead)]
    struct SeekCurrentBorrowed<'a> {
        #[deku(seek_from_current = "1", bytes_read = "1")]
        data: &'a [u8],
    }

    #[derive(Debug, PartialEq, DekuRead)]
    struct SeekEndBorrowed<'a> {
        #[deku(seek_from_end = "-2", bytes_read = "1")]
        data: &'a [u8],
    }

    #[derive(Debug, PartialEq, DekuRead)]
    struct SeekRewindBorrowed<'a> {
        prefix: u8,
        #[deku(seek_rewind, bytes_read = "1")]
        data: &'a [u8],
    }

    let start_input = [0, 1, 0xaa, 0xbb];
    let ((rest, bit_offset), value) = SeekStartBorrowed::from_bytes((&start_input, 0)).unwrap();
    assert_eq!((rest, bit_offset), (&start_input[3..], 0));
    assert_eq!(value.data.as_ptr(), start_input[2..3].as_ptr());

    let current_input = [0, 0xaa, 0xbb];
    let ((rest, bit_offset), value) = SeekCurrentBorrowed::from_bytes((&current_input, 0)).unwrap();
    assert_eq!((rest, bit_offset), (&current_input[2..], 0));
    assert_eq!(value.data.as_ptr(), current_input[1..2].as_ptr());

    let end_input = [0, 0xaa, 0xbb];
    let ((rest, bit_offset), value) = SeekEndBorrowed::from_bytes((&end_input, 0)).unwrap();
    assert_eq!((rest, bit_offset), (&end_input[2..], 0));
    assert_eq!(value.data.as_ptr(), end_input[1..2].as_ptr());

    let rewind_input = [0xaa, 0xbb];
    let ((rest, bit_offset), value) = SeekRewindBorrowed::from_bytes((&rewind_input, 0)).unwrap();
    assert_eq!((rest, bit_offset), (&rewind_input[1..], 0));
    assert_eq!(value.prefix, 0xaa);
    assert_eq!(value.data.as_ptr(), rewind_input[..1].as_ptr());
}

#[cfg(feature = "bits")]
#[test]
fn from_bytes_dispatches_aligned_bits_and_padding() {
    #[derive(Debug, PartialEq, DekuRead)]
    struct AlignedBitsBorrowed<'a> {
        #[deku(bits = "8")]
        prefix: u8,
        #[deku(bits = "16")]
        data: &'a [u8],
    }

    #[derive(Debug, PartialEq, DekuRead)]
    struct PaddedBorrowed<'a> {
        #[deku(pad_bits_before = "8", bytes_read = "1", pad_bits_after = "8")]
        data: &'a [u8],
    }

    let aligned_input = [1, 0xaa, 0xbb, 0xcc];
    let ((rest, bit_offset), value) = AlignedBitsBorrowed::from_bytes((&aligned_input, 0)).unwrap();
    assert_eq!((rest, bit_offset), (&aligned_input[3..], 0));
    assert_eq!(value.data.as_ptr(), aligned_input[1..3].as_ptr());

    let padded_input = [0, 0xaa, 0, 0xbb];
    let ((rest, bit_offset), value) = PaddedBorrowed::from_bytes((&padded_input, 0)).unwrap();
    assert_eq!((rest, bit_offset), (&padded_input[3..], 0));
    assert_eq!(value.data.as_ptr(), padded_input[1..2].as_ptr());
}

#[cfg(feature = "bits")]
#[test]
fn from_bytes_borrows_after_an_aligned_input_offset() {
    let input = [0xff, 3, 0xaa, 0xbb, 0xcc];
    let ((rest, bit_offset), value) = Borrowed::from_bytes((&input, 8)).unwrap();

    assert_eq!((rest, bit_offset), (&[][..], 0));
    assert_eq!(value.data, &input[2..]);
    assert_eq!(value.data.as_ptr(), input[2..].as_ptr());
}

#[test]
fn stream_read_rejects_borrowed_slice_field() {
    let input = [1, 0xaa];
    let mut cursor = deku::no_std_io::Cursor::new(&input[..]);

    assert!(Borrowed::from_reader((&mut cursor, 0)).is_err());
}
