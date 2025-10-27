//! Writer for writer functions

#[cfg(feature = "bits")]
use crate::{bitvec::*, BoundedBitVec};
use no_std_io::io::{Seek, SeekFrom, Write};

#[cfg(feature = "logging")]
use log;

#[cfg(feature = "bits")]
use crate::ctx::Order;

use crate::DekuError;

#[cfg(feature = "bits")]
const fn bits_of<T>() -> usize {
    core::mem::size_of::<T>().saturating_mul(<u8>::BITS as usize)
}

/// Container to use with `from_reader`
pub struct Writer<W: Write + Seek> {
    pub(crate) inner: W,
    /// Leftover bits
    #[cfg(feature = "bits")]
    pub leftover: (BoundedBitVec<[u8; 1], Msb0>, Order),
    /// Total bits written
    pub bits_written: usize,
}

impl<W: Write + Seek> Seek for Writer<W> {
    fn seek(&mut self, pos: SeekFrom) -> no_std_io::io::Result<u64> {
        #[cfg(feature = "logging")]
        log::trace!("seek: {pos:?}");

        // clear leftover
        #[cfg(feature = "bits")]
        {
            self.leftover.0.clear();
            self.leftover.1 = Order::Msb0;
        }

        self.inner.seek(pos)
    }
}

impl<W: Write + Seek> Writer<W> {
    /// Create a new `Writer`
    #[inline]
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            #[cfg(feature = "bits")]
            leftover: (BoundedBitVec::new(), Order::Msb0),
            bits_written: 0,
        }
    }

    /// Return the unused bits
    #[inline]
    #[cfg(all(feature = "bits", feature = "alloc"))]
    pub fn rest(&mut self) -> alloc::vec::Vec<bool> {
        self.leftover.0.as_bitslice().iter().by_vals().collect()
    }

    #[cfg(feature = "bits")]
    fn write_bits_order_msb_msb(
        &mut self,
        bits: &BitSlice<u8, Msb0>,
        order: Order,
    ) -> Result<(), DekuError> {
        assert_eq!(self.leftover.1, Order::Msb0);
        assert_eq!(order, Order::Msb0);

        debug_assert!(self.leftover.0.len() < self.leftover.0.capacity());

        let mut leftover = (BoundedBitVec::new(), Order::Msb0);
        core::mem::swap(&mut self.leftover, &mut leftover);

        let rest = if leftover.0.is_empty() {
            (bits, order)
        } else {
            debug_assert!(leftover.0.capacity() >= leftover.0.len());
            let complement = leftover.0.capacity() - leftover.0.len();
            let complement = core::cmp::min(complement, bits.len());
            let (complement, rest) = bits.split_at(complement);
            let (first, complement, rest) = (
                (leftover.0.as_bitslice(), leftover.1),
                (complement, order),
                (rest, order),
            );

            self.leftover.0.extend_from_bitslice(first.0);
            self.leftover.0.extend_from_bitslice(complement.0);

            debug_assert!(self.leftover.0.is_full() || rest.0.is_empty());

            if self.leftover.0.is_full() {
                self.inner.write_all(self.leftover.0.as_raw_slice())?;
                self.bits_written += self.leftover.0.len();
                self.leftover = (BoundedBitVec::new(), Order::Msb0);
            }
            rest
        };

        let iter = rest.0.chunks_exact(bits_of::<u8>());
        let remainder = iter.remainder();
        for byte in iter {
            self.inner.write_all(&[byte.load_be()])?;
        }

        self.bits_written += rest.0.len() - remainder.len();
        debug_assert!(self.leftover.0.len() + remainder.len() <= self.leftover.0.capacity());
        self.leftover.0.extend_from_bitslice(remainder);
        self.leftover.1 = order;
        Ok(())
    }

    #[cfg(feature = "bits")]
    fn write_bits_order_msb_lsb(
        &mut self,
        bits: &BitSlice<u8, Msb0>,
        order: Order,
    ) -> Result<(), DekuError> {
        assert_eq!(self.leftover.1, Order::Msb0);
        assert_eq!(order, Order::Lsb0);

        debug_assert!(self.leftover.0.len() < self.leftover.0.capacity());

        let mut leftover = (BoundedBitVec::new(), Order::Msb0);
        core::mem::swap(&mut self.leftover, &mut leftover);

        let (first, complement, bulk, last) = if leftover.0.is_empty() {
            (
                (BitSlice::empty(), leftover.1),
                (BitSlice::empty(), order),
                (bits, order),
                (BitSlice::empty(), leftover.1),
            )
        } else {
            let remainder = bits.len() % leftover.0.capacity();
            let complement = leftover.0.capacity() - remainder;
            let complement = core::cmp::min(complement, leftover.0.len());
            let front = core::cmp::min(bits.len(), leftover.0.capacity() - complement);
            let (complement, rest) = leftover.0.as_bitslice().split_at(complement);
            let (front, back) = bits.split_at(front);
            (
                (complement, leftover.1),
                (front, order),
                (back, order),
                (rest, leftover.1),
            )
        };

        self.leftover.0.extend_from_bitslice(first.0);
        self.leftover.0.extend_from_bitslice(complement.0);

        if self.leftover.0.is_full() {
            self.inner.write_all(self.leftover.0.as_raw_slice())?;
            self.bits_written += self.leftover.0.len();
            self.leftover = (BoundedBitVec::new(), Order::Msb0);
        }

        let iter = bulk.0.chunks_exact(bits_of::<u8>());
        let remainder = iter.remainder();
        for byte in iter {
            self.inner.write_all(&[byte.load_be()])?;
        }
        self.bits_written += bulk.0.len() - remainder.len();

        debug_assert!(self.leftover.0.len() + remainder.len() <= self.leftover.0.capacity());
        let complement = leftover.0.capacity() - remainder.len();
        let complement = core::cmp::min(complement, last.0.len());
        let (complement, rest) = last.0.split_at(complement);
        self.leftover.0.extend_from_bitslice(remainder);
        self.leftover.0.extend_from_bitslice(complement);

        debug_assert!(self.leftover.0.is_full() || rest.is_empty());

        if self.leftover.0.is_full() {
            self.inner.write_all(self.leftover.0.as_raw_slice())?;
            self.bits_written += self.leftover.0.len();
            self.leftover = (BoundedBitVec::new(), Order::Msb0);
        }

        self.leftover.0.extend_from_bitslice(rest);
        self.leftover.1 = order;
        Ok(())
    }

    #[cfg(feature = "bits")]
    fn write_bits_order_lsb_msb(
        &mut self,
        bits: &BitSlice<u8, Msb0>,
        order: Order,
    ) -> Result<(), DekuError> {
        assert_eq!(self.leftover.1, Order::Lsb0);
        assert_eq!(order, Order::Msb0);

        debug_assert!(self.leftover.0.len() < self.leftover.0.capacity());

        let mut leftover = (BoundedBitVec::new(), Order::Msb0);
        core::mem::swap(&mut self.leftover, &mut leftover);

        let (first, complement, rest) = if leftover.0.is_empty() {
            (
                (bits, order),
                (BitSlice::empty(), leftover.1),
                (BitSlice::empty(), leftover.1),
            )
        } else {
            let remainder = bits.len() % leftover.0.capacity();
            let complement = leftover.0.capacity() - remainder;
            let complement = core::cmp::min(complement, leftover.0.len());
            let (complement, rest) = leftover.0.as_bitslice().split_at(complement);
            ((bits, order), (complement, leftover.1), (rest, leftover.1))
        };

        let iter = first.0.rchunks_exact(bits_of::<u8>());
        let remainder = iter.remainder();
        for byte in iter {
            self.inner.write_all(&[byte.load_be()])?;
        }

        self.bits_written += first.0.len() - remainder.len();
        debug_assert!(self.leftover.0.len() + remainder.len() <= self.leftover.0.capacity());

        self.leftover.0.extend_from_bitslice(remainder);
        self.leftover.0.extend_from_bitslice(complement.0);
        self.leftover.1 = order;

        debug_assert!(self.leftover.0.is_full() || rest.0.is_empty());

        if self.leftover.0.is_full() {
            self.inner.write_all(self.leftover.0.as_raw_slice())?;
            self.bits_written += self.leftover.0.len();
            self.leftover = (BoundedBitVec::new(), Order::Msb0);
        }

        self.leftover.0.extend_from_bitslice(rest.0);
        Ok(())
    }

    #[cfg(feature = "bits")]
    fn write_bits_order_lsb_lsb(
        &mut self,
        bits: &BitSlice<u8, Msb0>,
        order: Order,
    ) -> Result<(), DekuError> {
        assert_eq!(self.leftover.1, Order::Lsb0);
        assert_eq!(order, Order::Lsb0);

        debug_assert!(self.leftover.0.len() < self.leftover.0.capacity());

        let mut leftover = (BoundedBitVec::new(), Order::Msb0);
        core::mem::swap(&mut self.leftover, &mut leftover);

        let rest = if leftover.0.is_empty() {
            (bits, order)
        } else {
            let complement = leftover.0.capacity() - leftover.0.len();
            let complement = core::cmp::min(complement, bits.len());
            let (rest, complement) = bits.split_at(bits.len() - complement);
            let (first, complement, rest) = (
                (complement, order),
                (leftover.0.as_bitslice(), leftover.1),
                (rest, order),
            );

            self.leftover.0.extend_from_bitslice(first.0);
            self.leftover.0.extend_from_bitslice(complement.0);

            debug_assert!(self.leftover.0.is_full() || rest.0.is_empty());

            if self.leftover.0.is_full() {
                self.inner.write_all(self.leftover.0.as_raw_slice())?;
                self.bits_written += self.leftover.0.len();
                self.leftover = (BoundedBitVec::new(), Order::Msb0);
            }
            rest
        };

        let iter = rest.0.rchunks_exact(bits_of::<u8>());
        let remainder = iter.remainder();
        for byte in iter {
            self.inner.write_all(&[byte.load_be()])?;
        }

        self.bits_written += rest.0.len() - remainder.len();
        debug_assert!(self.leftover.0.len() + remainder.len() <= self.leftover.0.capacity());
        self.leftover.0.extend_from_bitslice(remainder);
        self.leftover.1 = order;
        Ok(())
    }

    /// Write all bits to `Writer` buffer if bits can fit into a byte buffer
    #[cfg(feature = "bits")]
    #[inline]
    pub fn write_bits_order(
        &mut self,
        bits: &BitSlice<u8, Msb0>,
        order: Order,
    ) -> Result<(), DekuError> {
        match self.leftover.1 {
            Order::Msb0 => match order {
                Order::Msb0 => self.write_bits_order_msb_msb(bits, order),
                Order::Lsb0 => self.write_bits_order_msb_lsb(bits, order),
            },
            Order::Lsb0 => match order {
                Order::Msb0 => self.write_bits_order_lsb_msb(bits, order),
                Order::Lsb0 => self.write_bits_order_lsb_lsb(bits, order),
            },
        }
    }

    /// Write all bits to `Writer` buffer if bits can fit into a byte buffer
    #[cfg(feature = "bits")]
    #[inline]
    pub fn write_bits(&mut self, bits: &BitSlice<u8, Msb0>) -> Result<(), DekuError> {
        self.write_bits_order(bits, Order::Msb0)
    }

    /// Write `buf` into `Writer`
    // The following inline(always) helps performance significantly
    #[inline(always)]
    pub fn write_bytes(&mut self, buf: &[u8]) -> Result<(), DekuError> {
        #[cfg(feature = "logging")]
        log::trace!("writing {} bytes", buf.len());

        #[cfg(feature = "bits")]
        if !self.leftover.0.is_empty() {
            #[cfg(feature = "logging")]
            log::trace!("leftover exists");

            // TODO: we could check here and only send the required bits to finish the byte?
            // (instead of sending the entire thing)
            self.write_bits(BitSlice::from_slice(buf))?;
        } else {
            if let Err(e) = self.inner.write_all(buf) {
                return Err(DekuError::Io(e.kind()));
            }
            self.bits_written += buf.len() * 8;
        }

        #[cfg(not(feature = "bits"))]
        {
            if let Err(e) = self.inner.write_all(buf) {
                return Err(DekuError::Io(e.kind()));
            }
            self.bits_written += buf.len() * 8;
        }

        Ok(())
    }

    /// Write all remaining bits into `Writer`, adding empty bits to the end so that we can write
    /// into a byte buffer
    #[inline]
    pub fn finalize(&mut self) -> Result<(), DekuError> {
        #[cfg(feature = "bits")]
        {
            let padded = bitarr!(u8, Msb0; 0; 8);
            debug_assert!(self.leftover.0.len() < 8);
            let len = (8 - self.leftover.0.len()) % 8;
            self.write_bits_order(&padded[..len], self.leftover.1)?;
        }
        Ok(())
    }
}

#[cfg(all(feature = "std", feature = "bits"))]
#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use assert_hex::assert_eq_hex;
    use hexlit::hex;

    #[test]
    fn test_writer_bits() {
        let mut out_buf = Cursor::new(vec![]);
        let mut writer = Writer::new(&mut out_buf);

        let input = hex!("aa");
        writer.write_bytes(&input).unwrap();

        let bv = BitVec::<u8, Msb0>::from_slice(&[0xbb]);
        writer.write_bits(&bv).unwrap();

        let bv = bitvec![u8, Msb0; 1, 1, 1, 1];
        writer.write_bits(&bv).unwrap();
        let bv = bitvec![u8, Msb0; 0, 0, 0, 1];
        writer.write_bits(&bv).unwrap();

        let input = hex!("aa");
        writer.write_bytes(&input).unwrap();

        let bv = bitvec![u8, Msb0; 0, 0, 0, 1];
        writer.write_bits(&bv).unwrap();
        let bv = bitvec![u8, Msb0; 1, 1, 1, 1];
        writer.write_bits(&bv).unwrap();

        let bv = bitvec![u8, Msb0; 0, 0, 0, 1];
        writer.write_bits(&bv).unwrap();

        let input = hex!("aa");
        writer.write_bytes(&input).unwrap();

        let bv = bitvec![u8, Msb0; 1, 1, 1, 1];
        writer.write_bits(&bv).unwrap();

        assert_eq!(
            &mut out_buf.into_inner(),
            &mut vec![0xaa, 0xbb, 0xf1, 0xaa, 0x1f, 0x1a, 0xaf]
        );
    }

    #[test]
    fn test_writer_bytes() {
        let mut out_buf = Cursor::new(vec![]);
        let mut writer = Writer::new(&mut out_buf);

        let input = hex!("aa");
        writer.write_bytes(&input).unwrap();

        assert_eq!(&mut out_buf.into_inner(), &mut vec![0xaa]);
    }

    #[test]
    fn test_bit_order() {
        let mut out_buf = Cursor::new(vec![]);
        let mut writer = Writer::new(&mut out_buf);
        writer
            .write_bits_order(&bitvec![u8, Msb0; 1, 0, 1, 0], Order::Msb0)
            .unwrap();
        writer
            .write_bits_order(&bitvec![u8, Msb0; 0, 1, 0, 1], Order::Msb0)
            .unwrap();
        writer.finalize().unwrap();
        assert_eq!(out_buf.into_inner(), [0b1010_0101]);

        let mut out_buf = Cursor::new(vec![]);
        let mut writer = Writer::new(&mut out_buf);
        writer
            .write_bits_order(&bitvec![u8, Msb0; 1, 0, 1, 0], Order::Lsb0)
            .unwrap();
        writer
            .write_bits_order(&bitvec![u8, Msb0; 0, 1, 0, 1], Order::Lsb0)
            .unwrap();
        writer.finalize().unwrap();
        assert_eq!(out_buf.into_inner(), [0b0101_1010]);

        let mut out_buf = Cursor::new(vec![]);
        let mut writer = Writer::new(&mut out_buf);
        writer
            .write_bits_order(&bitvec![u8, Msb0; 1, 0, 1, 0], Order::Msb0)
            .unwrap();
        writer
            .write_bits_order(&bitvec![u8, Msb0; 0, 1, 0, 1], Order::Lsb0)
            .unwrap();
        writer.finalize().unwrap();
        assert_eq!(out_buf.into_inner(), [0b1010_0101]);

        let mut out_buf = Cursor::new(vec![]);
        let mut writer = Writer::new(&mut out_buf);
        writer
            .write_bits_order(&bitvec![u8, Msb0; 1, 0, 1, 0, 1, 0], Order::Msb0)
            .unwrap();
        writer
            .write_bits_order(&bitvec![u8, Msb0; 0, 1, 0, 1, 0, 1], Order::Msb0)
            .unwrap();
        writer.finalize().unwrap();
        assert_eq!(out_buf.into_inner(), [0b1010_1001, 0b0101_0000]);

        let mut out_buf = Cursor::new(vec![]);
        let mut writer = Writer::new(&mut out_buf);
        writer
            .write_bits_order(&bitvec![u8, Msb0; 1, 0, 1, 0, 1, 0], Order::Lsb0)
            .unwrap();
        writer
            .write_bits_order(&bitvec![u8, Msb0; 0, 1, 0, 1, 0, 1], Order::Lsb0)
            .unwrap();
        writer.finalize().unwrap();
        assert_eq!(out_buf.into_inner(), [0b0110_1010, 0b0000_0101]);

        let mut out_buf = Cursor::new(vec![]);
        let mut writer = Writer::new(&mut out_buf);
        writer
            .write_bits_order(&bitvec![u8, Msb0; 1, 0, 1, 0, 1, 0], Order::Lsb0)
            .unwrap();
        writer
            .write_bits_order(&bitvec![u8, Msb0; 0, 1, 0, 1, 0, 1], Order::Msb0)
            .unwrap();
        writer.finalize().unwrap();
        assert_eq_hex!(out_buf.into_inner(), [0b0101_0110, 0b1010_0000]);

        let mut out_buf = Cursor::new(vec![]);
        let mut writer = Writer::new(&mut out_buf);
        writer
            .write_bits_order(&bitvec![u8, Msb0; 1, 0, 1, 0, 1, 0], Order::Msb0)
            .unwrap();
        writer
            .write_bits_order(&bitvec![u8, Msb0; 0, 1, 0, 1, 0, 1], Order::Lsb0)
            .unwrap();
        writer.finalize().unwrap();
        assert_eq!(out_buf.into_inner(), [0b1001_0101, 0b0000_1010]);
    }
}
