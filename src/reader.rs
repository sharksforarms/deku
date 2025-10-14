//! Reader for reader functions

#[cfg(feature = "bits")]
use bitvec::prelude::*;
use no_std_io::io::{ErrorKind, Read, Seek, SeekFrom};

use crate::{ctx::Order, prelude::NeedSize, DekuError};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;
#[cfg(feature = "bits")]
use core::cmp::Ordering;

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

/// Bits or Byte stored from previous read, such as the case of id_pat
#[derive(Debug, Clone)]
pub enum Leftover {
    /// byte value
    Byte(u8),
    /// bit values
    #[cfg(feature = "bits")]
    Bits(crate::BoundedBitVec<[u8; 1], Msb0>),
}

/// Reader to use with `from_reader_with_ctx`
pub struct Reader<R: Read + Seek> {
    inner: R,
    /// bits stored from previous reads that didn't read to the end of a byte size
    pub leftover: Option<Leftover>,
    /// Amount of bits read during the use of [read_bits](Reader::read_bits) and [read_bytes](Reader::read_bytes)
    pub bits_read: usize,
}

impl<R: Read + Seek> Seek for Reader<R> {
    #[inline]
    fn seek(&mut self, pos: SeekFrom) -> no_std_io::io::Result<u64> {
        #[cfg(feature = "logging")]
        log::trace!("seek: {pos:?}");

        // clear leftover
        self.leftover = None;
        // set bits read
        match pos {
            // When reading from the start, reset the bits_read so from_bytes
            // return can still be reasonable
            SeekFrom::Start(n) => {
                if n > 0 {
                    self.bits_read = (n * 8) as usize;
                }
            }
            SeekFrom::End(_) => (),
            // If seeking from current, act as if we just read those bytes
            SeekFrom::Current(n) => {
                if n > 0 {
                    self.bits_read += (n * 8) as usize;
                }
            }
        }
        self.inner.seek(pos)
    }
}

impl<R: Read + Seek> AsMut<R> for Reader<R> {
    #[inline]
    fn as_mut(&mut self) -> &mut R {
        &mut self.inner
    }
}

impl<R: Read + Seek> Reader<R> {
    /// Create a new `Reader`
    #[inline]
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            leftover: None,
            bits_read: 0,
        }
    }

    /// Consume self, returning inner Reader
    #[inline]
    pub fn into_inner(self) -> R {
        self.inner
    }

    /// Return the unused bits
    ///
    /// Once the parsing is complete for a struct, if the total size of the field using the `bits` attribute
    /// isn't byte aligned the returned values could be unexpected as the "Read" will always read
    /// to a full byte.
    ///
    /// ```rust
    /// use deku::prelude::*;
    ///
    /// # #[cfg(feature = "std")]
    /// use std::io::Cursor;
    ///
    /// # #[cfg(feature = "bits")]
    /// #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    /// #[deku(endian = "big")]
    /// struct DekuTest {
    ///     #[deku(bits = 4)]
    ///     field_a: u8,
    ///     #[deku(bits = 2)]
    ///     field_b: u8,
    /// }
    ///
    /// # #[cfg(feature = "bits")]
    /// # fn main() {
    /// //                       |         | <= this entire byte is Read
    /// let data: Vec<u8> = vec![0b0110_1101, 0xbe, 0xef];
    /// let mut cursor = no_std_io::io::Cursor::new(data);
    /// let mut reader = Reader::new(&mut cursor);
    /// let val = DekuTest::from_reader_with_ctx(&mut reader, ()).unwrap();
    /// assert_eq!(DekuTest {
    ///     field_a: 0b0110,
    ///     field_b: 0b11,
    /// }, val);
    ///
    /// // last 2 bits in that byte
    /// assert_eq!(reader.rest(), vec![false, true]);
    /// # }
    ///
    /// # #[cfg(not(feature = "bits"))]
    /// # fn main() {}
    /// ```
    #[inline]
    #[cfg(feature = "alloc")]
    pub fn rest(&mut self) -> Vec<bool> {
        #[cfg(feature = "bits")]
        match &self.leftover {
            Some(Leftover::Bits(bits)) => {
                debug_assert!(bits.len() <= 8);
                bits.as_bitslice().iter().by_vals().collect()
            }
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
    /// The byte that was read will be internally buffered and will *not* be included in the `bits_read` count.
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

            self.leftover = Some(Leftover::Byte(buf[0]));
            false
        }
    }

    /// Used at the beginning of `from_reader`.
    ///
    /// This will increment `bits_read`.
    // TODO: maybe send into read_bytes() if amt >= 8
    #[inline]
    pub fn skip_bits(&mut self, amt: usize, _order: Order) -> Result<(), DekuError> {
        #[cfg(feature = "bits")]
        {
            #[cfg(feature = "logging")]
            log::trace!("skip_bits: {amt}");

            let bytes_amt = amt / 8;
            let mut bits_amt = amt % 8;

            if let Some(Leftover::Bits(bits)) = &self.leftover {
                let mut buf = bitarr!(u8, Msb0; 0; 8);
                let needed = core::cmp::min(bits_amt, bits.len());
                bits_amt -= needed;
                self.read_bits_into(&mut buf[..needed], _order)?;
            }

            // first, seek with bytes
            if bytes_amt != 0 {
                self.seek(SeekFrom::Current(
                    i64::try_from(bytes_amt).expect("could not convert seek usize into i64"),
                ))
                .map_err(|e| DekuError::Io(e.kind()))?;
            }

            // Save, and keep the leftover bits since the read will most likely be less than a byte
            // Note that the leftover bits are kept in self.leftover
            let mut buf = bitarr!(u8, Msb0; 0; 8);
            self.read_bits_into(&mut buf[..bits_amt], _order)?;
        }

        #[cfg(not(feature = "bits"))]
        {
            if amt > 0 {
                panic!("requires deku feature: bits");
            }
        }
        Ok(())
    }

    /// Attempt to read bits from `Reader`. If enough bits are already "Read",
    /// we just grab enough bits to satisfy `dst.len()`, but will also "Read"
    /// more from the stream and store the leftovers if enough are not already
    /// "Read".
    ///
    /// # Guarantees
    /// - if Some(bits), `dst` will be filled and `self.bits_read` will increase
    ///   by `dst.len()`.
    /// - Implementation will not allocate on the heap
    ///
    /// # Params
    /// `order` - The order by which to interpret the read bits
    /// `dst` - The slice used as the destination for the read bits
    #[inline(never)]
    #[cfg(feature = "bits")]
    pub fn read_bits_into(
        &mut self,
        dst: &mut BitSlice<u8, Msb0>,
        order: Order,
    ) -> Result<(), DekuError> {
        #[cfg(feature = "logging")]
        log::trace!("read_bits_into: {order:?}, {:?}", dst.len());

        if dst.is_empty() {
            return Ok(());
        }

        let mut leftover = None;
        core::mem::swap(&mut leftover, &mut self.leftover);

        if let Some(Leftover::Byte(byte)) = leftover {
            leftover = Some(Leftover::Bits(BitArray::from([byte]).into()));
        }

        let previous_len = if let Some(Leftover::Bits(bits)) = &leftover {
            bits.len()
        } else {
            0
        };

        match dst.len().cmp(&previous_len) {
            Ordering::Less => {
                let Some(Leftover::Bits(mut bits)) = leftover else {
                    unreachable!();
                };
                debug_assert!(bits.len() <= 8);
                match order {
                    Order::Lsb0 => {
                        let used = bits.split_off(bits.len() - dst.len());
                        dst.copy_from_bitslice(used.as_bitslice());
                        self.leftover = Some(Leftover::Bits(bits));
                    }
                    Order::Msb0 => {
                        let used = bits.split_off(dst.len());
                        dst.copy_from_bitslice(bits.as_bitslice());
                        self.leftover = Some(Leftover::Bits(used));
                    }
                }
            }
            Ordering::Equal => {
                let Some(Leftover::Bits(bits)) = &mut leftover else {
                    unreachable!();
                };
                debug_assert!(bits.len() <= 8);
                let mut bbv: crate::BoundedBitVec<[u8; 1], Msb0> = crate::BoundedBitVec::new();
                core::mem::swap(&mut bbv, bits);
                let (consumed, _dst) = dst.split_at_mut(bbv.len());
                let end = bbv.len();
                // Make sure stores for `consumed` and `bbv` are typed as T::Alias
                consumed.copy_from_bitslice(bbv.as_mut_bitslice().split_at_mut(end).0);
            }
            Ordering::Greater => {
                let (start, end) = if order == Order::Lsb0 {
                    let need = dst.len() - previous_len;
                    let start = 8 - ((need.div_ceil(8) * 8) - need);
                    (start, need)
                } else if let Some(Leftover::Bits(bits)) = &leftover {
                    debug_assert_eq!(order, Order::Msb0);
                    let end = bits.len();
                    dst[..end].copy_from_bitslice(bits.as_bitslice().split_at(end).0);
                    (end, dst.len())
                } else {
                    (0, dst.len())
                };

                // read in new bytes
                // TODO: Profile and optimise
                let remainder = if order == Order::Lsb0 {
                    if dst.len() % 8 != 0 {
                        let mut iter = dst[..end].rchunks_exact_mut(8);
                        for slot in iter.by_ref() {
                            let mut buf: [u8; 1] = [0u8];
                            if let Err(e) = self.inner.read_exact(&mut buf) {
                                if e.kind() == ErrorKind::UnexpectedEof {
                                    return Err(DekuError::Incomplete(NeedSize::new(dst.len())));
                                }
                            }
                            slot.store_be(buf[0]);
                        }
                        iter.into_remainder()
                    } else {
                        let mut iter = dst[..end].chunks_exact_mut(8);
                        for slot in iter.by_ref() {
                            let mut buf: [u8; 1] = [0u8];
                            if let Err(e) = self.inner.read_exact(&mut buf) {
                                if e.kind() == ErrorKind::UnexpectedEof {
                                    return Err(DekuError::Incomplete(NeedSize::new(dst.len())));
                                }
                            }
                            slot.store_be(buf[0]);
                        }
                        iter.into_remainder()
                    }
                } else {
                    debug_assert_eq!(order, Order::Msb0);
                    let mut iter = dst[start..end].chunks_exact_mut(8);
                    for slot in iter.by_ref() {
                        let mut buf: [u8; 1] = [0u8];
                        if let Err(e) = self.inner.read_exact(&mut buf) {
                            if e.kind() == ErrorKind::UnexpectedEof {
                                return Err(DekuError::Incomplete(NeedSize::new(dst.len())));
                            }
                        }
                        slot.store_be(buf[0]);
                    }
                    iter.into_remainder()
                };

                if order == Order::Lsb0 {
                    if !remainder.is_empty() {
                        let mut buf: [u8; 1] = [0u8];
                        if let Err(e) = self.inner.read_exact(&mut buf) {
                            if e.kind() == ErrorKind::UnexpectedEof {
                                return Err(DekuError::Incomplete(NeedSize::new(dst.len())));
                            }
                            return Err(DekuError::Io(e.kind()));
                        }
                        let slice: &mut BitSlice<u8, Msb0> =
                            BitSlice::try_from_slice_mut(buf.as_mut_slice()).unwrap();
                        let (rest, used) = slice.split_at_mut(8 - remainder.len());
                        let len = used.len();
                        remainder.copy_from_bitslice(used.split_at_mut(len).0);
                        self.leftover = Some(Leftover::Bits(rest.into()));
                    }
                    if let Some(Leftover::Bits(bits)) = leftover {
                        dst[end..].copy_from_bitslice(bits.as_bitslice());
                    }
                } else if !remainder.is_empty() {
                    debug_assert_eq!(Order::Msb0, order);
                    let mut buf: [u8; 1] = [0u8];
                    if let Err(e) = self.inner.read_exact(&mut buf) {
                        if e.kind() == ErrorKind::UnexpectedEof {
                            return Err(DekuError::Incomplete(NeedSize::new(dst.len())));
                        }
                        return Err(DekuError::Io(e.kind()));
                    }

                    // mut horror-show due to bitvec generic/safety shenanigans
                    let slice: &mut BitSlice<u8, Msb0> =
                        BitSlice::try_from_slice_mut(buf.as_mut_slice()).unwrap();
                    let (used, rest) = slice.split_at_mut(remainder.len());
                    let end = used.len();
                    remainder.copy_from_bitslice(used.split_at_mut(end).0);
                    self.leftover = Some(Leftover::Bits(rest.into()));
                }
            }
        }

        self.bits_read += dst.len();
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
    /// `amt`    - Amount of bits that will be read
    #[inline(never)]
    #[cfg(feature = "bits")]
    pub fn read_bits(
        &mut self,
        amt: usize,
        order: Order,
    ) -> Result<Option<BitVec<u8, Msb0>>, DekuError> {
        let mut vec = BitVec::repeat(false, amt);
        self.read_bits_into(vec.as_mut_bitslice(), order)?;
        Ok(Some(vec))
    }

    /// Attempt to read bytes from `Reader`. This will return `ReaderRet::Bytes` with a valid
    /// `buf` of bytes if we have no "leftover" bytes and thus are byte aligned. If we are not byte
    /// aligned, this will call `read_bits` and return `ReaderRet::Bits(_)` of size `amt` * 8.
    ///
    /// # Params
    /// `amt`    - Amount of bytes that will be read
    /// `buf`    - result bytes
    #[inline(always)]
    pub fn read_bytes(
        &mut self,
        amt: usize,
        buf: &mut [u8],
        order: Order,
    ) -> Result<ReaderRet, DekuError> {
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
            self.bits_read += bits_read;

            #[cfg(feature = "logging")]
            log::trace!("read_bytes: returning {:02x?}", &buf[..amt]);

            return Ok(ReaderRet::Bytes);
        }

        // Trying to keep this not in the hot path
        self.read_bytes_other(amt, buf, order)
    }

    fn read_bytes_other(
        &mut self,
        amt: usize,
        buf: &mut [u8],
        _order: Order,
    ) -> Result<ReaderRet, DekuError> {
        match self.leftover {
            Some(Leftover::Byte(byte)) => self.read_bytes_leftover(buf, byte, amt),
            #[cfg(feature = "bits")]
            Some(Leftover::Bits(_)) => {
                let slice = BitSlice::from_slice_mut(&mut buf[..amt]);
                self.read_bits_into(slice, _order)?;
                Ok(ReaderRet::Bytes)
            }
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

            self.bits_read += amt * 8;
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
        self.bits_read += amt * 8;

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
        order: Order,
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

            self.bits_read += N * 8;

            #[cfg(feature = "logging")]
            log::trace!("read_bytes_const: returning {:02x?}", &buf);

            return Ok(ReaderRet::Bytes);
        }

        // Trying to keep this not in the hot path
        self.read_bytes_const_other::<N>(buf, order)
    }

    fn read_bytes_const_other<const N: usize>(
        &mut self,
        buf: &mut [u8; N],
        _order: Order,
    ) -> Result<ReaderRet, DekuError> {
        match self.leftover {
            Some(Leftover::Byte(byte)) => {
                self.read_bytes_const_leftover(buf, byte)?;
                Ok(ReaderRet::Bytes)
            }
            #[cfg(feature = "bits")]
            Some(Leftover::Bits(_)) => {
                let slice = BitSlice::from_slice_mut(buf);
                self.read_bits_into(slice, _order)?;
                Ok(ReaderRet::Bytes)
            }
            _ => unreachable!(),
        }
    }

    /// Attempt to read bytes from `Reader` into `buf`, taking care of the case
    /// where we're not byte-aligned with respect to the data source.
    ///
    /// # Guarantees
    /// - Implementation will not allocate on the heap
    ///
    /// # Params
    /// `buf` - result bytes
    pub fn read_bytes_const_into<const N: usize>(
        &mut self,
        buf: &mut [u8; N],
        _order: Order,
    ) -> Result<(), DekuError> {
        if self.leftover.is_none() {
            if let Err(e) = self.inner.read_exact(buf) {
                if e.kind() == ErrorKind::UnexpectedEof {
                    return Err(DekuError::Incomplete(NeedSize::new(N * 8)));
                }
                return Err(DekuError::Io(e.kind()));
            }
            self.bits_read += N * 8;

            return Ok(());
        }

        match self.leftover {
            Some(Leftover::Byte(byte)) => self.read_bytes_const_leftover(buf, byte),
            #[cfg(feature = "bits")]
            Some(Leftover::Bits(_)) => {
                let slice = BitSlice::from_slice_mut(buf);
                self.read_bits_into(slice, _order)?;
                Ok(())
            }
            None => unreachable!(),
        }
    }

    fn read_bytes_const_leftover<const N: usize>(
        &mut self,
        buf: &mut [u8; N],
        byte: u8,
    ) -> Result<(), DekuError> {
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
            self.bits_read += N * 8;

            return Ok(());
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
        self.bits_read += N * 8;

        #[cfg(feature = "logging")]
        log::trace!("read_bytes_const_leftover: returning {:02x?}", &buf);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "bits")]
    use alloc::vec;
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
        let _ = reader.read_bytes_const::<2>(&mut buf, Order::Lsb0).unwrap();
        assert!(reader.end());
        assert_eq!(reader.bits_read, 8 * 2);

        let input = hex!("aa");
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        assert!(!reader.end());
        let _ = reader.read_bits(4, Order::Lsb0).unwrap();
        assert!(!reader.end());
        let _ = reader.read_bits(4, Order::Lsb0).unwrap();
        assert!(reader.end());
        assert_eq!(reader.bits_read, 8);

        let input = hex!("aa");
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        assert!(!reader.end());
        let mut buf = [0; 1];
        let _ = reader.read_bytes(1, &mut buf, Order::Lsb0).unwrap();
        assert!(reader.end());
        assert_eq!(reader.bits_read, 8);
    }

    #[test]
    #[cfg(feature = "bits")]
    fn test_bits_less() {
        let input = hex!("aa");
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        let _ = reader.read_bits(1, Order::Lsb0);
        let _ = reader.read_bits(4, Order::Lsb0);
        let _ = reader.read_bits(3, Order::Lsb0);
    }

    #[test]
    fn test_inner() {
        let input = hex!("aabbcc");
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        let mut buf = [0; 1];
        let _ = reader.read_bytes(1, &mut buf, Order::Lsb0).unwrap();
        assert_eq!([0xaa], buf);
        assert_eq!(reader.bits_read, 8);
    }

    #[cfg(feature = "bits")]
    #[test]
    fn test_bit_order() {
        // Lsb0 one byte
        let input = hex!("ab");
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        // b
        let bits = reader.read_bits(4, Order::Lsb0).unwrap();
        assert_eq!(bits, Some(bitvec![u8, Msb0; 1, 0, 1, 1]));
        // a
        let bits = reader.read_bits(4, Order::Lsb0).unwrap();
        assert_eq!(bits, Some(bitvec![u8, Msb0; 1, 0, 1, 0]));

        // Msb0 one byte
        let input = hex!("ab");
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        // a
        let bits = reader.read_bits(4, Order::Msb0).unwrap();
        assert_eq!(bits, Some(bitvec![u8, Msb0; 1, 0, 1, 0]));
        // b
        let bits = reader.read_bits(4, Order::Msb0).unwrap();
        assert_eq!(bits, Some(bitvec![u8, Msb0; 1, 0, 1, 1]));

        // Lsb0 two bytes
        let input = hex!("abcd");
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        // b
        let bits = reader.read_bits(4, Order::Lsb0).unwrap();
        assert_eq!(bits, Some(bitvec![u8, Msb0; 1, 0, 1, 1]));
        // a
        let bits = reader.read_bits(4, Order::Lsb0).unwrap();
        assert_eq!(bits, Some(bitvec![u8, Msb0; 1, 0, 1, 0]));
        // d
        let bits = reader.read_bits(4, Order::Lsb0).unwrap();
        assert_eq!(bits, Some(bitvec![u8, Msb0; 1, 1, 0, 1]));
        // c
        let bits = reader.read_bits(4, Order::Lsb0).unwrap();
        assert_eq!(bits, Some(bitvec![u8, Msb0; 1, 1, 0, 0]));

        // Msb0 two byte
        let input = hex!("abcd");
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        // a
        let bits = reader.read_bits(4, Order::Msb0).unwrap();
        assert_eq!(bits, Some(bitvec![u8, Msb0; 1, 0, 1, 0]));
        // b
        let bits = reader.read_bits(4, Order::Msb0).unwrap();
        assert_eq!(bits, Some(bitvec![u8, Msb0; 1, 0, 1, 1]));
        // c
        let bits = reader.read_bits(4, Order::Msb0).unwrap();
        assert_eq!(bits, Some(bitvec![u8, Msb0; 1, 1, 0, 0]));
        // d
        let bits = reader.read_bits(4, Order::Msb0).unwrap();
        assert_eq!(bits, Some(bitvec![u8, Msb0; 1, 1, 0, 1]));

        // split order two bytes, lsb first
        let input = hex!("abcd");
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        // b
        let bits = reader.read_bits(4, Order::Lsb0).unwrap();
        assert_eq!(bits, Some(bitvec![u8, Msb0; 1, 0, 1, 1]));
        // a
        let bits = reader.read_bits(4, Order::Lsb0).unwrap();
        assert_eq!(bits, Some(bitvec![u8, Msb0; 1, 0, 1, 0]));
        // c
        let bits = reader.read_bits(4, Order::Msb0).unwrap();
        assert_eq!(bits, Some(bitvec![u8, Msb0; 1, 1, 0, 0]));
        // d
        let bits = reader.read_bits(4, Order::Msb0).unwrap();
        assert_eq!(bits, Some(bitvec![u8, Msb0; 1, 1, 0, 1]));

        // split order two bytes, msb first
        let input = hex!("abcd");
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        // a
        let bits = reader.read_bits(4, Order::Msb0).unwrap();
        assert_eq!(bits, Some(bitvec![u8, Msb0; 1, 0, 1, 0]));
        // b
        let bits = reader.read_bits(4, Order::Msb0).unwrap();
        assert_eq!(bits, Some(bitvec![u8, Msb0; 1, 0, 1, 1]));
        // d
        let bits = reader.read_bits(4, Order::Lsb0).unwrap();
        assert_eq!(bits, Some(bitvec![u8, Msb0; 1, 1, 0, 1]));
        // c
        let bits = reader.read_bits(4, Order::Lsb0).unwrap();
        assert_eq!(bits, Some(bitvec![u8, Msb0; 1, 1, 0, 0]));

        let input = hex!("ab");
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        let bits = reader.read_bits(1, Order::Msb0).unwrap();
        assert_eq!(bits, Some(bitvec![u8, Msb0; 1]));
        let bits = reader.read_bits(3, Order::Msb0).unwrap();
        assert_eq!(bits, Some(bitvec![u8, Msb0; 0, 1, 0]));
        // b
        let bits = reader.read_bits(4, Order::Msb0).unwrap();
        assert_eq!(bits, Some(bitvec![u8, Msb0; 1, 0, 1, 1]));

        // 10101011
        //        |
        // |||
        //    ||||
        let input = hex!("ab");
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        let bits = reader.read_bits(1, Order::Lsb0).unwrap();
        assert_eq!(bits, Some(bitvec![u8, Msb0; 1]));
        let bits = reader.read_bits(3, Order::Msb0).unwrap();
        assert_eq!(bits, Some(bitvec![u8, Msb0; 1, 0, 1]));
        let bits = reader.read_bits(4, Order::Msb0).unwrap();
        assert_eq!(bits, Some(bitvec![u8, Msb0; 0, 1, 0, 1]));
    }

    #[cfg(feature = "bits")]
    #[test]
    fn test_long_unaligned_bytes_read() {
        let input = vec![0xff; 0xff * 2];
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);

        // read 1 bits to make this unaligned
        let bits = reader.read_bits(1, Order::Lsb0).unwrap();
        assert_eq!(bits, Some(bitvec![u8, Msb0; 1]));
        // Now, read the bytes
        let mut out = vec![0x00; 0xff * 2];
        // doesn't crash
        let _ = reader.read_bytes(0xfe * 2, &mut out, Order::Lsb0).unwrap();
    }

    #[cfg(all(feature = "alloc", feature = "bits"))]
    #[test]
    fn test_regression_msb0() {
        // 0110_0100b, 0010_0000b

        let input = [0x64, 0x20];
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        reader.leftover = Some(Leftover::Bits([true, false].as_slice().into()));
        let bits = reader.read_bits(17, Order::Msb0).unwrap();
        assert_eq!(
            bits,
            //                     |left|first                |last                |
            Some(bitvec![u8, Msb0; 1, 0, 0, 1, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0])
        );
    }

    #[cfg(all(feature = "alloc", feature = "bits"))]
    #[test]
    fn test_regression_lsb0() {
        // 0110_0100b, 0010_0000b
        let input = [0x64, 0x20];
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        reader.leftover = Some(Leftover::Bits([true, false].as_slice().into()));
        let bits = reader.read_bits(17, Order::Lsb0).unwrap();
        assert_eq!(
            bits,
            //                     |first               |last                   |left|
            Some(bitvec![u8, Msb0; 0, 1, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 1, 0, 0, 1, 0])
        );
    }
}
