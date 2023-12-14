//! Example of a close replacement for deku::input
use deku::prelude::*;
use std::io::{self, Cursor, Read};

/// Every read to this struct will be saved into an internal cache. This is to keep the cache
/// around for the crc without reading from the buffer twice
struct ReaderCrc<R: Read> {
    reader: R,
    pub cache: Vec<u8>,
}

impl<R: Read> ReaderCrc<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            cache: vec![],
        }
    }
}

impl<R: Read> Read for ReaderCrc<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.reader.read(buf);
        self.cache.extend_from_slice(buf);
        n
    }
}

#[derive(Debug, DekuRead)]
pub struct DekuStruct {
    pub a: u8,
    pub b: u8,
}

fn main() {
    let data = vec![0x01, 0x02];
    let input = Cursor::new(&data);
    let mut reader = ReaderCrc::new(input);
    let (_, s) = DekuStruct::from_reader((&mut reader, 0)).unwrap();
    assert_eq!(reader.cache, data);
    assert_eq!(s.a, 1);
    assert_eq!(s.b, 2);
}
