use deku::prelude::*;

// Invalid allignment assumptions when converting
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
    let _packet = Packet::from_bytes((&bytes, 0)).unwrap();
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

        let data = &[a, b, c, a, b, c];
        let (_, BitsBytes { bits, bytes }) = BitsBytes::from_bytes((data, 0)).unwrap();

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

        let data = &[a, b, c, a, b, c];
        let (_, BitsBytes { bits, bytes }) = BitsBytes::from_bytes((data, 0)).unwrap();

        assert_eq!(bits, expected);
        assert_eq!(bytes, expected);
    }
}
