//! Reader for reader functions

use core::cmp::Ordering;

use bitvec::prelude::*;
use no_std_io::io::{ErrorKind, Read};

use crate::{prelude::NeedSize, DekuError};
use alloc::vec::Vec;

#[cfg(feature = "logging")]
use log;

/// Return from `read_bytes`
pub enum ReaderRet {
    /// Successfully read bytes
    Bytes,
    /// Successfully read bits
    Bits(Option<BitVec<u8, Msb0>>),
}

/// Reader to use with `from_reader_with_ctx`
pub struct Reader<'a, R: Read> {
    inner: &'a mut R,
    /// bits stored from previous reads that didn't read to the end of a byte size
    leftover: BitVec<u8, Msb0>,
    /// Amount of bits read during the use of [read_bits](Reader::read_bits) and [read_bytes](Reader::read_bytes).
    pub bits_read: usize,
}

/// Max bits requested from [`Reader::read_bits`] during one call
pub const MAX_BITS_AMT: usize = 128;

impl<'a, R: Read> Reader<'a, R> {
    /// Create a new `Reader`
    #[inline]
    pub fn new(inner: &'a mut R) -> Self {
        Self {
            inner,
            leftover: BitVec::new(), // with_capacity 8?
            bits_read: 0,
        }
    }

    /// Return the unused bits
    ///
    /// Once the parsing is complete for a struct, if the total size of the field using the `bits` attribute
    /// isn't byte aligned the returned values could be unexpected as the "Read" will always read
    /// to a full byte.
    ///
    /// ```rust
    /// use std::io::Cursor;
    /// use deku::prelude::*;
    ///
    /// #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    /// #[deku(endian = "big")]
    /// struct DekuTest {
    ///     #[deku(bits = "4")]
    ///     field_a: u8,
    ///     #[deku(bits = "2")]
    ///     field_b: u8,
    /// }
    /// //                       |         | <= this entire byte is Read
    /// let data: Vec<u8> = vec![0b0110_1101, 0xbe, 0xef];
    /// let mut cursor = Cursor::new(data);
    /// let mut reader = Reader::new(&mut cursor);
    /// let val = DekuTest::from_reader_with_ctx(&mut reader, ()).unwrap();
    /// assert_eq!(DekuTest {
    ///     field_a: 0b0110,
    ///     field_b: 0b11,
    /// }, val);
    ///
    /// // last 2 bits in that byte
    /// assert_eq!(reader.rest(), vec![false, true]);
    /// ```
    #[inline]
    pub fn rest(&mut self) -> Vec<bool> {
        self.leftover.iter().by_vals().collect()
    }

    /// Return true if we are at the end of a reader and there are no cached bits in the reader
    ///
    /// The byte that was read will be internally buffered
    #[inline]
    pub fn end(&mut self) -> bool {
        if !self.leftover.is_empty() {
            #[cfg(feature = "logging")]
            log::trace!("not end");
            false
        } else {
            let mut buf = [0; 1];
            if let Err(e) = self.inner.read_exact(&mut buf) {
                if e.kind() == ErrorKind::UnexpectedEof {
                    #[cfg(feature = "logging")]
                    log::trace!("end");
                    return true;
                }
            }

            // logic is best if we just turn this into bits right now
            self.leftover = BitVec::try_from_slice(&buf).unwrap();
            #[cfg(feature = "logging")]
            log::trace!("not end");
            false
        }
    }

    /// Used at the beginning of `from_reader`.
    /// TODO: maybe send into read_bytes() if amt >= 8
    #[inline]
    pub fn skip_bits(&mut self, amt: usize) -> Result<(), DekuError> {
        #[cfg(feature = "logging")]
        log::trace!("skip_bits: {amt}");
        // Save, and keep the leftover bits since the read will most likely be less than a byte
        self.read_bits(amt)?;

        Ok(())
    }

    /// Attempt to read bits from `Reader`. If enough bits are already "Read", we just grab
    /// enough bits to satisfy `amt`, but will also "Read" more from the stream and store the
    /// leftovers if enough are not already "Read".
    ///
    /// # Guarantees
    /// - if Some(bits), the returned `BitVec` will have the size of `amt` and
    /// `self.bits_read` will increase by `amt`
    ///
    /// # Params
    /// `amt`    - Amount of bits that will be read. Must be <= [`MAX_BITS_AMT`].
    #[inline]
    pub fn read_bits(&mut self, amt: usize) -> Result<Option<BitVec<u8, Msb0>>, DekuError> {
        #[cfg(feature = "logging")]
        log::trace!("read_bits: requesting {amt} bits");
        if amt == 0 {
            #[cfg(feature = "logging")]
            log::trace!("read_bits: returned None");
            return Ok(None);
        }
        let mut ret = BitVec::new();

        match amt.cmp(&self.leftover.len()) {
            // exact match, just use leftover
            Ordering::Equal => {
                core::mem::swap(&mut ret, &mut self.leftover);
                self.leftover.clear();
            }
            // previous read was not enough to satisfy the amt requirement, return all previously
            Ordering::Greater => {
                // read bits
                ret.extend_from_bitslice(&self.leftover);

                // calculate the amount of bytes we need to read to read enough bits
                let bits_left = amt - self.leftover.len();
                let mut bytes_len = bits_left / 8;
                if (bits_left % 8) != 0 {
                    bytes_len += 1;
                }

                // read in new bytes
                let mut buf = [0; MAX_BITS_AMT];
                if let Err(e) = self.inner.read_exact(&mut buf[..bytes_len]) {
                    if e.kind() == ErrorKind::UnexpectedEof {
                        return Err(DekuError::Incomplete(NeedSize::new(amt)));
                    }

                    // TODO: other errors?
                }
                let read_buf = &buf[..bytes_len];

                #[cfg(feature = "logging")]
                log::trace!("read_bits: read() {:02x?}", read_buf);

                // create bitslice and remove unused bits
                let rest = BitSlice::try_from_slice(read_buf).unwrap();
                let (rest, not_needed) = rest.split_at(bits_left);
                core::mem::swap(&mut not_needed.to_bitvec(), &mut self.leftover);

                // create return
                ret.extend_from_bitslice(rest);
            }
            // The entire bits we need to return have been already read previously from bytes but
            // not all were read, return required leftover bits
            Ordering::Less => {
                let used = self.leftover.split_off(amt);
                ret.extend_from_bitslice(&self.leftover);
                self.leftover = used;
            }
        }

        self.bits_read += ret.len();
        #[cfg(feature = "logging")]
        log::trace!("read_bits: returning {ret}");
        Ok(Some(ret))
    }

    /// Attempt to read bytes from `Reader`. This will return `ReaderRet::Bytes` with a valid
    /// `buf` of bytes if we have no "leftover" bytes and thus are byte aligned. If we are not byte
    /// aligned, this will call `read_bits` and return `ReaderRet::Bits(_)` of size `amt` * 8.
    ///
    /// # Params
    /// `amt`    - Amount of bytes that will be read
    #[inline]
    pub fn read_bytes(&mut self, amt: usize, buf: &mut [u8]) -> Result<ReaderRet, DekuError> {
        #[cfg(feature = "logging")]
        log::trace!("read_bytes: requesting {amt} bytes");
        if self.leftover.is_empty() {
            if buf.len() < amt {
                return Err(DekuError::Incomplete(NeedSize::new(amt * 8)));
            }
            if let Err(e) = self.inner.read_exact(&mut buf[..amt]) {
                if e.kind() == ErrorKind::UnexpectedEof {
                    return Err(DekuError::Incomplete(NeedSize::new(amt * 8)));
                }

                // TODO: other errors?
            }

            self.bits_read += amt * 8;

            #[cfg(feature = "logging")]
            log::trace!("read_bytes: returning {buf:02x?}");

            Ok(ReaderRet::Bytes)
        } else {
            Ok(ReaderRet::Bits(self.read_bits(amt * 8)?))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hexlit::hex;
    use no_std_io::io::Cursor;

    #[test]
    fn test_end() {
        let input = hex!("aa");
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        assert!(!reader.end());
        let mut buf = [0; 1];
        let _ = reader.read_bytes(1, &mut buf);
        assert!(reader.end());

        let input = hex!("aa");
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        assert!(!reader.end());
        let _ = reader.read_bits(4);
        assert!(!reader.end());
        let _ = reader.read_bits(4);
        assert!(reader.end());
    }

    #[test]
    fn test_bits_less() {
        let input = hex!("aa");
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        let _ = reader.read_bits(1);
        let _ = reader.read_bits(4);
        let _ = reader.read_bits(3);
    }

    #[test]
    fn test_inner() {
        let input = hex!("aabbcc");
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        let mut buf = [0; 1];
        let _ = reader.read_bytes(1, &mut buf);
        assert_eq!([0xaa], buf);
    }
}
