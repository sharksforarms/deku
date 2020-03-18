use hex_literal::hex;

use bitvec::prelude::*;
use deku::{BitsReader, BitsWriter, DekuRead, DekuWrite};

/// Ipv4 Header
/// ```text
///     0                   1                   2                   3
///     0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
///    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///    |Version|  IHL  |    DSCP   | ECN |        Total Length         |
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
pub struct Ipv4Header {
    #[deku(bits = "4")]
    pub version: u8, // Version
    #[deku(bits = "4")]
    pub ihl: u8, // Internet Header Length
    #[deku(bits = "6")]
    pub dscp: u8, // Differentiated Services Code Point
    #[deku(bits = "2")]
    pub ecn: u8, // Explicit Congestion Notification
    #[deku(bytes = "2")]
    pub length: u16, // Total Length
    #[deku(bytes = "2")]
    pub identification: u16, // Identification
    #[deku(bits = "3")]
    pub flags: u8, // Flags
    #[deku(bits = "13")]
    pub offset: u16, // Fragment Offset
    #[deku(bytes = "1")]
    pub ttl: u8, // Time To Live
    #[deku(bytes = "1")]
    pub protocol: u8, // Protocol
    #[deku(bytes = "2")]
    pub checksum: u16, // Header checksum
    #[deku(bytes = "4")]
    pub src: u32, // Source IP Address
    #[deku(bytes = "4")]
    pub dst: u32, // Destination IP Address
                  // options
                  // padding
}

fn main() {
    let test_data = hex!("450000502bc1400040068f37c0a8016bc01efd7d").to_vec();

    let ip_header = Ipv4Header::from(test_data.as_ref());

    assert_eq!(
        Ipv4Header {
            version: 4,
            ihl: 5,
            ecn: 0,
            dscp: 0,
            length: 80,
            identification: 0x2bc1,
            flags: 2,
            offset: 0,
            ttl: 64,
            protocol: 6,
            checksum: 0x8f37,
            src: 3232235883,
            dst: 3223256445,
        },
        ip_header
    );

    let ip_header_data: Vec<u8> = ip_header.into();

    assert_eq!(test_data, ip_header_data);
}
