use crate::{DekuError, DekuRead, DekuWrite};
use bitvec::prelude::*;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

impl<'a, Ctx> DekuRead<'a, Ctx> for Ipv4Addr
where
    u32: DekuRead<'a, Ctx>,
{
    fn read(
        input: &'a BitSlice<u8, Msb0>,
        ctx: Ctx,
    ) -> Result<(&'a BitSlice<u8, Msb0>, Self), DekuError>
    where
        Self: Sized,
    {
        let (rest, ip) = u32::read(input, ctx)?;
        Ok((rest, ip.into()))
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

impl<'a, Ctx> DekuRead<'a, Ctx> for Ipv6Addr
where
    u128: DekuRead<'a, Ctx>,
{
    fn read(
        input: &'a BitSlice<u8, Msb0>,
        ctx: Ctx,
    ) -> Result<(&'a BitSlice<u8, Msb0>, Self), DekuError>
    where
        Self: Sized,
    {
        let (rest, ip) = u128::read(input, ctx)?;
        Ok((rest, ip.into()))
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
    use super::*;
    use crate::ctx::Endian;
    use rstest::rstest;

    #[rstest(input, endian, expected, expected_rest,
        case::normal_le([237, 160, 254, 145].as_ref(), Endian::Little, Ipv4Addr::new(145, 254, 160, 237), bits![u8, Msb0;]),
        case::normal_be([145, 254, 160, 237].as_ref(), Endian::Big, Ipv4Addr::new(145, 254, 160, 237), bits![u8, Msb0;]),
    )]
    fn test_ipv4(
        input: &[u8],
        endian: Endian,
        expected: Ipv4Addr,
        expected_rest: &BitSlice<u8, Msb0>,
    ) {
        let bit_slice = input.view_bits::<Msb0>();

        let (rest, res_read) = Ipv4Addr::read(bit_slice, endian).unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, rest);

        let mut res_write = bitvec![u8, Msb0;];
        res_read.write(&mut res_write, endian).unwrap();
        assert_eq!(input.to_vec(), res_write.into_vec());
    }

    #[rstest(input, endian, expected, expected_rest,
        case::normal_le([0xFF, 0x02, 0x0A, 0xC0, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00].as_ref(), Endian::Little, Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xc00a, 0x02ff), bits![u8, Msb0;]),
        case::normal_be([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xC0, 0x0A, 0x02, 0xFF].as_ref(), Endian::Big, Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xc00a, 0x02ff), bits![u8, Msb0;]),
    )]
    fn test_ipv6(
        input: &[u8],
        endian: Endian,
        expected: Ipv6Addr,
        expected_rest: &BitSlice<u8, Msb0>,
    ) {
        let bit_slice = input.view_bits::<Msb0>();

        let (rest, res_read) = Ipv6Addr::read(bit_slice, endian).unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, rest);

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
                0xFF, 0x02, 0x0A, 0xC0, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00
            ],
            ret_write.into_vec()
        );
    }
}
