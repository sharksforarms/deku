//! Reader for reader functions

use core::cmp::Ordering;

#[cfg(feature = "bits")]
use bitvec::prelude::*;
use no_std_io::io::{ErrorKind, Read, Seek, SeekFrom};

use crate::{prelude::NeedSize, DekuError};
use alloc::vec::Vec;

#[cfg(feature = "logging")]
use log;

/// Return from `read_bytes`
pub enum ReaderRet {
    /// Successfully read bytes
    Bytes,
    /// Successfully read bits
    #[cfg(feature = "bits")]
    Bits(Option<BitVec<u8, Msb0>>),
}

/// Max bits requested from [`Reader::read_bits`] during one call
pub const MAX_BITS_AMT: usize = 128;

enum Leftover {
    Byte(u8),
    #[cfg(feature = "bits")]
    Bits(BitVec<u8, Msb0>),
}

/// Reader to use with `from_reader_with_ctx`
pub struct Reader<'a, R: Read + Seek> {
    inner: &'a mut R,
    /// bits stored from previous reads that didn't read to the end of a byte size
    leftover: Option<Leftover>,
    /// Amount of bits read after last read, reseted before reading enum ids
    pub last_bits_read_amt: usize,
    /// Amount of bits read during the use of [read_bits](Reader::read_bits) and [read_bytes](Reader::read_bytes)
    pub bits_read: usize,
}

impl<R: Read + Seek> Seek for Reader<'_, R> {
    #[inline]
    fn seek(&mut self, pos: SeekFrom) -> no_std_io::io::Result<u64> {
        #[cfg(feature = "logging")]
        log::trace!("seek: {pos:?}");

        // clear leftover
        self.leftover = None;
        self.inner.seek(pos)
    }
}

impl<R: Read + Seek> AsMut<R> for Reader<'_, R> {
    #[inline]
    fn as_mut(&mut self) -> &mut R {
        self.inner
    }
}

impl<'a, R: Read + Seek> Reader<'a, R> {
    /// Create a new `Reader`
    #[inline]
    pub fn new(inner: &'a mut R) -> Self {
        Self {
            inner,
            leftover: None,
            last_bits_read_amt: 0,
            bits_read: 0,
        }
    }

    /// Seek to previous previous before the last read, used for `id_pat`
    #[inline]
    pub fn seek_last_read(&mut self) -> no_std_io::io::Result<()> {
        let number = self.last_bits_read_amt as i64;
        let seek_amt = (number / 8).saturating_add((number % 8).signum());
        self.seek(SeekFrom::Current(seek_amt.saturating_neg()))?;
        self.bits_read -= self.last_bits_read_amt;
        self.leftover = None;

        Ok(())
    }

    /// Consume self, returning inner Reader
    #[inline]
    pub fn into_inner(self) -> &'a mut R {
        self.inner
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
    ///     #[deku(bits = 4)]
    ///     field_a: u8,
    ///     #[deku(bits = 2)]
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
        #[cfg(feature = "bits")]
        match &self.leftover {
            Some(Leftover::Bits(bits)) => bits.iter().by_vals().collect(),
            Some(Leftover::Byte(byte)) => {
                let bytes: &[u8] = &[*byte];
                let bits: BitVec<u8, Msb0> = BitVec::try_from_slice(bytes).unwrap();
                bits.iter().by_vals().collect()
            }
            None => alloc::vec![],
        }
        #[cfg(not(feature = "bits"))]
        alloc::vec![]
    }

    /// Return true if we are at the end of a reader and there are no cached bits in the reader.
    /// Since this uses [Read] internally, this will return true when [Read] returns [ErrorKind::UnexpectedEof].
    ///
    /// The byte that was read will be internally buffered
    #[inline]
    pub fn end(&mut self) -> bool {
        if self.leftover.is_some() {
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

            #[cfg(feature = "logging")]
            log::trace!("not end: read {:02x?}", &buf);

            self.bits_read += 8;
            self.leftover = Some(Leftover::Byte(buf[0]));
            false
        }
    }

    /// Used at the beginning of `from_reader`.
    // TODO: maybe send into read_bytes() if amt >= 8
    #[inline]
    pub fn skip_bits(&mut self, amt: usize) -> Result<(), DekuError> {
        #[cfg(feature = "bits")]
        {
            #[cfg(feature = "logging")]
            log::trace!("skip_bits: {amt}");
            // Save, and keep the leftover bits since the read will most likely be less than a byte
            self.read_bits(amt)?;
        }

        #[cfg(not(feature = "bits"))]
        {
            if amt > 0 {
                panic!("deku features no-bits was used");
            }
        }

        Ok(())
    }

    /// Attempt to read bits from `Reader`. If enough bits are already "Read", we just grab
    /// enough bits to satisfy `amt`, but will also "Read" more from the stream and store the
    /// leftovers if enough are not already "Read".
    ///
    /// # Guarantees
    /// - if Some(bits), the returned `BitVec` will have the size of `amt` and
    ///   `self.bits_read` will increase by `amt`
    ///
    /// # Params
    /// `amt`    - Amount of bits that will be read. Must be <= [`MAX_BITS_AMT`].
    #[inline(never)]
    #[cfg(feature = "bits")]
    pub fn read_bits(&mut self, amt: usize) -> Result<Option<BitVec<u8, Msb0>>, DekuError> {
        #[cfg(feature = "logging")]
        log::trace!("read_bits: requesting {amt} bits");
        if amt == 0 {
            #[cfg(feature = "logging")]
            log::trace!("read_bits: returned None");
            return Ok(None);
        }
        let mut ret = BitVec::new();

        // if Leftover::Bytes exists, convert into Bits
        if let Some(Leftover::Byte(byte)) = self.leftover {
            let bytes: &[u8] = &[byte];
            let bits: BitVec<u8, Msb0> = BitVec::try_from_slice(bytes).unwrap();
            self.leftover = Some(Leftover::Bits(bits));
        }

        let previous_len = match &self.leftover {
            Some(Leftover::Bits(bits)) => bits.len(),
            None => 0,
            Some(Leftover::Byte(_)) => unreachable!(),
        };

        match amt.cmp(&previous_len) {
            // exact match, just use leftover
            Ordering::Equal => {
                if let Some(Leftover::Bits(bits)) = &mut self.leftover {
                    core::mem::swap(&mut ret, bits);
                    self.leftover = None;
                } else {
                    unreachable!();
                }
            }
            // previous read was not enough to satisfy the amt requirement, return all previously
            Ordering::Greater => {
                // read bits
                match self.leftover {
                    Some(Leftover::Bits(ref bits)) => {
                        ret.extend_from_bitslice(bits);
                    }
                    Some(Leftover::Byte(_)) => {
                        unreachable!();
                    }
                    None => {}
                }

                // calculate the amount of bytes we need to read to read enough bits
                let bits_left = amt - ret.len();
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
                    return Err(DekuError::Io(e.kind()));
                }
                let read_buf = &buf[..bytes_len];

                #[cfg(feature = "logging")]
                log::trace!("read_bits: read() {:02x?}", read_buf);

                // create bitslice and remove unused bits
                let rest = BitSlice::try_from_slice(read_buf).unwrap();
                let (rest, not_needed) = rest.split_at(bits_left);
                self.leftover = Some(Leftover::Bits(not_needed.to_bitvec()));

                // create return
                ret.extend_from_bitslice(rest);
            }
            // The entire bits we need to return have been already read previously from bytes but
            // not all were read, return required leftover bits
            Ordering::Less => {
                // read bits
                if let Some(Leftover::Bits(bits)) = &mut self.leftover {
                    let used = bits.split_off(amt);
                    ret.extend_from_bitslice(bits);
                    self.leftover = Some(Leftover::Bits(used));
                } else {
                    unreachable!();
                }
            }
        }

        let bits_read = ret.len();
        self.last_bits_read_amt += bits_read;
        self.bits_read += bits_read;

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
    /// `buf`    - result bytes
    #[inline(always)]
    pub fn read_bytes(&mut self, amt: usize, buf: &mut [u8]) -> Result<ReaderRet, DekuError> {
        #[cfg(feature = "logging")]
        log::trace!("read_bytes: requesting {amt} bytes");

        if self.leftover.is_none() {
            if let Err(e) = self.inner.read_exact(&mut buf[..amt]) {
                if e.kind() == ErrorKind::UnexpectedEof {
                    return Err(DekuError::Incomplete(NeedSize::new(amt * 8)));
                }
                return Err(DekuError::Io(e.kind()));
            }

            let bits_read = amt * 8;
            self.last_bits_read_amt += bits_read;
            self.bits_read += bits_read;

            #[cfg(feature = "logging")]
            log::trace!("read_bytes: returning {:02x?}", &buf[..amt]);

            return Ok(ReaderRet::Bytes);
        }

        // Trying to keep this not in the hot path
        self.read_bytes_other(amt, buf)
    }

    fn read_bytes_other(&mut self, amt: usize, buf: &mut [u8]) -> Result<ReaderRet, DekuError> {
        match self.leftover {
            Some(Leftover::Byte(byte)) => self.read_bytes_leftover(buf, byte, amt),
            #[cfg(feature = "bits")]
            Some(Leftover::Bits(_)) => Ok(ReaderRet::Bits(self.read_bits(amt * 8)?)),
            _ => unreachable!(),
        }
    }

    fn read_bytes_leftover(
        &mut self,
        buf: &mut [u8],
        byte: u8,
        amt: usize,
    ) -> Result<ReaderRet, DekuError> {
        buf[0] = byte;

        #[cfg(feature = "logging")]
        log::trace!("read_bytes_leftover: using previous read {:02x?}", &buf[0]);

        self.leftover = None;
        let remaining = amt - 1;
        if remaining == 0 {
            #[cfg(feature = "logging")]
            log::trace!("read_bytes_const_leftover: returning {:02x?}", &buf);

            return Ok(ReaderRet::Bytes);
        }
        let buf_len = buf.len();
        if buf_len < remaining {
            return Err(DekuError::Incomplete(NeedSize::new(remaining * 8)));
        }
        if let Err(e) = self
            .inner
            .read_exact(&mut buf[amt - remaining..][..remaining])
        {
            if e.kind() == ErrorKind::UnexpectedEof {
                return Err(DekuError::Incomplete(NeedSize::new(remaining * 8)));
            }
            return Err(DekuError::Io(e.kind()));
        }
        self.bits_read += remaining * 8;

        #[cfg(feature = "logging")]
        log::trace!("read_bytes_leftover: returning {:02x?}", &buf);

        Ok(ReaderRet::Bytes)
    }

    /// Attempt to read bytes from `Reader`. This will return `ReaderRet::Bytes` with a valid
    /// `buf` of bytes if we have no "leftover" bytes and thus are byte aligned. If we are not byte
    /// aligned, this will call `read_bits` and return `ReaderRet::Bits(_)` of size `N` * 8.
    ///
    /// # Params
    /// `buf`    - result bytes
    #[inline(always)]
    pub fn read_bytes_const<const N: usize>(
        &mut self,
        buf: &mut [u8; N],
    ) -> Result<ReaderRet, DekuError> {
        #[cfg(feature = "logging")]
        log::trace!("read_bytes_const: requesting {N} bytes");

        if self.leftover.is_none() {
            if let Err(e) = self.inner.read_exact(buf) {
                if e.kind() == ErrorKind::UnexpectedEof {
                    return Err(DekuError::Incomplete(NeedSize::new(N * 8)));
                }
                return Err(DekuError::Io(e.kind()));
            }

            self.last_bits_read_amt += N * 8;
            self.bits_read += N * 8;

            #[cfg(feature = "logging")]
            log::trace!("read_bytes_const: returning {:02x?}", &buf);

            return Ok(ReaderRet::Bytes);
        }

        // Trying to keep this not in the hot path
        self.read_bytes_const_other::<N>(buf)
    }

    fn read_bytes_const_other<const N: usize>(
        &mut self,
        buf: &mut [u8; N],
    ) -> Result<ReaderRet, DekuError> {
        match self.leftover {
            Some(Leftover::Byte(byte)) => self.read_bytes_const_leftover(buf, byte),
            #[cfg(feature = "bits")]
            Some(Leftover::Bits(_)) => Ok(ReaderRet::Bits(self.read_bits(N * 8)?)),
            _ => unreachable!(),
        }
    }

    fn read_bytes_const_leftover<const N: usize>(
        &mut self,
        buf: &mut [u8; N],
        byte: u8,
    ) -> Result<ReaderRet, DekuError> {
        buf[0] = byte;

        #[cfg(feature = "logging")]
        log::trace!(
            "read_bytes_const_leftover: using previous read {:02x?}",
            &buf[0]
        );

        self.leftover = None;
        let remaining = N - 1;
        if remaining == 0 {
            #[cfg(feature = "logging")]
            log::trace!("read_bytes_const_leftover: returning {:02x?}", &buf);

            return Ok(ReaderRet::Bytes);
        }
        let buf_len = buf.len();
        if buf_len < remaining {
            return Err(DekuError::Incomplete(NeedSize::new(remaining * 8)));
        }
        if let Err(e) = self
            .inner
            .read_exact(&mut buf[N - remaining..][..remaining])
        {
            if e.kind() == ErrorKind::UnexpectedEof {
                return Err(DekuError::Incomplete(NeedSize::new(remaining * 8)));
            }
            return Err(DekuError::Io(e.kind()));
        }
        self.bits_read += remaining * 8;

        #[cfg(feature = "logging")]
        log::trace!("read_bytes_const_leftover: returning {:02x?}", &buf);

        Ok(ReaderRet::Bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hexlit::hex;
    use no_std_io::io::Cursor;

    #[test]
    #[cfg(feature = "bits")]
    fn test_end() {
        let input = hex!("aabb");
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        assert!(!reader.end());
        let mut buf = [0; 2];
        let _ = reader.read_bytes_const::<2>(&mut buf).unwrap();
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
    #[cfg(feature = "bits")]
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

    #[test]
    fn test_seek_last_read_bytes() {
        // bytes
        let input = hex!("aa");
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        let mut buf = [0; 1];
        let _ = reader.read_bytes(1, &mut buf);
        assert_eq!([0xaa], buf);
        reader.seek_last_read().unwrap();
        let _ = reader.read_bytes(1, &mut buf);
        assert_eq!([0xaa], buf);

        // 2 bytes (and const)
        let input = hex!("aabb");
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        let mut buf = [0; 2];
        let _ = reader.read_bytes_const::<2>(&mut buf);
        assert_eq!([0xaa, 0xbb], buf);
        reader.seek_last_read().unwrap();
        let _ = reader.read_bytes_const::<2>(&mut buf);
        assert_eq!([0xaa, 0xbb], buf);
    }

    #[cfg(feature = "bits")]
    #[test]
    fn test_seek_last_read_bits() {
        let input = hex!("ab");
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        let bits = reader.read_bits(4).unwrap();
        assert_eq!(bits, Some(bitvec![u8, Msb0; 1, 0, 1, 0]));
        reader.seek_last_read().unwrap();
        let bits = reader.read_bits(4).unwrap();
        assert_eq!(bits, Some(bitvec![u8, Msb0; 1, 0, 1, 0]));

        // more than byte
        let input = hex!("abd0");
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        let bits = reader.read_bits(9).unwrap();
        assert_eq!(bits, Some(bitvec![u8, Msb0; 1, 0, 1, 0, 1, 0, 1, 1, 1]));
        reader.seek_last_read().unwrap();
        let bits = reader.read_bits(9).unwrap();
        assert_eq!(bits, Some(bitvec![u8, Msb0; 1, 0, 1, 0, 1, 0, 1, 1, 1]));
    }
}
