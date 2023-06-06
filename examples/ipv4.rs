use std::convert::TryInto;
use std::net::Ipv4Addr;

use deku::prelude::*;
use hexlit::hex;

/// Ipv4 Header
/// ```text
///     0                   1                   2                   3
///     0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
///    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///    |Version|  IHL  |    DSCP   |ECN|         Total Length          |
///    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///    |         Identification        |Flags|      Fragment Offset    |
///    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///    |  Time to Live |    Protocol   |         Header Checksum       |
///    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///    |                       Source Address                          |
///    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///    |                    Destination Address                        |
///    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///    |                    Options                    |    Padding    |
///    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
pub struct Ipv4Header {
    #[deku(bits = "4")]
    pub version: u8, // Version
    #[deku(bits = "4")]
    pub ihl: u8, // Internet Header Length
    #[deku(bits = "6")]
    pub dscp: u8, // Differentiated Services Code Point
    #[deku(bits = "2")]
    pub ecn: u8, // Explicit Congestion Notification
    pub length: u16,         // Total Length
    pub identification: u16, // Identification
    #[deku(bits = "3")]
    pub flags: u8, // Flags
    #[deku(bits = "13")]
    pub offset: u16, // Fragment Offset
    pub ttl: u8,             // Time To Live
    pub protocol: u8,        // Protocol
    pub checksum: u16,       // Header checksum
    pub src: Ipv4Addr,       // Source IP Address
    pub dst: Ipv4Addr,       /* Destination IP Address
                              * options
                              * padding */
}

fn main() {
    let test_data = hex!("4500004b0f490000801163a591fea0ed91fd02cb").to_vec();

    let mut cursor = std::io::Cursor::new(test_data.clone());
    let mut reader = deku::reader::Reader::new(&mut cursor);
    let ip_header = Ipv4Header::from_reader_with_ctx(&mut reader, ()).unwrap();

    assert_eq!(
        Ipv4Header {
            version: 4,
            ihl: 5,
            ecn: 0,
            dscp: 0,
            length: 75,
            identification: 0x0f49,
            flags: 0,
            offset: 0,
            ttl: 128,
            protocol: 17,
            checksum: 0x63a5,
            src: Ipv4Addr::new(145, 254, 160, 237),
            dst: Ipv4Addr::new(145, 253, 2, 203),
        },
        ip_header
    );

    let ip_header_data: Vec<u8> = ip_header.try_into().unwrap();

    assert_eq!(test_data, ip_header_data);
}
