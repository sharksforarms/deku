use bitvec::prelude::*;
use std::io::Read;

pub struct Container<R: std::io::Read> {
    inner: R,
    leftover: BitVec<u8, Msb0>,
}

impl<R: Read> Container<R> {
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            leftover: BitVec::new(), // with_capacity 8?
        }
    }

    pub fn read_bits(&mut self, amt: usize) -> std::io::Result<BitVec<u8, Msb0>> {
        let mut ret = BitVec::with_capacity(amt);

        if amt < self.leftover.len() {
            let used = self.leftover.split_off(amt);
            ret.extend(&mut self.leftover);
            self.leftover = used;
        } else {
            ret.extend(self.leftover.clone());

            let bits_left = amt - self.leftover.len();
            let mut bytes_len = (bits_left / 8);
            if (bits_left % 8) != 0 {
                bytes_len += 1;
            }
            let mut buf = vec![0; bytes_len];
            self.inner.read_exact(&mut buf);

            let mut rest: BitVec<u8, Msb0> = BitVec::try_from_slice(&buf).unwrap();
            let add = rest.split_off(bits_left);
            ret.extend_from_bitslice(rest.as_bitslice());

            self.leftover = add;
        }

        Ok(ret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_container() {
        use std::io::Cursor;
        let buf = [0x12, 0x34];
        let buf = std::io::Cursor::new(buf);

        let mut container = Container::new(buf);

        let bits = container.read_bits(4).unwrap();
        println!("{}", bits);
        let bits = container.read_bits(4).unwrap();
        println!("{}", bits);
        let bits = container.read_bits(4).unwrap();
        println!("{}", bits);
        let bits = container.read_bits(4).unwrap();
        println!("{}", bits);
    }
}
