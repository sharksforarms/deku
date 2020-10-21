use deku::prelude::*;
use std::convert::TryFrom;

#[test]
fn test_map() {
    #[derive(PartialEq, Debug, DekuRead)]
    struct TestStruct {
        #[deku(map = "|field: u8| -> Result<_, DekuError> { Ok(field.to_string()) }")]
        field_a: String,
        #[deku(map = "TestStruct::map_field_b")]
        field_b: String,
    }

    impl TestStruct {
        fn map_field_b(field_b: u8) -> Result<String, DekuError> {
            Ok(field_b.to_string())
        }
    }

    let test_data: Vec<u8> = [0x01, 0x02].to_vec();

    let ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
    assert_eq!(
        TestStruct {
            field_a: "1".to_string(),
            field_b: "2".to_string(),
        },
        ret_read
    );
}
