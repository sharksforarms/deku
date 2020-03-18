#[cfg(test)]
mod tests {
    use bitvec::prelude::*;
    use deku::{BitsReader, BitsWriter, DekuRead, DekuWrite};

    #[test]
    fn it_works() {
        #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
        struct DekuTest {
            #[deku(endian = "little", bits = "7")]
            id: u8,
            #[deku(endian = "little", bytes = "4")]
            field: u32,
            #[deku(endian = "little", bits = "1")]
            field2: u8,
            #[deku(endian = "big", bytes = "2")]
            field3: u16,
        }

        let test_data: &[u8] = [
            0b1000_0011,
            0b0000_0000,
            0x00,
            0x00,
            0b0000_0011,
            0b1010_1010, //0xAA
            0b1011_1011, //0xBB
        ]
        .as_ref();

        let test_deku: DekuTest = test_data.into();

        assert_eq!(
            test_deku,
            DekuTest {
                id: 0b0100_0001,
                field: 0b1000_0000_0000_0000_0000_0000_0000_0001,
                field2: 1,
                field3: 0xBBAA,
            }
        );

        dbg!(&test_deku);
        let test_deku: Vec<u8> = test_deku.into();
        assert_eq!(test_data.to_vec(), test_deku);
    }
}
