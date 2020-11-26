use deku::prelude::*;
use hexlit::hex;

#[derive(Debug, DekuRead, DekuWrite, PartialEq)]
#[deku(type = "u8", bits = "2")]
pub enum FrameType {
    #[deku(id = "0")]
    Management,
    #[deku(id = "1")]
    Control,
    #[deku(id = "2")]
    Data,
}

#[derive(Debug, DekuRead, DekuWrite, PartialEq)]
pub struct Flags {
    #[deku(bits = 1)]
    pub to_ds: u8,
    #[deku(bits = 1)]
    pub from_ds: u8,
    #[deku(bits = 1)]
    pub more_fragments: u8,
    #[deku(bits = 1)]
    pub retry: u8,
    #[deku(bits = 1)]
    pub power_management: u8,
    #[deku(bits = 1)]
    pub more_data: u8,
    #[deku(bits = 1)]
    pub protected_frame: u8,
    #[deku(bits = 1)]
    pub order: u8,
}

#[derive(Debug, DekuRead, DekuWrite, PartialEq)]
pub struct FrameControl {
    #[deku(bits = 2)]
    pub protocol_version: u8,
    pub frame_type: FrameType,
    #[deku(bits = 4)]
    pub sub_type: u8,

    pub flags: Flags,
}

fn main() {
    let data = hex!("85d1").to_vec();
    println!("{:x?}", data);
    let bit_data = BitVec::<Lsb0, _>::from_vec(data);

    let (_, control_frame) = FrameControl::read(&bit_data, ()).unwrap();
    assert_eq!(
        control_frame,
        FrameControl {
            protocol_version: 1,
            frame_type: FrameType::Control,
            sub_type: 8,

            flags: Flags {
                to_ds: 1,
                from_ds: 0,
                more_fragments: 0,
                retry: 0,
                power_management: 1,
                more_data: 0,
                protected_frame: 1,
                order: 1,
            }
        }
    );
    println!("{:#?}", control_frame);
}
