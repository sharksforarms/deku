use deku::prelude::*;
use std::io::Cursor;

// Invalid alignment assumptions when converting
// BitSlice to type
//
// https://github.com/sharksforarms/deku/issues/224
#[cfg(feature = "bits")]
#[test]
fn issue_224() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    pub struct Packet {
        pub data: Data,
    }

    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    pub struct Data {
        pub one: One,
        pub two: Two,
    }

    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(id_type = "u8", bits = 2)]
    pub enum One {
        Start = 0,
        Go = 1,
        Stop = 2,
    }

    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(id_type = "u8", bits = 4)]
    pub enum Two {
        #[deku(id = "0b0000")]
        Put(Op1),
        #[deku(id = "0b0001")]
        Store(Op1),
        #[deku(id = "0b0010")]
        Load(Op2),
        #[deku(id = "0b0011")]
        Allocate(Op2),
    }

    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    pub struct Op1 {
        #[deku(bits = 2)]
        pub i: u8,
        #[deku(bits = 4)]
        pub o: u8,
    }

    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    pub struct Op2 {
        #[deku(bits = 8)]
        pub w: u8,
        #[deku(bits = 6)]
        pub j: u8,
    }

    let packet = Packet {
        data: Data {
            one: One::Start,
            two: Two::Load(Op2 { w: 1, j: 2 }),
        },
    };
    let bytes = packet.to_bytes().unwrap();
    let mut c = std::io::Cursor::new(bytes);
    let _packet = Packet::from_reader((&mut c, 0)).unwrap();
}

#[cfg(feature = "bits")]
// Extra zeroes added when reading fewer bytes than needed to fill a number
//
// https://github.com/sharksforarms/deku/issues/282
mod issue_282 {
    use super::*;

    #[test]
    fn be() {
        #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
        #[deku(endian = "big")]
        struct BitsBytes {
            #[deku(bits = 24)]
            bits: u32,

            #[deku(bytes = 3)]
            bytes: u32,
        }

        let expected: u32 = 11280317;
        let [zero, a, b, c] = expected.to_be_bytes();

        // the u32 is stored as three bytes in big-endian order
        assert_eq!(zero, 0);

        let data = [a, b, c, a, b, c];
        let mut cursor = Cursor::new(data);
        let (_, BitsBytes { bits, bytes }) = BitsBytes::from_reader((&mut cursor, 0)).unwrap();

        assert_eq!(bits, expected);
        assert_eq!(bytes, expected);
    }

    #[test]
    fn le() {
        #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
        #[deku(endian = "little")]
        struct BitsBytes {
            #[deku(bits = 24)]
            bits: u32,

            #[deku(bytes = 3)]
            bytes: u32,
        }

        let expected: u32 = 11280317;
        let [a, b, c, zero] = expected.to_le_bytes();

        // the u32 is stored as three bytes in little-endian order
        assert_eq!(zero, 0);

        let data = [a, b, c, a, b, c];
        let mut cursor = Cursor::new(data);
        let (_, BitsBytes { bits, bytes }) = BitsBytes::from_reader((&mut cursor, 0)).unwrap();

        assert_eq!(bits, expected);
        assert_eq!(bytes, expected);
    }
}

// Invalid alignment assumptions when converting doing Bits and Bytes optimizations
//
// https://github.com/sharksforarms/deku/issues/292
#[cfg(feature = "bits")]
#[test]
fn test_regression_292() {
    let test_data = [0x0f, 0xf0];

    #[derive(Debug, PartialEq, DekuRead)]
    #[deku(endian = "little")]
    struct Reader {
        #[deku(bits = 4)]
        field1: u8,
        field2: u8,
        #[deku(bits = 4)]
        field3: u8,
    }

    let mut cursor = Cursor::new(test_data);
    assert_eq!(
        Reader::from_reader((&mut cursor, 0)).unwrap().1,
        Reader {
            field1: 0,
            field2: 0xff,
            field3: 0,
        }
    );

    #[derive(Debug, PartialEq, DekuRead)]
    #[deku(endian = "little")]
    struct ReaderBits {
        #[deku(bits = 4)]
        field1: u8,
        #[deku(bits = 8)]
        field2: u8,
        #[deku(bits = 4)]
        field3: u8,
    }

    let mut cursor = Cursor::new(test_data);
    assert_eq!(
        ReaderBits::from_reader((&mut cursor, 0)).unwrap().1,
        ReaderBits {
            field1: 0,
            field2: 0xff,
            field3: 0,
        }
    );

    #[derive(Debug, PartialEq, DekuRead)]
    struct ReaderByteNoEndian {
        #[deku(bits = 4)]
        field1: u8,
        field2: u8,
        #[deku(bits = 4)]
        field3: u8,
    }

    let mut cursor = Cursor::new(test_data);
    assert_eq!(
        ReaderByteNoEndian::from_reader((&mut cursor, 0)).unwrap().1,
        ReaderByteNoEndian {
            field1: 0,
            field2: 0xff,
            field3: 0,
        }
    );

    #[derive(Debug, PartialEq, DekuRead)]
    struct ReaderBitPadding {
        #[deku(pad_bits_before = "4")]
        field2: u8,
        #[deku(bits = 4)]
        field3: u8,
    }

    let mut cursor = Cursor::new(test_data);
    assert_eq!(
        ReaderBitPadding::from_reader((&mut cursor, 0)).unwrap().1,
        ReaderBitPadding {
            field2: 0xff,
            field3: 0,
        }
    );

    #[derive(Debug, PartialEq, DekuRead)]
    struct ReaderBitPadding1 {
        #[deku(bits = 2)]
        field1: u8,
        #[deku(pad_bits_before = "2")]
        field2: u8,
        #[deku(bits = 4)]
        field3: u8,
    }

    let mut cursor = Cursor::new(test_data);
    assert_eq!(
        ReaderBitPadding1::from_reader((&mut cursor, 0)).unwrap().1,
        ReaderBitPadding1 {
            field1: 0,
            field2: 0xff,
            field3: 0,
        }
    );

    let test_data = [0b11000000, 0b00111111];

    #[derive(Debug, PartialEq, DekuRead)]
    #[deku(endian = "little")]
    struct ReaderTwo {
        #[deku(bits = 2)]
        field1: u8,
        field2: u8,
        #[deku(bits = 6)]
        field3: u8,
    }

    let mut cursor = Cursor::new(test_data);
    assert_eq!(
        ReaderTwo::from_reader((&mut cursor, 0)).unwrap().1,
        ReaderTwo {
            field1: 0b11,
            field2: 0,
            field3: 0b111111,
        }
    );

    let test_data = [0b11000000, 0b00000000, 0b00111111];

    #[derive(Debug, PartialEq, DekuRead)]
    #[deku(endian = "little")]
    struct ReaderU16Le {
        #[deku(bits = 2)]
        field1: u8,
        field2: u16,
        #[deku(bits = 6)]
        field3: u8,
    }

    let mut cursor = Cursor::new(test_data);
    assert_eq!(
        ReaderU16Le::from_reader((&mut cursor, 0)).unwrap().1,
        ReaderU16Le {
            field1: 0b11,
            field2: 0,
            field3: 0b111111,
        }
    );

    let test_data = [0b11000000, 0b00000000, 0b00111111];

    #[derive(Debug, PartialEq, DekuRead)]
    #[deku(endian = "big")]
    struct ReaderU16Be {
        #[deku(bits = 2)]
        field1: u8,
        field2: u16,
        #[deku(bits = 6)]
        field3: u8,
    }

    let mut cursor = Cursor::new(test_data);
    assert_eq!(
        ReaderU16Be::from_reader((&mut cursor, 0)).unwrap().1,
        ReaderU16Be {
            field1: 0b11,
            field2: 0,
            field3: 0b111111,
        }
    );

    let test_data = [0b11000000, 0b00000000, 0b01100001];

    #[derive(Debug, PartialEq, DekuRead)]
    #[deku(endian = "big")]
    struct ReaderI16Le {
        #[deku(bits = 2)]
        field1: i8,
        field2: i16,
        #[deku(bits = 6)]
        field3: i8,
    }

    let mut cursor = Cursor::new(test_data);
    assert_eq!(
        ReaderI16Le::from_reader((&mut cursor, 0)).unwrap().1,
        ReaderI16Le {
            field1: -0b01,
            field2: 1,
            field3: -0b011111,
        }
    );

    let test_data = [0b11000000, 0b00000000, 0b01100001];

    #[derive(Debug, PartialEq, DekuRead)]
    #[deku(endian = "big")]
    struct ReaderI16Be {
        #[deku(bits = 2)]
        field1: i8,
        field2: i16,
        #[deku(bits = 6)]
        field3: i8,
    }

    let mut cursor = Cursor::new(test_data);
    assert_eq!(
        ReaderI16Be::from_reader((&mut cursor, 0)).unwrap().1,
        ReaderI16Be {
            field1: -0b01,
            field2: 1,
            field3: -0b011111,
        }
    );
}

#[test]
fn issue_310() {
    use deku::prelude::*;

    #[allow(dead_code)]
    struct Result {}

    #[derive(DekuRead, DekuWrite)]
    struct Test {}
}

#[test]
fn issue_397() {
    use deku::prelude::*;

    #[derive(Debug, Copy, Clone, PartialEq, DekuRead, DekuWrite)]
    struct Header {
        kind: PacketType,
    }

    #[derive(Debug, Copy, Clone, PartialEq, DekuRead, DekuWrite)]
    #[deku(id_type = "u8", endian = "big")]
    enum PacketType {
        #[deku(id = 0)]
        Zero,
    }

    #[derive(Debug, Copy, Clone, PartialEq, DekuRead, DekuWrite)]
    struct Packet {
        header: Header,
        #[deku(ctx = "header")]
        payload: Payload,
    }

    #[derive(Debug, Copy, Clone, PartialEq, DekuRead, DekuWrite)]
    #[deku(ctx = "header: &Header", id = "header.kind")]
    enum Payload {
        #[deku(id = "PacketType::Zero")]
        Zero(u8),
    }
    let _ = Packet::from_bytes((&[0x00, 0x01], 0));
}

#[test]
fn issue_533() {
    #[derive(PartialEq, Debug, DekuRead)]
    #[deku(id_type = "u8", bits = "1")]
    pub enum BitsAndValue {
        #[deku(id = 0)]
        Zero,
        #[deku(id_pat = "_")]
        Other(#[deku(bits = 1)] u8),
    }
    let input = [0b0100_0000];
    let mut cursor = Cursor::new(input);
    let mut reader = Reader::new(&mut cursor);
    let v = BitsAndValue::from_reader_with_ctx(&mut reader, ()).unwrap();
    assert_eq!(v, BitsAndValue::Zero);

    let v = BitsAndValue::from_reader_with_ctx(&mut reader, ()).unwrap();
    assert_eq!(v, BitsAndValue::Other(1));
}
