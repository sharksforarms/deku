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
