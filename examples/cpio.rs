use deku::prelude::*;
use std::io::{Read, Seek, SeekFrom, Write};

pub trait ReadSeek: Read + Seek {}
// pub trait BufReadSeek: BufRead + Seek + Send {}
impl<T: Read + Seek> ReadSeek for T {}

pub enum Data {
    /// On read: Save current stream_position() as `Offset`, seek `header.filesize`
    /// This will be used to seek this this position if we want to extract *just* this file
    Offset(u64),
    /// On write: Write `Reader` to write buffer
    Reader(Box<dyn ReadSeek>),
}

/// pad out to a multiple of 4 bytes
fn pad_to_4(len: usize) -> usize {
    match len % 4 {
        0 => 0,
        x => 4 - x,
    }
}

impl DekuReader<'_, u32> for Data {
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut Reader<R>,
        filesize: u32,
    ) -> Result<Data, DekuError> {
        let reader = reader.as_mut();

        // Save the current offset, this is where the file exists for reading later
        let current_pos = reader.stream_position().unwrap();

        // Seek past that file
        let position = filesize as i64 + pad_to_4(filesize as usize) as i64;
        let _ = reader.seek(SeekFrom::Current(position));

        Ok(Self::Offset(current_pos))
    }
}

impl DekuWriterMut for Data {
    fn to_writer_mut<W: Write + Seek>(
        &mut self,
        writer: &mut Writer<W>,
        _: (),
    ) -> Result<(), DekuError> {
        if let Self::Reader(reader) = self {
            // read from reader
            let mut data = vec![];
            reader.read_to_end(&mut data).unwrap();

            // write to deku
            data.to_writer(writer, ())?;

            // add padding
            for _ in 0..pad_to_4(data.len()) {
                0_u8.to_writer(writer, ())?;
            }
        } else {
            panic!("ah");
        }

        Ok(())
    }
}

#[derive(DekuWriteMut)]
pub struct Object {
    // other fields
    data: Data,
}

fn main() {}
