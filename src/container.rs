use bitvec::prelude::*;
use std::io::{self, Read};

use crate::{prelude::NeedSize, DekuError};

/// Return from `read_bytes`
pub enum ContainerRet {
    /// Successfully read bytes
    Bytes,
    /// Read Bits intead
    Bits(BitVec<u8, Msb0>),
}

/// Container to use with `from_reader`
pub struct Container<R: Read> {
    inner: R,
    /// bits stored from previous reads that didn't read to the end of a byte size
    leftover: BitVec<u8, Msb0>,
    /// Amount of bits read during the use of `read_bits` and `read_bytes`.
    pub bits_read: usize,
}

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

    /// Attempt to read bits from `Container`. This will always return a `BitVec` and will
    /// correctly add previously read and stored "leftover" bits from previous reads.
    ///
    /// # Guarantees
    /// - The returned `BitVec` will have the size of `amt`
    /// - `self.bits_read` will increase by `amt`
    ///
    /// # Params
    /// `amt`    - Amount of bits that will be read
    #[inline]
    pub fn read_bits(&mut self, amt: usize) -> Result<BitVec<u8, Msb0>, DekuError> {
        if amt == 0 {
            return Ok(BitVec::new());
        }
        let mut ret = BitVec::with_capacity(amt);

        if amt == self.leftover.len() {
            // exact match, just use leftover
            ret = self.leftover.clone();
            self.leftover.clear();
        } else if amt < self.leftover.len() {
            // The entire bits we need to return have been already read previously from bytes but
            // not all were read, return required leftover bits
            let used = self.leftover.split_off(amt);
            ret.extend_from_bitslice(&self.leftover);
            self.leftover = used;
        } else {
            // previous read was not enought to satisfy the amt requirement, return all previously
            // read bits
            ret.extend_from_bitslice(&self.leftover);

            // calulcate the amount of bytes we need to read to read enough bits
            let bits_left = amt - self.leftover.len();
            let mut bytes_len = bits_left / 8;
            if (bits_left % 8) != 0 {
                bytes_len += 1;
            }

            // read in new bytes
            let mut buf = vec![0; bytes_len];
            if let Err(e) = self.inner.read_exact(&mut buf) {
                if e.kind() == io::ErrorKind::UnexpectedEof {
                    return Err(DekuError::Incomplete(NeedSize::new(amt)));
                }

                // TODO: other errors?
            }

            // create bitslice and remove unused bits
            let mut rest: BitVec<u8, Msb0> = BitVec::try_from_slice(&buf).unwrap();
            let not_needed = rest.split_off(bits_left);
            self.leftover = not_needed;

            // create return
            ret.extend_from_bitslice(rest.as_bitslice());
        }

        self.bits_read += ret.len();
        Ok(ret)
    }

    /// Attempt to read bytes from `Container`. This will return `ContainerRet::Bytes` with a valid
    /// `buf` of bytes if we have no "leftover" bytes and thus are byte aligned. If we are not byte
    /// aligned, this will call `read_bits` and return `ContainerRet::Bits(_)` of size `amt` * 8.
    ///
    /// # Params
    /// `amt`    - Amount of bytes that will be read
    #[inline]
    pub fn read_bytes(&mut self, amt: usize, buf: &mut [u8]) -> Result<ContainerRet, DekuError> {
        if self.leftover.is_empty() {
            if buf.len() < amt {
                return Err(DekuError::Incomplete(NeedSize::new(amt * 8)));
            }
            if let Err(e) = self.inner.read_exact(&mut buf[..amt]) {
                if e.kind() == io::ErrorKind::UnexpectedEof {
                    return Err(DekuError::Incomplete(NeedSize::new(amt * 8)));
                }

                // TODO: other errors?
            }
            self.bits_read += amt * 8;
            Ok(ContainerRet::Bytes)
        } else {
            Ok(ContainerRet::Bits(self.read_bits(amt * 8)?))
        }
    }
}