use core::convert::{TryFrom, TryInto};

use deku::prelude::*;

/// Update field value
#[test]
fn test_update() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        #[deku(update = "5")]
        field_a: u8,
    }

    // Update `field_a` to 5
    let test_data: Vec<u8> = [0x01].to_vec();

    let mut ret_read = TestStruct::try_from(test_data.as_slice()).unwrap();
    assert_eq!(TestStruct { field_a: 0x01 }, ret_read);

    // `field_a` field should now be increased
    ret_read.update(()).unwrap();
    assert_eq!(0x05, ret_read.field_a);

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!([0x05].to_vec(), ret_write);
}

/// Update from field on `self`
#[test]
fn test_update_from_field() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        #[deku(update = "self.data.len()")]
        count: u8,
        #[deku(count = "count")]
        data: Vec<u8>,
    }

    // Update the value of `count` to the length of `data`
    let test_data: Vec<u8> = [0x02, 0xaa, 0xbb].to_vec();

    // Read
    let mut ret_read = TestStruct::try_from(test_data.as_slice()).unwrap();
    assert_eq!(
        TestStruct {
            count: 0x02,
            data: vec![0xaa, 0xbb]
        },
        ret_read
    );

    // Add an item to the vec
    ret_read.data.push(0xff);

    // `count` field should now be increased
    ret_read.update(()).unwrap();
    assert_eq!(3, ret_read.count);

    // Write
    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!([0x03, 0xaa, 0xbb, 0xff].to_vec(), ret_write);
}

/// Update error
#[test]
#[cfg(feature = "descriptive-errors")]
#[should_panic(
    expected = "Parse(\"error parsing int: out of range integral type conversion attempted\")"
)]
fn test_update_error() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        #[deku(update = "256")]
        count: u8,
    }

    let mut val = TestStruct { count: 0x01 };

    val.update(()).unwrap();
}

/// Test update_also propagates update to nested struct
#[test]
fn test_update_also() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct Inner {
        #[deku(update = "0xFF")]
        value: u8,
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct Outer {
        #[deku(update_also)]
        inner: Inner,
    }

    let mut outer = Outer {
        inner: Inner { value: 0x00 },
    };

    outer.update(()).unwrap();
    assert_eq!(0xFF, outer.inner.value);
}

/// Test update_also without update_ctx (uses unit context)
#[test]
fn test_update_also_no_ctx() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct Inner {
        #[deku(update = "0xAB")]
        value: u8,
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct Outer {
        #[deku(update_also)]
        inner: Inner,
        other: u8,
    }

    let mut outer = Outer {
        inner: Inner { value: 0x00 },
        other: 0x01,
    };

    outer.update(()).unwrap();
    assert_eq!(0xAB, outer.inner.value);
    assert_eq!(0x01, outer.other);
}

/// Test update_ctx passes context to nested struct
#[test]
fn test_update_ctx() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(update_ctx = "new_len: u16")]
    struct Header {
        #[deku(update = "new_len")]
        length: u16,
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct Container {
        #[deku(update_also, update_ctx = "self.data.len() as u16")]
        header: Header,
        #[deku(count = "header.length")]
        data: Vec<u8>,
    }

    let mut container = Container {
        header: Header { length: 2 },
        data: vec![0xAA, 0xBB],
    };

    // Add more data
    container.data.push(0xCC);
    container.data.push(0xDD);

    // Update should propagate length to header
    container.update(()).unwrap();
    assert_eq!(4, container.header.length);
}

/// Test update_ctx with multiple context values
#[test]
fn test_update_ctx_multiple() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(update_ctx = "val_a: u8, val_b: u8")]
    struct Inner {
        #[deku(update = "val_a")]
        a: u8,
        #[deku(update = "val_b")]
        b: u8,
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct Outer {
        #[deku(update_also, update_ctx = "0x11, 0x22")]
        inner: Inner,
    }

    let mut outer = Outer {
        inner: Inner { a: 0, b: 0 },
    };

    outer.update(()).unwrap();
    assert_eq!(0x11, outer.inner.a);
    assert_eq!(0x22, outer.inner.b);
}

/// Test update_with calls custom function
#[test]
fn test_update_with() {
    fn custom_update(value: &mut u8, _ctx: ()) -> Result<(), DekuError> {
        *value = value.wrapping_add(10);
        Ok(())
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        #[deku(update_with = "custom_update")]
        value: u8,
    }

    let mut test = TestStruct { value: 5 };
    test.update(()).unwrap();
    assert_eq!(15, test.value);
}

/// Test update_with with context
#[test]
fn test_update_with_ctx() {
    fn custom_update(value: &mut u16, ctx: (u8, u8)) -> Result<(), DekuError> {
        *value = (ctx.0 as u16) << 8 | (ctx.1 as u16);
        Ok(())
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        #[deku(update_with = "custom_update", update_ctx = "0xAB, 0xCD")]
        value: u16,
    }

    let mut test = TestStruct { value: 0 };
    test.update(()).unwrap();
    assert_eq!(0xABCD, test.value);
}

/// Test update_with as method on struct
#[test]
fn test_update_with_method() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        #[deku(update_with = "Self::update_checksum", update_ctx = "self.a, self.b")]
        checksum: u8,
        a: u8,
        b: u8,
    }

    impl TestStruct {
        fn update_checksum(checksum: &mut u8, ctx: (u8, u8)) -> Result<(), DekuError> {
            *checksum = ctx.0 ^ ctx.1;
            Ok(())
        }
    }

    let mut test = TestStruct {
        checksum: 0,
        a: 0xAA,
        b: 0x55,
    };

    test.update(()).unwrap();
    assert_eq!(0xFF, test.checksum); // 0xAA ^ 0x55 = 0xFF
}

/// Test combining update_also and update_with in same struct
#[test]
fn test_mixed_update_attributes() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct Inner {
        #[deku(update = "0xBB")]
        value: u8,
    }

    fn double(value: &mut u8, _ctx: ()) -> Result<(), DekuError> {
        *value = value.wrapping_mul(2);
        Ok(())
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct Outer {
        #[deku(update_also)]
        inner: Inner,
        #[deku(update = "self.count + 1")]
        count: u8,
        #[deku(update_with = "double")]
        doubled: u8,
    }

    let mut outer = Outer {
        inner: Inner { value: 0 },
        count: 5,
        doubled: 3,
    };

    outer.update(()).unwrap();
    assert_eq!(0xBB, outer.inner.value);
    assert_eq!(6, outer.count);
    assert_eq!(6, outer.doubled);
}
