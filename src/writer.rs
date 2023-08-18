//! Writer for writer functions

use bitvec::bitvec;
use bitvec::{field::BitField, prelude::*};
use no_std_io::io::Write;

#[cfg(feature = "logging")]
use log;

use crate::DekuError;

const fn bits_of<T>() -> usize {
    core::mem::size_of::<T>().saturating_mul(<u8>::BITS as usize)
}

/// Container to use with `from_reader`
pub struct Writer<'a, W: Write> {
    pub(crate) inner: &'a mut W,
    leftover: BitVec<u8, Msb0>,
    /// Total bits written
    pub bits_written: usize,
}

impl<'a, W: Write> Writer<'a, W> {
    /// Create a new `Writer`
    #[inline]
    pub fn new(inner: &'a mut W) -> Self {
        Self {
            inner,
            leftover: BitVec::new(),
            bits_written: 0,
        }
    }

    /// Return the unused bits
    #[inline]
    pub fn rest(&mut self) -> alloc::vec::Vec<bool> {
        self.leftover.iter().by_vals().collect()
    }

    /// Write all bits to `Writer` buffer if bits can fit into a byte buffer
    #[inline]
    pub fn write_bits(&mut self, bits: &BitSlice<u8, Msb0>) -> Result<(), DekuError> {
        #[cfg(feature = "logging")]
        log::trace!("attempting {} bits", bits.len());

        // quick return if we can't write to the bytes buffer
        if (self.leftover.len() + bits.len()) < 8 {
            self.leftover.extend_from_bitslice(bits);
            return Ok(());
        }

        // pre-pend the previous attempt to write if needed
        let mut bits = if self.leftover.is_empty() {
            bits
        } else {
            #[cfg(feature = "logging")]
            log::trace!("pre-pending {} bits", self.leftover.len());
            self.leftover.extend_from_bitslice(bits);
            &mut self.leftover
        };

        // one shot impl of BitSlice::read(no read_exact), but for no_std
        let mut buf = alloc::vec![0x00; bits.len() / 8];
        let mut count = 0;
        bits.chunks_exact(bits_of::<u8>())
            .zip(buf.iter_mut())
            .for_each(|(byte, slot)| {
                *slot = byte.load_be();
                count += 1;
            });
        bits = unsafe { bits.get_unchecked(count * bits_of::<u8>()..) };

        // TODO: with_capacity?
        self.bits_written = buf.len() * 8;
        self.leftover = bits.to_bitvec();
        if self.inner.write_all(&buf).is_err() {
            return Err(DekuError::WriteError);
        }
        #[cfg(feature = "logging")]
        log::trace!("wrote {} bits", buf.len() * 8);

        Ok(())
    }

    /// Write `buf` into `Writer`
    // The following inline(always) helps performance significantly
    #[inline(always)]
    pub fn write_bytes(&mut self, buf: &[u8]) -> Result<(), DekuError> {
        #[cfg(feature = "logging")]
        log::trace!("writing {} bytes", buf.len());
        if !self.leftover.is_empty() {
            #[cfg(feature = "logging")]
            log::trace!("leftover exists");
            // TODO: we could check here and only send the required bits to finish the byte?
            // (instead of sending the entire thing)
            self.write_bits(&BitVec::from_slice(buf))?;
        } else {
            if self.inner.write_all(buf).is_err() {
                return Err(DekuError::WriteError);
            }
            self.bits_written = buf.len() * 8;
        }

        Ok(())
    }

    /// Write all remaining bits into `Writer`, adding empty bits to the end so that we can write
    /// into a byte buffer
    #[inline]
    pub fn finalize(&mut self) -> Result<(), DekuError> {
        if !self.leftover.is_empty() {
            #[cfg(feature = "logging")]
            log::trace!("finalized: {} bits leftover", self.leftover.len());

            // add bits to be byte aligned so we can write
            self.leftover
                .extend_from_bitslice(&bitvec![u8, Msb0; 0; 8 - self.leftover.len()]);
            let mut buf = alloc::vec![0x00; self.leftover.len() / 8];

            // write as many leftover to the buffer (as we can, can't write bits just bytes)
            // TODO: error if bits are leftover? (not bytes aligned)
            self.leftover
                .chunks_exact(bits_of::<u8>())
                .zip(buf.iter_mut())
                .for_each(|(byte, slot)| {
                    *slot = byte.load_be();
                });

            if self.inner.write_all(&buf).is_err() {
                return Err(DekuError::WriteError);
            }
            #[cfg(feature = "logging")]
            log::trace!("finalized: wrote {} bits", buf.len() * 8);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hexlit::hex;

    #[test]
    fn test_writer() {
        let mut out_buf = vec![];
        let mut writer = Writer::new(&mut out_buf);

        let mut input = hex!("aa");
        writer.write_bytes(&mut input).unwrap();

        let mut bv = BitVec::<u8, Msb0>::from_slice(&[0xbb]);
        writer.write_bits(&mut bv).unwrap();

        let mut bv = bitvec![u8, Msb0; 1, 1, 1, 1];
        writer.write_bits(&mut bv).unwrap();
        let mut bv = bitvec![u8, Msb0; 0, 0, 0, 1];
        writer.write_bits(&mut bv).unwrap();

        let mut input = hex!("aa");
        writer.write_bytes(&mut input).unwrap();

        let mut bv = bitvec![u8, Msb0; 0, 0, 0, 1];
        writer.write_bits(&mut bv).unwrap();
        let mut bv = bitvec![u8, Msb0; 1, 1, 1, 1];
        writer.write_bits(&mut bv).unwrap();

        let mut bv = bitvec![u8, Msb0; 0, 0, 0, 1];
        writer.write_bits(&mut bv).unwrap();

        let mut input = hex!("aa");
        writer.write_bytes(&mut input).unwrap();

        let mut bv = bitvec![u8, Msb0; 1, 1, 1, 1];
        writer.write_bits(&mut bv).unwrap();

        assert_eq!(
            &mut out_buf,
            &mut vec![0xaa, 0xbb, 0xf1, 0xaa, 0x1f, 0x1a, 0xaf]
        );
    }
}
