use deku::prelude::*;

#[test]
fn test_primitive_sizes() {
    assert_eq!(u8::SIZE_BYTES, Some(1));
    assert_eq!(u16::SIZE_BYTES, Some(2));
    assert_eq!(u32::SIZE_BYTES, Some(4));
    assert_eq!(u64::SIZE_BYTES, Some(8));
    assert_eq!(u128::SIZE_BYTES, Some(16));

    assert_eq!(i8::SIZE_BYTES, Some(1));
    assert_eq!(i16::SIZE_BYTES, Some(2));
    assert_eq!(i32::SIZE_BYTES, Some(4));
    assert_eq!(i64::SIZE_BYTES, Some(8));
    assert_eq!(i128::SIZE_BYTES, Some(16));

    assert_eq!(f32::SIZE_BYTES, Some(4));
    assert_eq!(f64::SIZE_BYTES, Some(8));

    assert_eq!(bool::SIZE_BYTES, Some(1));
}

#[test]
fn test_array_sizes() {
    assert_eq!(<[u8; 4]>::SIZE_BYTES, Some(4));
    assert_eq!(<[u16; 4]>::SIZE_BYTES, Some(8));
    assert_eq!(<[u32; 10]>::SIZE_BYTES, Some(40));
    assert_eq!(<[u8; 0]>::SIZE_BYTES, Some(0));

    assert_eq!(<[[u8; 2]; 3]>::SIZE_BYTES, Some(6));
}

#[test]
fn test_tuple_sizes() {
    assert_eq!(<(u8,)>::SIZE_BYTES, Some(1));
    assert_eq!(<(u8, u16)>::SIZE_BYTES, Some(3));
    assert_eq!(<(u8, u16, u32)>::SIZE_BYTES, Some(7));
    assert_eq!(<(u8, u8, u8, u8)>::SIZE_BYTES, Some(4));
}

#[test]
fn test_simple_struct() {
    #[derive(DekuRead, DekuWrite, DekuSize)]
    struct SimpleStruct {
        a: u8,
        b: u16,
        c: u32,
    }

    assert_eq!(SimpleStruct::SIZE_BYTES, Some(7));
}

#[test]
#[cfg(feature = "bits")]
fn test_bit_sized_fields() {
    #[derive(DekuRead, DekuWrite, DekuSize)]
    #[deku(endian = "big")]
    struct BitStruct {
        #[deku(bits = 4)]
        a: u8,
        #[deku(bits = 4)]
        b: u8,
        c: u8,
    }

    assert_eq!(BitStruct::SIZE_BYTES, Some(2));
}

#[test]
#[cfg(feature = "bits")]
fn test_non_byte_aligned() {
    #[derive(DekuRead, DekuWrite, DekuSize)]
    #[deku(endian = "big")]
    struct NonAligned {
        #[deku(bits = 3)]
        a: u8,
        #[deku(bits = 5)]
        b: u8,
        #[deku(bits = 6)]
        c: u8,
    }

    assert_eq!(NonAligned::SIZE_BITS, 14);
    assert_eq!(NonAligned::SIZE_BYTES, None);
}

#[test]
fn test_nested_struct() {
    #[derive(DekuRead, DekuWrite, DekuSize)]
    struct Inner {
        x: u16,
        y: u16,
    }

    #[derive(DekuRead, DekuWrite, DekuSize)]
    struct Outer {
        header: u8,
        inner: Inner,
        footer: u8,
    }

    assert_eq!(Inner::SIZE_BYTES, Some(4));
    assert_eq!(Outer::SIZE_BYTES, Some(6));
}

#[test]
fn test_struct_with_arrays() {
    #[derive(DekuRead, DekuWrite, DekuSize)]
    struct WithArray {
        count: u8,
        data: [u8; 4],
    }

    assert_eq!(WithArray::SIZE_BYTES, Some(5));
}

#[test]
fn test_unit_struct() {
    #[derive(DekuRead, DekuWrite, DekuSize)]
    struct UnitStruct;

    assert_eq!(UnitStruct::SIZE_BYTES, Some(0));
}

#[test]
fn test_tuple_struct() {
    #[derive(DekuRead, DekuWrite, DekuSize)]
    struct TupleStruct(u8, u16, u32);

    assert_eq!(TupleStruct::SIZE_BYTES, Some(7));
}

#[test]
fn test_simple_enum() {
    #[derive(DekuRead, DekuWrite, DekuSize)]
    #[deku(id_type = "u8")]
    enum SimpleEnum {
        #[deku(id = 0)]
        A,
        #[deku(id = 1)]
        B,
    }

    assert_eq!(SimpleEnum::SIZE_BYTES, Some(1));
}

#[test]
fn test_enum_with_fields() {
    #[derive(DekuRead, DekuWrite, DekuSize)]
    #[deku(id_type = "u8")]
    enum DataEnum {
        #[deku(id = 0)]
        Small(u8),
        #[deku(id = 1)]
        Large(u32),
    }

    assert_eq!(DataEnum::SIZE_BYTES, Some(5));
}

#[test]
fn test_enum_with_named_fields() {
    #[derive(DekuRead, DekuWrite, DekuSize)]
    #[deku(id_type = "u8")]
    enum NamedEnum {
        #[deku(id = 0)]
        A { x: u16 },
        #[deku(id = 1)]
        B { y: u32, z: u8 },
    }

    assert_eq!(NamedEnum::SIZE_BYTES, Some(6));
}

#[test]
fn test_with_to_slice() {
    #[derive(DekuRead, DekuWrite, DekuSize)]
    #[deku(endian = "big")]
    struct Message {
        msg_type: u8,
        payload: [u8; 16],
        checksum: u16,
    }

    const MSG_SIZE: usize = Message::SIZE_BYTES.unwrap();
    let mut buffer = [0u8; MSG_SIZE];

    assert_eq!(MSG_SIZE, 19);

    let msg = Message {
        msg_type: 0x01,
        payload: [0xFF; 16],
        checksum: 0xABCD,
    };

    let written = msg.to_slice(&mut buffer).unwrap();
    assert_eq!(written, MSG_SIZE);

    let (_, parsed) = Message::from_bytes((&buffer, 0)).unwrap();
    assert_eq!(parsed.msg_type, 0x01);
    assert_eq!(parsed.checksum, 0xABCD);
}

#[test]
#[cfg(feature = "bits")]
fn test_enum_with_bit_fields() {
    #[derive(DekuRead, DekuWrite, DekuSize)]
    #[deku(id_type = "u8", bits = 2)]
    enum BitEnum {
        #[deku(id = 0)]
        A {
            #[deku(bits = 6)]
            val: u8,
        },
        #[deku(id = 1)]
        B {
            #[deku(bits = 3)]
            x: u8,
            #[deku(bits = 3)]
            y: u8,
        },
    }

    assert_eq!(BitEnum::SIZE_BYTES, Some(1));
}

#[test]
fn test_enum_uses_max_variant_size() {
    #[derive(Debug, DekuRead, DekuWrite, DekuSize)]
    #[deku(id_type = "u8")]
    enum MixedSizeEnum {
        #[deku(id = "1")]
        Small { a: u8 },

        #[deku(id = "2")]
        Medium { b: u16 },

        #[deku(id = "3")]
        Large { c: u64 },
    }

    assert_eq!(MixedSizeEnum::SIZE_BYTES, Some(9));
}

#[cfg(feature = "alloc")]
#[test]
fn test_enum_actual_vs_max_size() {
    #[derive(Debug, DekuRead, DekuWrite, DekuSize)]
    #[deku(id_type = "u8")]
    enum VarySizes {
        #[deku(id = "1")]
        Tiny { x: u8 },

        #[deku(id = "2")]
        Huge {
            #[deku(endian = "big")]
            a: u64,
            #[deku(endian = "big")]
            b: u64,
            #[deku(endian = "big")]
            c: u32,
        },
    }

    assert_eq!(VarySizes::SIZE_BYTES, Some(21));

    let tiny = VarySizes::Tiny { x: 42 };
    assert_eq!(tiny.to_bytes().unwrap().len(), 2);

    let huge = VarySizes::Huge { a: 1, b: 2, c: 3 };
    assert_eq!(
        huge.to_bytes().unwrap().len(),
        VarySizes::SIZE_BYTES.unwrap()
    );
}

#[cfg(feature = "alloc")]
#[test]
fn test_unit_variants_are_zero_size() {
    #[derive(Debug, DekuRead, DekuWrite, DekuSize)]
    #[deku(id_type = "u8")]
    enum Status {
        #[deku(id = "0")]
        Idle,

        #[deku(id = "1")]
        Active { pid: u32 },

        #[deku(id = "2")]
        Failed,
    }

    assert_eq!(Status::SIZE_BYTES, Some(5));

    let idle = Status::Idle;
    assert_eq!(idle.to_bytes().unwrap().len(), 1);

    let active = Status::Active { pid: 999 };
    assert_eq!(
        active.to_bytes().unwrap().len(),
        Status::SIZE_BYTES.unwrap()
    );
}

#[test]
fn test_enum_field_is_another_enum() {
    #[derive(Debug, DekuRead, DekuWrite, DekuSize)]
    #[deku(id_type = "u8")]
    enum InnerEnum {
        #[deku(id = "1")]
        Small { x: u8 },

        #[deku(id = "2")]
        Large { y: u64 },
    }

    #[derive(Debug, DekuRead, DekuWrite, DekuSize)]
    #[deku(id_type = "u8")]
    enum OuterEnum {
        #[deku(id = "10")]
        Simple { value: u32 },

        #[deku(id = "20")]
        WithInner { inner: InnerEnum },
    }

    assert_eq!(InnerEnum::SIZE_BYTES, Some(9));
    assert_eq!(OuterEnum::SIZE_BYTES, Some(10));
}

#[test]
fn test_deeply_nested_enums() {
    #[derive(Debug, DekuRead, DekuWrite, DekuSize)]
    #[deku(id_type = "u8")]
    enum Level3 {
        #[deku(id = "1")]
        A { a: u16 },

        #[deku(id = "2")]
        B { b: u32 },
    }

    #[derive(Debug, DekuRead, DekuWrite, DekuSize)]
    #[deku(id_type = "u8")]
    enum Level2 {
        #[deku(id = "1")]
        X { x: u8 },

        #[deku(id = "2")]
        Y { inner: Level3 },
    }

    #[derive(Debug, DekuRead, DekuWrite, DekuSize)]
    #[deku(id_type = "u8")]
    enum Level1 {
        #[deku(id = "1")]
        Simple { s: u8 },

        #[deku(id = "2")]
        Nested { n: Level2 },
    }

    assert_eq!(Level3::SIZE_BYTES, Some(5));
    assert_eq!(Level2::SIZE_BYTES, Some(6));
    assert_eq!(Level1::SIZE_BYTES, Some(7));
}

#[test]
fn test_arrays_of_various_sizes() {
    #[derive(Debug, DekuRead, DekuWrite, DekuSize)]
    struct WithArrays {
        small: [u8; 1],
        medium: [u16; 5],
        large: [u32; 10],
    }

    assert_eq!(WithArrays::SIZE_BYTES, Some(51));
}

#[test]
fn test_nested_arrays() {
    #[derive(Debug, DekuRead, DekuWrite, DekuSize)]
    struct NestedArrays {
        matrix: [[u8; 4]; 3],
    }

    assert_eq!(NestedArrays::SIZE_BYTES, Some(12));
}

#[test]
fn test_complex_tuples() {
    assert_eq!(<(u8, u16, u32, u64)>::SIZE_BYTES, Some(15));

    assert_eq!(<((u8, u16), (u32, u64))>::SIZE_BYTES, Some(15));

    assert_eq!(<(u8, [u8; 4], u16)>::SIZE_BYTES, Some(7));
}

#[test]
fn test_zero_sized_types() {
    #[derive(Debug, DekuRead, DekuWrite, DekuSize)]
    struct ZeroSized;

    assert_eq!(ZeroSized::SIZE_BYTES, Some(0));

    #[derive(Debug, DekuRead, DekuWrite, DekuSize)]
    struct WithZeroSizedField {
        a: u8,
        b: ZeroSized,
        c: u16,
    }

    assert_eq!(WithZeroSizedField::SIZE_BYTES, Some(3));
}

#[test]
#[cfg(feature = "bits")]
fn test_enum_with_bits_discriminant_and_data() {
    #[derive(Debug, DekuRead, DekuWrite, DekuSize)]
    #[deku(id_type = "u8", bits = 4)]
    enum SmallDiscriminant {
        #[deku(id = "0")]
        A,
        #[deku(id = "1")]
        B { x: u8 },
        #[deku(id = "2")]
        C { y: u16, z: u8 },
    }

    assert_eq!(SmallDiscriminant::SIZE_BITS, 28);
    assert_eq!(SmallDiscriminant::SIZE_BYTES, None);
}

#[test]
fn test_generic_struct_with_deku_size() {
    #[derive(Debug, DekuRead, DekuWrite, DekuSize)]
    struct GenericStruct<T>
    where
        T: DekuSize + for<'a> DekuReader<'a> + DekuWriter,
    {
        value: T,
    }

    assert_eq!(GenericStruct::<u8>::SIZE_BYTES, Some(1));
    assert_eq!(GenericStruct::<u16>::SIZE_BYTES, Some(2));
    assert_eq!(GenericStruct::<u32>::SIZE_BYTES, Some(4));
}

#[test]
fn test_struct_with_top_level_magic() {
    #[derive(DekuRead, DekuWrite, DekuSize)]
    #[deku(magic = b"DEKU")]
    struct MagicStruct {
        a: u8,
        b: u16,
    }

    assert_eq!(MagicStruct::SIZE_BYTES, Some(7));
    assert_eq!(MagicStruct::SIZE_BITS, 56);
}

#[test]
fn test_enum_with_top_level_magic() {
    #[derive(DekuRead, DekuWrite, DekuSize)]
    #[deku(magic = b"TEST", id_type = "u8")]
    enum MagicEnum {
        #[deku(id = 0)]
        Small(u8),
        #[deku(id = 1)]
        Large(u32),
    }

    assert_eq!(MagicEnum::SIZE_BYTES, Some(9));
    assert_eq!(MagicEnum::SIZE_BITS, 72);
}

#[test]
fn test_struct_with_field_level_magic() {
    #[derive(DekuRead, DekuWrite, DekuSize)]
    struct FieldMagicStruct {
        #[deku(magic = b"HDR")]
        header: u8,
        data: u16,
    }

    assert_eq!(FieldMagicStruct::SIZE_BYTES, Some(6));
    assert_eq!(FieldMagicStruct::SIZE_BITS, 48);
}

#[test]
fn test_struct_with_both_magic_types() {
    #[derive(DekuRead, DekuWrite, DekuSize)]
    #[deku(magic = b"TOP")]
    struct BothMagicStruct {
        #[deku(magic = b"FLD")]
        field: u8,
        data: u32,
    }

    assert_eq!(BothMagicStruct::SIZE_BYTES, Some(11));
    assert_eq!(BothMagicStruct::SIZE_BITS, 88);
}

#[test]
#[cfg(feature = "bits")]
fn test_enum_with_magic_and_bit_discriminant() {
    #[derive(DekuRead, DekuWrite, DekuSize)]
    #[deku(magic = b"AB", id_type = "u8", bits = 4)]
    enum BitMagicEnum {
        #[deku(id = 0)]
        A {
            #[deku(bits = 4)]
            val: u8,
        },
        #[deku(id = 1)]
        B { x: u8 },
    }

    assert_eq!(BitMagicEnum::SIZE_BITS, 28);
    assert_eq!(BitMagicEnum::SIZE_BYTES, None);
}

#[test]
fn test_multiple_field_magic() {
    #[derive(DekuRead, DekuWrite, DekuSize)]
    struct MultiFieldMagic {
        #[deku(magic = b"A")]
        field_a: u8,
        #[deku(magic = b"BB")]
        field_b: u16,
        #[deku(magic = b"CCC")]
        field_c: u32,
    }

    assert_eq!(MultiFieldMagic::SIZE_BYTES, Some(13));
}

#[test]
fn test_empty_struct_with_magic() {
    #[derive(DekuRead, DekuWrite, DekuSize)]
    #[deku(magic = b"EMPTY")]
    struct EmptyMagicStruct;

    assert_eq!(EmptyMagicStruct::SIZE_BYTES, Some(5));
}

#[test]
fn test_nested_struct_with_magic() {
    #[derive(DekuRead, DekuWrite, DekuSize)]
    #[deku(magic = b"IN")]
    struct InnerWithMagic {
        value: u16,
    }

    #[derive(DekuRead, DekuWrite, DekuSize)]
    #[deku(magic = b"OUT")]
    struct OuterWithMagic {
        inner: InnerWithMagic,
        footer: u8,
    }

    assert_eq!(InnerWithMagic::SIZE_BYTES, Some(4));
    assert_eq!(OuterWithMagic::SIZE_BYTES, Some(8));
}
