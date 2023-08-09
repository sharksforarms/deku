//! Container for reader functions

use core::cmp::Ordering;

use acid_io::{self, Read};
use bitvec::prelude::*;

use crate::{prelude::NeedSize, DekuError};

#[cfg(feature = "logging")]
use log;

/// Return from `read_bytes`
pub enum ContainerRet {
    /// Successfully read bytes
    Bytes,
    /// Read Bits intead
    Bits(Option<BitVec<u8, Msb0>>),
}

/// Container to use with `from_reader`
pub struct Container<R: Read> {
    inner: R,
    /// bits stored from previous reads that didn't read to the end of a byte size
    leftover: BitVec<u8, Msb0>,
    /// Amount of bits read during the use of `read_bits` and `read_bytes`.
    pub bits_read: usize,
}

/// Max bits requested from [`read_bits`] during one call
pub const MAX_BITS_AMT: usize = 128;

impl<R: Read> Container<R> {
    /// Create a new `Container`
    #[inline]
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            leftover: BitVec::new(), // with_capacity 8?
            bits_read: 0,
        }
    }

    /// Used at the beginning of `from_bytes`. Will read the `amt` of bits, but
    /// not increase bits_read.
    #[inline]
    pub fn skip_bits(&mut self, amt: usize) -> Result<(), DekuError> {
        #[cfg(feature = "logging")]
        log::trace!("skip_bits: {amt}");
        // Save, and keep the leftover bits since the read will most likely be less than a byte
        self.read_bits(amt)?;
        self.bits_read = 0;

        Ok(())
    }

    /// Attempt to read bits from `Container`. This will always return a `BitVec` and will
    /// correctly add previously read and stored "leftover" bits from previous reads.
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
                    if e.kind() == acid_io::ErrorKind::UnexpectedEof {
                        return Err(DekuError::Incomplete(NeedSize::new(amt)));
                    }

                    // TODO: other errors?
                }
                #[cfg(feature = "logging")]
                log::trace!("read_bits: read() {:02x?}", &buf[..bytes_len]);

                // create bitslice and remove unused bits
                let rest = BitSlice::try_from_slice(&buf[..bytes_len]).unwrap();
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

    /// Attempt to read bytes from `Container`. This will return `ContainerRet::Bytes` with a valid
    /// `buf` of bytes if we have no "leftover" bytes and thus are byte aligned. If we are not byte
    /// aligned, this will call `read_bits` and return `ContainerRet::Bits(_)` of size `amt` * 8.
    ///
    /// # Params
    /// `amt`    - Amount of bytes that will be read
    #[inline]
    pub fn read_bytes(&mut self, amt: usize, buf: &mut [u8]) -> Result<ContainerRet, DekuError> {
        #[cfg(feature = "logging")]
        log::trace!("read_bytes: requesting {amt} bytes");
        if self.leftover.is_empty() {
            if buf.len() < amt {
                return Err(DekuError::Incomplete(NeedSize::new(amt * 8)));
            }
            if let Err(e) = self.inner.read_exact(&mut buf[..amt]) {
                if e.kind() == acid_io::ErrorKind::UnexpectedEof {
                    return Err(DekuError::Incomplete(NeedSize::new(amt * 8)));
                }

                // TODO: other errors?
            }
            self.bits_read += amt * 8;
            #[cfg(feature = "logging")]
            log::trace!("read_bytes: returning {buf:02x?}");
            Ok(ContainerRet::Bytes)
        } else {
            Ok(ContainerRet::Bits(self.read_bits(amt * 8)?))
        }
    }
}
