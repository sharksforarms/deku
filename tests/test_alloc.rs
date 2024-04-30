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
#[deku(id_type = "u8", ctx = "_endian: Endian")]
enum NestedEnum {
    #[deku(id = "0x01")]
    VarA(u8),
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(id_type = "u32", bytes = 2, ctx = "_endian: Endian")]
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
    #[deku(bits = 3)]
    field_g: u8, // 3 allocs (read_bits(Ordering::Greater))
    #[deku(bits = 5)]
    field_h: u8, // 1 alloc (read_bits(Ordering::Equal))
                 //field_i: NestedEnum2,
}

mod tests {
    use alloc_counter::count_alloc;
    use hexlit::hex;

    use super::*;

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_simple() {
        let input = hex!("aa_bbbb_cc_0102_dd_ffffff_aa_0100ff");

        assert_eq!(
            count_alloc(|| {
                let _ = TestDeku::from_reader((&mut input.as_slice(), 0)).unwrap();
            })
            .0,
            (5, 0, 5)
        );
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_simple_write() {
        let input = hex!("aa_bbbb_cc_0102_dd_ffffff_aa_0100ff");
        let t = TestDeku::from_reader((&mut input.as_slice(), 0)).unwrap().1;

        assert_eq!(
            count_alloc(|| {
                t.to_bytes().unwrap();
            })
            .0,
            (2, 1, 2)
        );
    }
}
