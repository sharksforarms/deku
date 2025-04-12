use deku::prelude::*;

#[test]
fn test_temp_value_with_cond() {
    #[deku_derive(DekuRead, DekuWrite)]
    #[derive(Debug, PartialEq)]
    #[deku(endian = "big")]
    struct TestStruct {
        #[deku(bits = "5", temp, temp_value = "0")]
        spare1: u8,
        #[deku(bits = "1", temp, temp_value = "if payload1.is_some() {1} else {0}")]
        fspec1_8: u8,
        #[deku(bits = "1", temp, temp_value = "if payload2.is_some() {1} else {0}")]
        fspec1_7: u8,
        #[deku(bits = "1", temp, temp_value = "0")]
        fspec1_fx: u8,
        #[deku(skip, cond = "*fspec1_8 != 0x01", default = "None")]
        payload1: Option<u32>,
        #[deku(skip, cond = "*fspec1_7 != 0x01", default = "None")]
        payload2: Option<u32>,
    }

    {
        let d = TestStruct {
            payload1: Some(1),
            payload2: Some(2),
        };
        let bytes = d.to_bytes().unwrap();
        assert_eq!(bytes, vec![6, 0, 0, 0, 1, 0, 0, 0, 2]);
        let d2 = TestStruct::from_bytes((&bytes, 0)).unwrap().1;
        assert_eq!(d, d2);
    }
    {
        let d = TestStruct {
            payload1: None,
            payload2: Some(2),
        };
        let bytes = d.to_bytes().unwrap();
        assert_eq!(bytes, vec![2, 0, 0, 0, 2]);
        let d2 = TestStruct::from_bytes((&bytes, 0)).unwrap().1;
        assert_eq!(d, d2);
    }
    {
        let d = TestStruct {
            payload1: Some(1),
            payload2: None,
        };
        let bytes = d.to_bytes().unwrap();
        assert_eq!(bytes, vec![4, 0, 0, 0, 1]);
        let d2 = TestStruct::from_bytes((&bytes, 0)).unwrap().1;
        assert_eq!(d, d2);
    }
    {
        let d = TestStruct {
            payload1: None,
            payload2: None,
        };
        let bytes = d.to_bytes().unwrap();
        assert_eq!(bytes, vec![0]);
        let d2 = TestStruct::from_bytes((&bytes, 0)).unwrap().1;
        assert_eq!(d, d2);
    }
}
