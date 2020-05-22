use deku::prelude::*;

fn bit_flipper_read(
    field_a: u8,
    rest: &BitSlice<Msb0, u8>,
    bit_size: Option<usize>,
) -> Result<(&BitSlice<Msb0, u8>, u8), DekuError> {
    // Access to previously read fields
    println!("field_a = 0x{:X}", field_a);

    // The current rest
    println!("rest = {:?}", rest);

    // Size of the current field
    println!("bit_size: {:?}", bit_size);

    // read field_b, calling original func
    let (rest, value) = u8::read(rest, bit_size)?;

    // flip the bits on value if field_a is 0x01
    let value = if field_a == 0x01 { !value } else { value };

    Ok((rest, value))
}

fn bit_flipper_write(field_a: u8, field_val: u8, bit_size: Option<usize>) -> BitVec<Msb0, u8> {
    // Access to previously written fields
    println!("field_a = 0x{:X}", field_a);

    // value of field_b
    println!("field_b = 0x{:X}", field_val);

    // Size of the current field
    println!("bit_size: {:?}", bit_size);

    // flip the bits on value if field_a is 0x01
    let value = if field_a == 0x01 {
        !field_val
    } else {
        field_val
    };

    value.write(bit_size)
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuTest {
    field_a: u8,

    #[deku(
        bits = "8",
        reader = "bit_flipper_read(field_a, rest, field_bits)",
        writer = "bit_flipper_write(input.field_a, field_val, field_bits)"
    )]
    field_b: u8,
}

fn main() {
    let test_data: &[u8] = [0x01, 0b1001_0110].as_ref();

    let (_rest, ret_read) = DekuTest::from_bytes(test_data).unwrap();

    assert_eq!(
        ret_read,
        DekuTest {
            field_a: 0x01,
            field_b: 0b0110_1001
        }
    );

    let ret_write: Vec<u8> = ret_read.into();
    assert_eq!(test_data.to_vec(), ret_write);
}
