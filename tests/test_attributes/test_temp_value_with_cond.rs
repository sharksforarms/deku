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
        fspec1_6: u8,
        #[deku(bits = "1", temp, temp_value = "if payload2.is_some() {1} else {0}")]
        fspec1_7: u8,
        #[deku(bits = "1", temp, temp_value = "0")]
        fspec1_fx: u8,
        #[deku(skip, cond = "*fspec1_6 != 0x01", default = "None")]
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

#[test]
fn test_temp_value_with_cond_depending_on_skippable_entries() {
    // This test defines two control bytes, to control
    // if some payload is attached:
    // - byte 0 (fspec1):
    //   - bit 6+7 control the presence of payload1 & payload2
    //   - bit 8 controls if fspec2 is present (else fspec2 is interpreted as 0)
    // - optional byte 1 (fspec2):
    //   - bit 6+7 control the presence of payload3 & payload4
    // - payload1-payload4 (as Option)
    //
    // Notes:
    // - fspec1 & fspec2 are not present in the public API (temp)
    // - payload1 and payload2 depend on temp bits 6/7 of fspec1
    // - payload2 and payload3 depend on skippable bit 6/7 of fspec2 (cond+skip)

    #[deku_derive(DekuRead, DekuWrite)]
    #[derive(Debug, PartialEq)]
    #[deku(endian = "big")]
    struct TestStruct {
        // fspec1
        // ------
        #[deku(bits = "5", temp, temp_value = "0")]
        spare1: u8,
        #[deku(bits = "1", temp, temp_value = "if payload1.is_some() {1} else {0}")]
        fspec1_6: u8,
        #[deku(bits = "1", temp, temp_value = "if payload2.is_some() {1} else {0}")]
        fspec1_7: u8,
        #[deku(
            bits = "1",
            temp,
            temp_value = "if payload3.is_some()||payload4.is_some() {1} else {0}"
        )]
        fspec1_fx: u8,

        // fspec2
        // ------
        #[deku(
            skip,
            cond = "*fspec1_fx != 0x01",
            default = "0",
            bits = "5",
            temp,
            temp_value = "0"
        )]
        spare2: u8,
        #[deku(
            skip,
            cond = "*fspec1_fx != 0x01",
            default = "0",
            bits = "1",
            temp,
            temp_value = "if payload3.is_some() {1} else {0}"
        )]
        fspec2_6: u8,
        #[deku(
            skip,
            cond = "*fspec1_fx != 0x01",
            default = "0",
            bits = "1",
            temp,
            temp_value = "if payload4.is_some() {1} else {0}"
        )]
        fspec2_7: u8,
        #[deku(
            skip,
            cond = "*fspec1_fx != 0x01",
            default = "0",
            bits = "1",
            temp,
            temp_value = "0"
        )]
        fspec2_fx: u8,

        // payload
        // -------
        #[deku(skip, cond = "*fspec1_6 != 0x01", default = "None")]
        payload1: Option<u32>,
        #[deku(skip, cond = "*fspec1_7 != 0x01", default = "None")]
        payload2: Option<u32>,

        #[deku(skip, cond = "*fspec2_6 != 0x01", default = "None")]
        payload3: Option<u32>,
        #[deku(skip, cond = "*fspec2_7 != 0x01", default = "None")]
        payload4: Option<u32>,
    }

    {
        let d = TestStruct {
            payload1: Some(1),
            payload2: Some(2),
            payload3: Some(3),
            payload4: Some(4),
        };
        let bytes = d.to_bytes().unwrap();
        assert_eq!(
            bytes,
            vec![7, 6, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4]
        );
        let d2 = TestStruct::from_bytes((&bytes, 0)).unwrap().1;
        assert_eq!(d, d2);
    }
    {
        let d = TestStruct {
            payload1: None,
            payload2: Some(2),
            payload3: None,
            payload4: None,
        };
        let bytes = d.to_bytes().unwrap();
        assert_eq!(bytes, vec![2, 0, 0, 0, 2]);
        let d2 = TestStruct::from_bytes((&bytes, 0)).unwrap().1;
        assert_eq!(d, d2);
    }
    {
        let d = TestStruct {
            payload1: None,
            payload2: None,
            payload3: None,
            payload4: Some(4),
        };
        let bytes = d.to_bytes().unwrap();
        assert_eq!(bytes, vec![1, 2, 0, 0, 0, 4]);
        let d2 = TestStruct::from_bytes((&bytes, 0)).unwrap().1;
        assert_eq!(d, d2);
    }
    {
        let d = TestStruct {
            payload1: None,
            payload2: None,
            payload3: Some(3),
            payload4: None,
        };
        let bytes = d.to_bytes().unwrap();
        assert_eq!(bytes, vec![1, 4, 0, 0, 0, 3]);
        let d2 = TestStruct::from_bytes((&bytes, 0)).unwrap().1;
        assert_eq!(d, d2);
    }
    {
        let d = TestStruct {
            payload1: None,
            payload2: Some(2),
            payload3: Some(3),
            payload4: None,
        };
        let bytes = d.to_bytes().unwrap();
        assert_eq!(bytes, vec![3, 4, 0, 0, 0, 2, 0, 0, 0, 3]);
        let d2 = TestStruct::from_bytes((&bytes, 0)).unwrap().1;
        assert_eq!(d, d2);
    }
}
