use deku::prelude::*;
use hexlit::hex;
use rstest::*;

#[derive(DekuRead, DekuWrite, Debug, PartialEq, Eq)]
pub struct Test {
    // how many following bytes to skip
    skip_u8: u8,
    #[deku(seek_from_current = "*skip_u8")]
    byte: u8,
}

#[rstest(input, expected,
    case(&hex!("010020"), Test{ skip_u8: 1, byte: 0x20 }),
)]
fn test_seek_from_current(input: &[u8], expected: Test) {
    let input = input.to_vec();
    // TODO: fix, "too much data"
    //let ret_read = Test::try_from(input.as_slice()).unwrap();

    let mut cursor = std::io::Cursor::new(input.clone());
    let (_, ret_read) = Test::from_reader((&mut cursor, 0)).unwrap();

    assert_eq!(ret_read, expected);

    let bytes = ret_read.to_bytes().unwrap();
    assert_eq!(bytes, input);
}

#[derive(DekuRead, DekuWrite, Debug, PartialEq, Eq)]
#[deku(seek_from_current = "skip", ctx = "skip: usize")]
pub struct SeekCtxBefore {
    byte: u8,
}

#[rstest(input, ctx, expected,
    case(&hex!("0003"), 1, SeekCtxBefore{ byte: 0x03 }),
    case(&hex!("000004"), 2, SeekCtxBefore{ byte: 0x04 }),
)]
fn test_seek_ctx_before(input: &[u8], ctx: usize, expected: SeekCtxBefore) {
    use std::io::Cursor;
    let input = input.to_vec();

    let mut cursor = std::io::Cursor::new(input.clone());
    let mut reader = Reader::new(&mut cursor);
    let ret_read = SeekCtxBefore::from_reader_with_ctx(&mut reader, ctx).unwrap();

    assert_eq!(ret_read, expected);

    let mut buf = vec![];
    let mut cursor = Cursor::new(&mut buf);
    let mut writer = Writer::new(&mut cursor);
    let _ = ret_read.to_writer(&mut writer, ctx).unwrap();
    assert_eq!(buf, input);
}

#[derive(DekuRead, DekuWrite, Debug, PartialEq, Eq)]
#[deku(seek_from_start = "1")]
pub struct SeekCtxBeforeStart {
    byte: u8,
}

#[rstest(input, expected,
    case(&hex!("0003"), SeekCtxBeforeStart{ byte: 0x03 }),
    case(&hex!("00ff"), SeekCtxBeforeStart{ byte: 0xff }),
)]
fn test_seek_ctx_start(input: &[u8], expected: SeekCtxBeforeStart) {
    use std::io::Cursor;
    let input = input.to_vec();

    let mut cursor = std::io::Cursor::new(input.clone());
    let mut reader = Reader::new(&mut cursor);
    let ret_read = SeekCtxBeforeStart::from_reader_with_ctx(&mut reader, ()).unwrap();

    assert_eq!(ret_read, expected);

    let mut buf = vec![];
    let mut cursor = Cursor::new(&mut buf);
    let mut writer = Writer::new(&mut cursor);
    let _ = ret_read.to_writer(&mut writer, ()).unwrap();
    assert_eq!(buf, input);
}

#[derive(DekuRead, DekuWrite, Debug, PartialEq, Eq)]
#[deku(seek_from_end = "-2")]
pub struct SeekCtxBeforeEnd {
    byte: u8,
}

#[rstest(input, expected,
    case(&hex!("000300"), SeekCtxBeforeEnd{ byte: 0x03 }),
    case(&hex!("00ff00"), SeekCtxBeforeEnd{ byte: 0xff }),
)]
fn test_seek_ctx_end(input: &[u8], expected: SeekCtxBeforeEnd) {
    use std::io::Cursor;
    let input = input.to_vec();

    let mut cursor = std::io::Cursor::new(input.clone());
    let mut reader = Reader::new(&mut cursor);
    let ret_read = SeekCtxBeforeEnd::from_reader_with_ctx(&mut reader, ()).unwrap();

    assert_eq!(ret_read, expected);

    let mut buf = vec![0, 0, 0];
    let mut cursor = Cursor::new(&mut buf);
    let mut writer = Writer::new(&mut cursor);
    let _ = ret_read.to_writer(&mut writer, ()).unwrap();
    assert_eq!(buf, input);
}
