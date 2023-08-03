use bitvec::prelude::*;
use std::io::{self, Read};

use crate::{prelude::NeedSize, DekuError};

pub enum ContainerRet {
    Bytes,
    Bits(BitVec<u8, Msb0>),
}

pub struct Container<R: std::io::Read> {
    inner: R,
    // TODO; bitslice.len() == 8
    leftover: BitVec<u8, Msb0>,
    pub bits_read: usize,
}

impl<R: Read> Container<R> {
    #[inline]
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            leftover: BitVec::new(), // with_capacity 8?
            bits_read: 0,
        }
    }

    #[inline]
    pub fn read_bits(&mut self, amt: usize) -> Result<BitVec<u8, Msb0>, DekuError> {
        if amt == 0 {
            return Ok(BitVec::new());
        }
        let mut ret = BitVec::with_capacity(amt);

        if amt < self.leftover.len() {
            let used = self.leftover.split_off(amt);
            ret.extend(&mut self.leftover);
            self.leftover = used;
        } else {
            ret.extend(self.leftover.clone());

            let bits_left = amt - self.leftover.len();
            let mut bytes_len = bits_left / 8;
            if (bits_left % 8) != 0 {
                bytes_len += 1;
            }
            let mut buf = vec![0; bytes_len];
            if let Err(e) = self.inner.read_exact(&mut buf) {
                if e.kind() == io::ErrorKind::UnexpectedEof {
                    return Err(DekuError::Incomplete(NeedSize::new(amt)));
                }
            }

            let mut rest: BitVec<u8, Msb0> = BitVec::try_from_slice(&buf).unwrap();
            let add = rest.split_off(bits_left);
            ret.extend_from_bitslice(rest.as_bitslice());

            self.leftover = add;
        }

        self.bits_read += ret.len();
        Ok(ret)
    }

    // Attempt to read into bytes instead of bits
    //
    // 1. We must have no leftover bits, so that we are "aligned"
    #[inline]
    pub fn read_bytes(&mut self, amt: usize, buf: &mut [u8]) -> Result<ContainerRet, DekuError> {
        if self.leftover.is_empty() {
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
