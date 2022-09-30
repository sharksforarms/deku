use alloc_counter::AllocCounterSystem;
use deku::ctx::Endian;
use deku::prelude::*;

// Smoke tests for allocation counting

#[global_allocator]
static A: AllocCounterSystem = AllocCounterSystem;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(ctx = "_endian: Endian")]
struct NestedStruct {
    field_a: u8,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(type = "u8", ctx = "_endian: Endian")]
enum NestedEnum {
    #[deku(id = "0x01")]
    VarA(u8),
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(type = "u32", bytes = "2", ctx = "_endian: Endian")]
enum NestedEnum2 {
    #[deku(id = "0x01")]
    VarA(u8),
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
struct TestDeku {
    field_a: u8,
    field_b: u16,
    field_c: NestedStruct,
    field_d: NestedEnum,
    #[deku(count = "1")]
    field_e: Vec<u8>, // 1 alloc
    field_f: [u8; 3],
    #[deku(bits = "3")]
    field_g: u8, // 1 alloc (bits read)
    #[deku(bits = "5")]
    field_h: u8, // 1 alloc (bits read)
    field_i: NestedEnum2,
}

mod tests {
    use super::*;
    use alloc_counter::count_alloc;
    use hexlit::hex;
    use std::convert::TryFrom;

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_simple() {
        let input = hex!("aa_bbbb_cc_0102_dd_ffffff_aa_0100ff");

        assert_eq!(
            count_alloc(|| {
                let _ = TestDeku::try_from(input.as_ref()).unwrap();
            })
            .0,
            (3, 0, 3)
        );
    }
}
