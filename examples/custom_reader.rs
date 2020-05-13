use deku::prelude::*;
use std::convert::TryFrom;

fn bit_flipper(
    field_a: u8,
    rest: &BitSlice<Msb0, u8>,
    len: usize,
) -> Result<(&BitSlice<Msb0, u8>, u8), DekuError> {
    // Access to previously parsed fields
    println!("field_a = 0x{:X}", field_a);

    // The current rest
    println!("rest = {:?}", rest);

    // Length of the current: i.e. field_b bits = "8"
    println!("field_bits: {}", len);

    // read field_b, calling original func
    let (rest, value) = u8::read(rest, len)?;

    // flip the bits on value if field_a is 0x01
    let value = if field_a == 0x01 { !value } else { value };

    Ok((rest, value))
}

#[derive(Debug, PartialEq, DekuRead)]
struct DekuTest {
    field_a: u8,
    #[deku(bits = "8", reader = "bit_flipper(field_a, rest, field_bits)")]
    field_b: u8,
}

fn main() {
    let test_data: &[u8] = [0x01, 0b1001_0110].as_ref();

    let (_rest, test_deku) = DekuTest::from_bytes(test_data).unwrap();

    assert_eq!(
        test_deku,
        DekuTest {
            field_a: 0x01,
            field_b: 0b0110_1001
        }
    )
}
