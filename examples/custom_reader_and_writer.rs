use std::convert::TryInto;

use deku::ctx::BitSize;
use deku::writer::Writer;
use deku::{prelude::*, DekuWriter};
use no_std_io::io::{Seek, Write};

fn bit_flipper_read<R: std::io::Read + std::io::Seek>(
    field_a: u8,
    reader: &mut Reader<R>,
    bit_size: BitSize,
) -> Result<u8, DekuError> {
    // Access to previously read fields
    println!("field_a = 0x{field_a:X}");

    // Size of the current field
    println!("bit_size: {bit_size:?}");

    // read field_b, calling original func
    let value = u8::from_reader_with_ctx(reader, bit_size)?;

    // flip the bits on value if field_a is 0x01
    let value = if field_a == 0x01 { !value } else { value };

    Ok(value)
}

fn bit_flipper_write<W: Write + Seek>(
    field_a: u8,
    field_b: u8,
    writer: &mut Writer<W>,
    bit_size: BitSize,
) -> Result<(), DekuError> {
    // Access to previously written fields
    println!("field_a = 0x{field_a:X}");

    // value of field_b
    println!("field_b = 0x{field_b:X}");

    // Size of the current field
    println!("bit_size: {bit_size:?}");

    // flip the bits on value if field_a is 0x01
    let value = if field_a == 0x01 { !field_b } else { field_b };

    value.to_writer(writer, bit_size)
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuTest {
    field_a: u8,

    #[deku(
        reader = "bit_flipper_read(*field_a, deku::reader, BitSize(8))",
        writer = "bit_flipper_write(*field_a, *field_b, deku::writer, BitSize(8))"
    )]
    field_b: u8,
}

fn main() {
    let test_data = [0x01, 0b1001_0110];
    let mut cursor = std::io::Cursor::new(test_data);

    let (_read_amt, ret_read) = DekuTest::from_reader((&mut cursor, 0)).unwrap();

    assert_eq!(
        ret_read,
        DekuTest {
            field_a: 0x01,
            field_b: 0b0110_1001
        }
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data.to_vec(), ret_write);
}
