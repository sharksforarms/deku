use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use no_std_io::io::Read;

use bitvec::prelude::*;

use crate::{DekuError, DekuReader, DekuWrite};

impl<'a, Ctx> DekuReader<'a, Ctx> for Ipv4Addr
where
    u32: DekuReader<'a, Ctx>,
{
    fn from_reader_with_ctx<R: Read>(
        reader: &mut crate::reader::Reader<R>,
        inner_ctx: Ctx,
    ) -> Result<Self, DekuError> {
        let ip = u32::from_reader_with_ctx(reader, inner_ctx)?;
        Ok(ip.into())
    }
}

impl<Ctx> DekuWrite<Ctx> for Ipv4Addr
where
    u32: DekuWrite<Ctx>,
{
    fn write(&self, output: &mut BitVec<u8, Msb0>, ctx: Ctx) -> Result<(), DekuError> {
        let ip: u32 = (*self).into();
        ip.write(output, ctx)
    }
}

impl<'a, Ctx> DekuReader<'a, Ctx> for Ipv6Addr
where
    u128: DekuReader<'a, Ctx>,
{
    fn from_reader_with_ctx<R: Read>(
        reader: &mut crate::reader::Reader<R>,
        inner_ctx: Ctx,
    ) -> Result<Self, DekuError> {
        let ip = u128::from_reader_with_ctx(reader, inner_ctx)?;
        Ok(ip.into())
    }
}

impl<Ctx> DekuWrite<Ctx> for Ipv6Addr
where
    u128: DekuWrite<Ctx>,
{
    fn write(&self, output: &mut BitVec<u8, Msb0>, ctx: Ctx) -> Result<(), DekuError> {
        let ip: u128 = (*self).into();
        ip.write(output, ctx)
    }
}

impl<Ctx> DekuWrite<Ctx> for IpAddr
where
    Ipv6Addr: DekuWrite<Ctx>,
    Ipv4Addr: DekuWrite<Ctx>,
{
    fn write(&self, output: &mut BitVec<u8, Msb0>, ctx: Ctx) -> Result<(), DekuError> {
        match self {
            IpAddr::V4(ipv4) => ipv4.write(output, ctx),
            IpAddr::V6(ipv6) => ipv6.write(output, ctx),
        }
    }
}

#[cfg(test)]
mod tests {
    use no_std_io::io::Cursor;
    use rstest::rstest;

    use super::*;
    use crate::{ctx::Endian, reader::Reader};

    #[rstest(input, endian, expected,
        case::normal_le([237, 160, 254, 145].as_ref(), Endian::Little, Ipv4Addr::new(145, 254, 160, 237)),
        case::normal_be([145, 254, 160, 237].as_ref(), Endian::Big, Ipv4Addr::new(145, 254, 160, 237)),
    )]
    fn test_ipv4(input: &[u8], endian: Endian, expected: Ipv4Addr) {
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        let res_read = Ipv4Addr::from_reader_with_ctx(&mut reader, endian).unwrap();
        assert_eq!(expected, res_read);

        let mut res_write = bitvec![u8, Msb0;];
        res_read.write(&mut res_write, endian).unwrap();
        assert_eq!(input.to_vec(), res_write.into_vec());
    }

    #[rstest(input, endian, expected,
        case::normal_le([0xFF, 0x02, 0x0A, 0xC0, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00].as_ref(), Endian::Little, Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xc00a, 0x02ff)),
        case::normal_be([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xC0, 0x0A, 0x02, 0xFF].as_ref(), Endian::Big, Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xc00a, 0x02ff)),
    )]
    fn test_ipv6(input: &[u8], endian: Endian, expected: Ipv6Addr) {
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        let res_read = Ipv6Addr::from_reader_with_ctx(&mut reader, endian).unwrap();
        assert_eq!(expected, res_read);

        let mut res_write = bitvec![u8, Msb0;];
        res_read.write(&mut res_write, endian).unwrap();
        assert_eq!(input.to_vec(), res_write.into_vec());
    }

    #[test]
    fn test_ip_addr_write() {
        let ip_addr = IpAddr::V4(Ipv4Addr::new(145, 254, 160, 237));
        let mut ret_write = bitvec![u8, Msb0;];
        ip_addr.write(&mut ret_write, Endian::Little).unwrap();
        assert_eq!(vec![237, 160, 254, 145], ret_write.into_vec());

        let ip_addr = IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xc00a, 0x02ff));
        let mut ret_write = bitvec![u8, Msb0;];
        ip_addr.write(&mut ret_write, Endian::Little).unwrap();
        assert_eq!(
            vec![
                0xff, 0x02, 0x0a, 0xc0, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00
            ],
            ret_write.into_vec()
        );
    }
}
