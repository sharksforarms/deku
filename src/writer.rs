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

/// Max bits written to [`Reader::write_bits`] during one call
pub const MAX_BITS_AMT: usize = 128;

/// Container to use with `to_writer`
pub struct Writer<W: Write> {
    pub(crate) inner: W,
    /// Leftover bits
    pub leftover: BitVec<u8, Msb0>,
    /// Total bits written
    pub bits_written: usize,
}

impl<W: Write> Writer<W> {
    /// Create a new `Writer`
    #[inline]
    pub fn new(inner: W) -> Self {
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

    /// Write all `bits` to `Writer` buffer if bits can fit into a byte buffer.
    ///
    /// Any leftover bits will be written before `bits`, and non-written bits will
    /// be stored in `self.leftover`.
    ///
    /// # Params
    /// `bits`    - Amount of bits that will be written. length must be <= [`MAX_BITS_AMT`].
    #[inline(never)]
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
        let mut buf = [0; MAX_BITS_AMT];
        let buf = &mut buf[..bits.len() / 8];
        let mut count = 0;
        bits.chunks_exact(bits_of::<u8>())
            .zip(buf.iter_mut())
            .for_each(|(byte, slot)| {
                *slot = byte.load_be();
                count += 1;
            });

        // SAFETY: This does not have a safety comment in bitvec. But this is safe
        // because of `count` here will always still be within the bounds
        // of `bits`
        bits = unsafe { bits.get_unchecked(count * bits_of::<u8>()..) };

        self.leftover = bits.to_bitvec();
        if let Err(e) = self.inner.write_all(buf) {
            return Err(DekuError::Io(e.kind()));
        }

        self.bits_written += buf.len() * 8;
        #[cfg(feature = "logging")]
        log::trace!("wrote {} bits: {buf:02x?}", buf.len() * 8);

        Ok(())
    }

    /// Write `buf` into `Writer`
    ///
    /// If no `self.leftover`, this will write directly into `Writer`, and if not will write
    /// `buf` using `Self::write_bits()`.
    // The following inline(always) helps performance significantly
    #[inline(always)]
    pub fn write_bytes(&mut self, buf: &[u8]) -> Result<(), DekuError> {
        #[cfg(feature = "logging")]
        log::trace!("writing {} bytes: {buf:02x?}", buf.len());

        if !self.leftover.is_empty() {
            #[cfg(feature = "logging")]
            log::trace!("leftover exists");

            // TODO(perf): we could check here and only send the required bits to finish the byte,
            // instead of sending the entire thing. The rest would be through self.inner.write.
            self.write_bits(&BitVec::from_slice(buf))?;
        } else {
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
        if !self.leftover.is_empty() {
            #[cfg(feature = "logging")]
            log::trace!("finalized: {} bits leftover", self.leftover.len());

            // add bits to be byte aligned so we can write
            self.leftover
                .extend_from_bitslice(&bitvec![u8, Msb0; 0; 8 - self.leftover.len()]);
            let mut buf = alloc::vec![0x00; self.leftover.len() / 8];

            // write as many leftover to the buffer. Because of the previous extend,
            // this will include all the bits in self.leftover
            self.leftover
                .chunks_exact(bits_of::<u8>())
                .zip(buf.iter_mut())
                .for_each(|(byte, slot)| {
                    *slot = byte.load_be();
                });

            if let Err(e) = self.inner.write_all(&buf) {
                return Err(DekuError::Io(e.kind()));
            }
            self.bits_written += buf.len() * 8;

            #[cfg(feature = "logging")]
            log::trace!("finalized: wrote {} bits: {buf:02x?}", buf.len() * 8);
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
