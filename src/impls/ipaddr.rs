use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use no_std_io::io::{Read, Write};

use crate::reader::Reader;
use crate::writer::Writer;
use crate::{DekuError, DekuReader, DekuWriter};

impl<'a, Ctx> DekuReader<'a, Ctx> for Ipv4Addr
where
    u32: DekuReader<'a, Ctx>,
{
    fn from_reader_with_ctx<R: Read>(
        reader: &mut Reader<R>,
        inner_ctx: Ctx,
    ) -> Result<Self, DekuError> {
        let ip = u32::from_reader_with_ctx(reader, inner_ctx)?;
        Ok(ip.into())
    }
}

impl<Ctx> DekuWriter<Ctx> for Ipv4Addr
where
    u32: DekuWriter<Ctx>,
{
    fn to_writer<W: Write>(&self, writer: &mut Writer<W>, ctx: Ctx) -> Result<(), DekuError> {
        let ip: u32 = (*self).into();
        ip.to_writer(writer, ctx)
    }
}

impl<'a, Ctx> DekuReader<'a, Ctx> for Ipv6Addr
where
    u128: DekuReader<'a, Ctx>,
{
    fn from_reader_with_ctx<R: Read>(
        reader: &mut Reader<R>,
        inner_ctx: Ctx,
    ) -> Result<Self, DekuError> {
        let ip = u128::from_reader_with_ctx(reader, inner_ctx)?;
        Ok(ip.into())
    }
}

impl<Ctx> DekuWriter<Ctx> for Ipv6Addr
where
    u128: DekuWriter<Ctx>,
{
    fn to_writer<W: Write>(&self, writer: &mut Writer<W>, ctx: Ctx) -> Result<(), DekuError> {
        let ip: u128 = (*self).into();
        ip.to_writer(writer, ctx)
    }
}

impl<Ctx> DekuWriter<Ctx> for IpAddr
where
    Ipv6Addr: DekuWriter<Ctx>,
    Ipv4Addr: DekuWriter<Ctx>,
{
    fn to_writer<W: Write>(&self, writer: &mut Writer<W>, ctx: Ctx) -> Result<(), DekuError> {
        match self {
            IpAddr::V4(ipv4) => ipv4.to_writer(writer, ctx),
            IpAddr::V6(ipv6) => ipv6.to_writer(writer, ctx),
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

        let mut writer = Writer::new(vec![]);
        res_read.to_writer(&mut writer, endian).unwrap();
        assert_eq!(input.to_vec(), writer.inner);
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

        let mut writer = Writer::new(vec![]);
        res_read.to_writer(&mut writer, endian).unwrap();
        assert_eq!(input.to_vec(), writer.inner);
    }

    #[test]
    fn test_ip_addr_write() {
        let ip_addr = IpAddr::V4(Ipv4Addr::new(145, 254, 160, 237));

        let mut writer = Writer::new(vec![]);
        ip_addr.to_writer(&mut writer, Endian::Little).unwrap();
        assert_eq!(vec![237, 160, 254, 145], writer.inner);

        let ip_addr = IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xc00a, 0x02ff));
        let mut writer = Writer::new(vec![]);
        ip_addr.to_writer(&mut writer, Endian::Little).unwrap();
        assert_eq!(
            vec![
                0xff, 0x02, 0x0a, 0xc0, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00
            ],
            writer.inner
        );
    }
}
