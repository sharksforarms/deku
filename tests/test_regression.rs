use deku::prelude::*;

// Invalid alignment assumptions when converting
// BitSlice to type
//
// https://github.com/sharksforarms/deku/issues/224
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
    #[deku(type = "u8", bits = "2")]
    pub enum One {
        Start = 0,
        Go = 1,
        Stop = 2,
    }

    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(type = "u8", bits = "4")]
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
        #[deku(bits = "2")]
        pub i: u8,
        #[deku(bits = "4")]
        pub o: u8,
    }

    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    pub struct Op2 {
        #[deku(bits = "8")]
        pub w: u8,
        #[deku(bits = "6")]
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
        let (_, BitsBytes { bits, bytes }) =
            BitsBytes::from_reader((&mut data.as_slice(), 0)).unwrap();

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
        let (_, BitsBytes { bits, bytes }) =
            BitsBytes::from_reader((&mut data.as_slice(), 0)).unwrap();

        assert_eq!(bits, expected);
        assert_eq!(bytes, expected);
    }
}

// Invalid alignment assumptions when converting doing Bits and Bytes optimizations
//
// https://github.com/sharksforarms/deku/issues/292
#[test]
fn test_regression_292() {
    let test_data = [0x0f, 0xf0];

    #[derive(Debug, PartialEq, DekuRead)]
    #[deku(endian = "little")]
    struct Container {
        #[deku(bits = 4)]
        field1: u8,
        field2: u8,
        #[deku(bits = 4)]
        field3: u8,
    }

    assert_eq!(
        Container::from_reader((&mut test_data.as_slice(), 0))
            .unwrap()
            .1,
        Container {
            field1: 0,
            field2: 0xff,
            field3: 0,
        }
    );

    #[derive(Debug, PartialEq, DekuRead)]
    #[deku(endian = "little")]
    struct ContainerBits {
        #[deku(bits = 4)]
        field1: u8,
        #[deku(bits = 8)]
        field2: u8,
        #[deku(bits = 4)]
        field3: u8,
    }

    assert_eq!(
        ContainerBits::from_reader((&mut test_data.as_slice(), 0))
            .unwrap()
            .1,
        ContainerBits {
            field1: 0,
            field2: 0xff,
            field3: 0,
        }
    );

    #[derive(Debug, PartialEq, DekuRead)]
    struct ContainerByteNoEndian {
        #[deku(bits = 4)]
        field1: u8,
        field2: u8,
        #[deku(bits = 4)]
        field3: u8,
    }

    assert_eq!(
        ContainerByteNoEndian::from_reader((&mut test_data.as_slice(), 0))
            .unwrap()
            .1,
        ContainerByteNoEndian {
            field1: 0,
            field2: 0xff,
            field3: 0,
        }
    );

    #[derive(Debug, PartialEq, DekuRead)]
    struct ContainerBitPadding {
        #[deku(pad_bits_before = "4")]
        field2: u8,
        #[deku(bits = 4)]
        field3: u8,
    }

    assert_eq!(
        ContainerBitPadding::from_reader((&mut test_data.as_slice(), 0))
            .unwrap()
            .1,
        ContainerBitPadding {
            field2: 0xff,
            field3: 0,
        }
    );

    #[derive(Debug, PartialEq, DekuRead)]
    struct ContainerBitPadding1 {
        #[deku(bits = 2)]
        field1: u8,
        #[deku(pad_bits_before = "2")]
        field2: u8,
        #[deku(bits = 4)]
        field3: u8,
    }

    assert_eq!(
        ContainerBitPadding1::from_reader((&mut test_data.as_slice(), 0))
            .unwrap()
            .1,
        ContainerBitPadding1 {
            field1: 0,
            field2: 0xff,
            field3: 0,
        }
    );

    let test_data = [0b11000000, 0b00111111];

    #[derive(Debug, PartialEq, DekuRead)]
    #[deku(endian = "little")]
    struct ContainerTwo {
        #[deku(bits = 2)]
        field1: u8,
        field2: u8,
        #[deku(bits = 6)]
        field3: u8,
    }

    assert_eq!(
        ContainerTwo::from_reader((&mut test_data.as_slice(), 0))
            .unwrap()
            .1,
        ContainerTwo {
            field1: 0b11,
            field2: 0,
            field3: 0b111111,
        }
    );

    let test_data = [0b11000000, 0b00000000, 0b00111111];

    #[derive(Debug, PartialEq, DekuRead)]
    #[deku(endian = "little")]
    struct ContainerU16Le {
        #[deku(bits = 2)]
        field1: u8,
        field2: u16,
        #[deku(bits = 6)]
        field3: u8,
    }

    assert_eq!(
        ContainerU16Le::from_reader((&mut test_data.as_slice(), 0))
            .unwrap()
            .1,
        ContainerU16Le {
            field1: 0b11,
            field2: 0,
            field3: 0b111111,
        }
    );

    let test_data = [0b11000000, 0b00000000, 0b00111111];

    #[derive(Debug, PartialEq, DekuRead)]
    #[deku(endian = "big")]
    struct ContainerU16Be {
        #[deku(bits = 2)]
        field1: u8,
        field2: u16,
        #[deku(bits = 6)]
        field3: u8,
    }

    assert_eq!(
        ContainerU16Be::from_reader((&mut test_data.as_slice(), 0))
            .unwrap()
            .1,
        ContainerU16Be {
            field1: 0b11,
            field2: 0,
            field3: 0b111111,
        }
    );

    let test_data = [0b11000000, 0b00000000, 0b01100001];

    #[derive(Debug, PartialEq, DekuRead)]
    #[deku(endian = "big")]
    struct ContainerI16Le {
        #[deku(bits = 2)]
        field1: i8,
        field2: i16,
        #[deku(bits = 6)]
        field3: i8,
    }

    assert_eq!(
        ContainerI16Le::from_reader((&mut test_data.as_slice(), 0))
            .unwrap()
            .1,
        ContainerI16Le {
            field1: -0b01,
            field2: 1,
            field3: -0b011111,
        }
    );

    let test_data = [0b11000000, 0b00000000, 0b01100001];

    #[derive(Debug, PartialEq, DekuRead)]
    #[deku(endian = "big")]
    struct ContainerI16Be {
        #[deku(bits = 2)]
        field1: i8,
        field2: i16,
        #[deku(bits = 6)]
        field3: i8,
    }

    assert_eq!(
        ContainerI16Be::from_reader((&mut test_data.as_slice(), 0))
            .unwrap()
            .1,
        ContainerI16Be {
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
