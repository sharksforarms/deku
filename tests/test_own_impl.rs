#[test]
fn test_own_impl() {
    use deku::prelude::*;
    use std::io::{Read, Seek, SeekFrom};
    pub enum Data {
        /// On read: Save current stream_position() as `Offset`, seek `header.filesize`
        /// This will be used to seek this this position if we want to extract *just* this file
        #[expect(dead_code)]
        Offset(u64),
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
            let position = filesize as i64;
            let _ = reader.seek(SeekFrom::Current(position));

            Ok(Self::Offset(current_pos))
        }
    }
}
