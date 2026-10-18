use renamed_deku::prelude::*;

#[derive(DekuRead, DekuWrite)]
#[deku(endian = "big")]
struct Packet {
    value: u8,
}

#[test]
fn generated_reader_and_writer_support_a_renamed_dependency_with_logging() {
    let (_, packet) = Packet::from_bytes((&[0x42][..], 0)).unwrap();
    assert_eq!(packet.value, 0x42);
    assert_eq!(packet.to_bytes().unwrap(), [0x42]);
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(id_type = "u8")]
enum MessageKind {
    #[deku(id = "1")]
    Ping,
    #[deku(id = "2")]
    Pong,
}

#[test]
fn generated_enum_reader_and_writer_support_a_renamed_dependency() {
    let (_, kind) = MessageKind::from_bytes((&[0x02][..], 0)).unwrap();
    assert_eq!(kind, MessageKind::Pong);
    assert_eq!(kind.to_bytes().unwrap(), [0x02]);
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
struct Flags {
    #[deku(bits = "3")]
    mode: u8,
    #[deku(bits = "5")]
    count: u8,
}

#[test]
fn generated_bit_run_logging_supports_a_renamed_dependency() {
    let (_, flags) = Flags::from_bytes((&[0b101_10010][..], 0)).unwrap();
    assert_eq!(flags, Flags { mode: 5, count: 18 });
    assert_eq!(flags.to_bytes().unwrap(), [0b101_10010]);
}
