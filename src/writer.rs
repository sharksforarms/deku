//! Writer for writer functions

use bitvec::bitvec;
use bitvec::{field::BitField, prelude::*};
use no_std_io::io::Write;

#[cfg(feature = "logging")]
use log;

use crate::ctx::Order;
use crate::DekuError;

#[cfg(feature = "alloc")]
use alloc::borrow::ToOwned;

const fn bits_of<T>() -> usize {
    core::mem::size_of::<T>().saturating_mul(<u8>::BITS as usize)
}

/// Container to use with `from_reader`
pub struct Writer<'a, W: Write> {
    pub(crate) inner: &'a mut W,
    leftover: (BitVec<u8, Msb0>, Order),
    /// Total bits written
    pub bits_written: usize,
}

impl<'a, W: Write> Writer<'a, W> {
    /// Create a new `Writer`
    #[inline]
    pub fn new(inner: &'a mut W) -> Self {
        Self {
            inner,
            leftover: (BitVec::new(), Order::Msb0),
            bits_written: 0,
        }
    }

    /// Return the unused bits
    #[inline]
    pub fn rest(&mut self) -> alloc::vec::Vec<bool> {
        self.leftover.0.iter().by_vals().collect()
    }

    /// Write all bits to `Writer` buffer if bits can fit into a byte buffer
    #[inline]
    pub fn write_bits_order(
        &mut self,
        bits: &BitSlice<u8, Msb0>,
        order: Order,
    ) -> Result<(), DekuError> {
        #[cfg(feature = "logging")]
        log::trace!("attempting {} bits : {}", bits.len(), bits);

        // quick return if we don't have enough bits to write to the byte buffer
        if (self.leftover.0.len() + bits.len()) < 8 {
            if self.leftover.1 == Order::Msb0 {
                self.leftover.0.extend_from_bitslice(bits);
                self.leftover.1 = order;

                #[cfg(feature = "logging")]
                log::trace!(
                    "no write: pre-pending {} bits : {} => {}",
                    bits.len(),
                    bits,
                    self.leftover.0
                );
            } else {
                let tmp = self.leftover.0.clone();
                self.leftover.0 = bits.to_owned();
                self.leftover.0.extend_from_bitslice(&tmp);
                self.leftover.1 = order;

                #[cfg(feature = "logging")]
                log::trace!(
                    "no write: post-pending {} bits : {} => {}",
                    bits.len(),
                    bits,
                    self.leftover.0
                );
            }
            return Ok(());
        }

        let mut bits = if self.leftover.0.is_empty() {
            bits
        } else if self.leftover.1 == Order::Msb0 {
            #[cfg(feature = "logging")]
            log::trace!(
                "pre-pending {} bits : {}",
                self.leftover.0.len(),
                self.leftover.0
            );

            self.leftover.0.extend_from_bitslice(bits);

            #[cfg(feature = "logging")]
            log::trace!("now {} bits : {}", self.leftover.0.len(), self.leftover.0);
            &mut self.leftover.0
        } else {
            #[cfg(feature = "logging")]
            log::trace!(
                "post-pending {} bits : {}",
                self.leftover.0.len(),
                self.leftover.0
            );

            let tmp = self.leftover.0.clone();
            self.leftover.0 = bits.to_owned();
            self.leftover.0.extend_from_bitslice(&tmp);

            #[cfg(feature = "logging")]
            log::trace!("now {} bits : {}", self.leftover.0.len(), self.leftover.0);
            &mut self.leftover.0
        };

        if order == Order::Msb0 {
            // This is taken from bitvec's std::io::Read function for BitSlice, but
            // supports no-std
            let mut buf = alloc::vec![0x00; bits.len() / 8];
            let mut count = 0;
            bits.chunks_exact(bits_of::<u8>())
                .zip(buf.iter_mut())
                .for_each(|(byte, slot)| {
                    *slot = byte.load_be();
                    count += 1;
                });
            // SAFETY: there is no safety comment in bitvec, but assume this is safe b/c of bits
            // always still pointing to it's own instance of bits (size-wise)
            bits = unsafe { bits.get_unchecked(count * bits_of::<u8>()..) };

            // TODO: with_capacity?
            self.bits_written = buf.len() * 8;
            self.leftover = (bits.to_bitvec(), order);
            if self.inner.write_all(&buf).is_err() {
                return Err(DekuError::WriteError);
            }
            #[cfg(feature = "logging")]
            log::trace!("wrote {} bits : 0x{:02x?}", buf.len() * 8, &buf);
        } else {
            // This is more complicated, as we need to skip the first bytes until we are "byte aligned"
            // TODO: then reverse the buf before writing in the case that bits.len() > one byte buf ?
            let skip_amount = bits.len() % 8;

            // This is taken from bitvec's std::io::Read function for BitSlice, but
            // supports no-std
            let mut buf = alloc::vec![0x00; bits.len() / 8];
            let mut count = 0;

            // SAFETY: there is no safety comment in bitvec, but assume this is safe b/c of bits
            // always still pointing to it's own instance of bits (size-wise)
            let inner_bits = unsafe { bits.get_unchecked(skip_amount..) };
            inner_bits
                .chunks_exact(bits_of::<u8>())
                .zip(buf.iter_mut())
                .for_each(|(byte, slot)| {
                    *slot = byte.load_be();
                    count += 1;
                });
            // SAFETY: there is no safety comment in bitvec, but assume this is safe b/c of bits
            // always still pointing to it's own instance of bits (size-wise)
            bits = unsafe { bits.get_unchecked(..skip_amount) };

            // TODO: with_capacity?
            self.bits_written = buf.len() * 8;
            self.leftover = (bits.to_bitvec(), order);
            if self.inner.write_all(&buf).is_err() {
                return Err(DekuError::WriteError);
            }
            #[cfg(feature = "logging")]
            log::trace!("wrote {} bits : 0x{:02x?}", buf.len() * 8, &buf);
        }

        #[cfg(feature = "logging")]
        log::trace!(
            "leftover {} bits : {}",
            self.leftover.0.len(),
            self.leftover.0
        );

        Ok(())
    }

    /// Write all bits to `Writer` buffer if bits can fit into a byte buffer
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
        if !self.leftover.0.is_empty() {
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
        if !self.leftover.0.is_empty() {
            #[cfg(feature = "logging")]
            log::trace!("finalized: {} bits leftover", self.leftover.0.len());

            // add bits to be byte aligned so we can write
            self.leftover
                .0
                .extend_from_bitslice(&bitvec![u8, Msb0; 0; 8 - self.leftover.0.len()]);
            let mut buf = alloc::vec![0x00; self.leftover.0.len() / 8];

            // write as many leftover to the buffer (as we can, can't write bits just bytes)
            // TODO: error if bits are leftover? (not bytes aligned)
            self.leftover
                .0
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
