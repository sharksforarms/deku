//! Wrapper type that provides a fake [`Seek`] implementation.

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

use crate::no_std_io::{Read, Result, Seek, SeekFrom, Write};
use no_std_io::io::ErrorKind;

/// A wrapper that provides a limited implementation of
/// [`Seek`] for unseekable [`Read`] and [`Write`] streams.
///
/// This is useful when reading or writing from unseekable streams where deku
/// does not *actually* need to seek to successfully parse or write the data.
///
/// This implementation was taken from [binrw](https://docs.rs/binrw/0.14.0/binrw/io/struct.NoSeek.html)
pub struct NoSeek<T> {
    /// The original stream.
    inner: T,
    /// The virtual position of the seekable stream.
    pos: u64,
}

impl<T> NoSeek<T> {
    /// Creates a new seekable wrapper for the given value.
    pub fn new(inner: T) -> Self {
        NoSeek { inner, pos: 0 }
    }

    /// Gets a mutable reference to the underlying value.
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Gets a reference to the underlying value.
    pub fn get_ref(&self) -> &T {
        &self.inner
    }

    /// Consumes this wrapper, returning the underlying value.
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T> Seek for NoSeek<T> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        match pos {
            SeekFrom::Start(n) if self.pos == n => Ok(n),
            SeekFrom::Current(0) => Ok(self.pos),
            // https://github.com/rust-lang/rust/issues/86442
            #[cfg(feature = "std")]
            _ => Err(std::io::Error::new(
                ErrorKind::Other,
                "seek on unseekable file",
            )),
            #[cfg(not(feature = "std"))]
            _ => panic!("seek on unseekable file"),
        }
    }

    #[cfg(feature = "std")]
    fn stream_position(&mut self) -> Result<u64> {
        Ok(self.pos)
    }
}

impl<T: Read> Read for NoSeek<T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let n = self.inner.read(buf)?;
        self.pos += n as u64;
        Ok(n)
    }

    #[cfg(feature = "std")]
    fn read_vectored(&mut self, bufs: &mut [std::io::IoSliceMut<'_>]) -> Result<usize> {
        let n = self.inner.read_vectored(bufs)?;
        self.pos += n as u64;
        Ok(n)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        let n = self.inner.read_to_end(buf)?;
        self.pos += n as u64;
        Ok(n)
    }

    #[cfg(feature = "std")]
    fn read_to_string(&mut self, buf: &mut String) -> Result<usize> {
        let n = self.inner.read_to_string(buf)?;
        self.pos += n as u64;
        Ok(n)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        self.inner.read_exact(buf)?;
        self.pos += buf.len() as u64;
        Ok(())
    }
}

impl<T: Write> Write for NoSeek<T> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let n = self.inner.write(buf)?;
        self.pos += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }

    #[cfg(feature = "std")]
    fn write_vectored(&mut self, bufs: &[std::io::IoSlice<'_>]) -> Result<usize> {
        let n = self.inner.write_vectored(bufs)?;
        self.pos += n as u64;
        Ok(n)
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.inner.write_all(buf)?;
        self.pos += buf.len() as u64;
        Ok(())
    }
}
