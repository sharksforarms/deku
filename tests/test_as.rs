use deku::ctx::Size;
use deku::prelude::*;
use deku::{bitvec, DekuReadAs, DekuWriteAs, Same};
use std::marker::PhantomData;

#[derive(DekuRead, DekuWrite, PartialEq, Debug)]
struct Foo {
    #[deku(as = "VecWithLen<Same>", bytes = "4", ctx = "()")]
    ints: Box<[u32]>,
}

#[test]
fn test_roundtrip() {
    let foo = Foo {
        ints: Box::new([6, 1, 25, 9]),
    };
    let bytes = foo.to_bytes().unwrap();
    let foo2 = Foo::from_bytes((&bytes, 0)).unwrap().1;
    assert_eq!(foo2, foo);
}

#[test]
fn test_vecwithlen() {
    let foo = Foo {
        ints: Box::new([6, 1, 25, 9]),
    };
    let bytes = foo.to_bytes().unwrap();
    #[rustfmt::skip]
    assert_eq!(
        bytes,
        [
            4,  0, 0, 0,
            6,  0, 0, 0,
            1,  0, 0, 0,
            25, 0, 0, 0,
            9,  0, 0, 0
        ]
    );
}

struct VecWithLen<T> {
    _marker: PhantomData<T>,
}

impl<'a, Ctx: Copy, T, U: DekuReadAs<'a, T, Ctx>> DekuReadAs<'a, Box<[T]>, (Size, Ctx)>
    for VecWithLen<U>
{
    fn read_as(
        input: &'a bitvec::BitSlice<bitvec::Msb0, u8>,
        ctx: (Size, Ctx),
    ) -> Result<(&'a bitvec::BitSlice<bitvec::Msb0, u8>, Box<[T]>), DekuError> {
        let (rest, vec) = Self::read_as(input, ctx)?;
        Ok((rest, Vec::into_boxed_slice(vec)))
    }
}

impl<'a, T, U: DekuReadAs<'a, T, ()>> DekuReadAs<'a, Box<[T]>> for VecWithLen<U> {
    fn read_as(
        input: &'a bitvec::BitSlice<bitvec::Msb0, u8>,
        ctx: (),
    ) -> Result<(&'a bitvec::BitSlice<bitvec::Msb0, u8>, Box<[T]>), DekuError> {
        let (rest, vec) = Self::read_as(input, ctx)?;
        Ok((rest, Vec::into_boxed_slice(vec)))
    }
}

impl<'a, Ctx: Copy, T, U: DekuReadAs<'a, T, Ctx>> DekuReadAs<'a, Vec<T>, (Size, Ctx)>
    for VecWithLen<U>
{
    fn read_as(
        input: &'a bitvec::BitSlice<bitvec::Msb0, u8>,
        (size, ctx): (Size, Ctx),
    ) -> Result<(&'a bitvec::BitSlice<bitvec::Msb0, u8>, Vec<T>), DekuError> {
        let mut rest = input;
        let (r, len) = usize::read(rest, size)?;
        rest = r;
        let mut out = Vec::with_capacity(len);
        for _ in 0..len {
            let (r, elem) = U::read_as(rest, ctx)?;
            out.push(elem);
            rest = r
        }
        Ok((rest, out))
    }
}

impl<'a, T, U: DekuReadAs<'a, T, ()>> DekuReadAs<'a, Vec<T>> for VecWithLen<U> {
    fn read_as(
        input: &'a bitvec::BitSlice<bitvec::Msb0, u8>,
        _ctx: (),
    ) -> Result<(&'a bitvec::BitSlice<bitvec::Msb0, u8>, Vec<T>), DekuError> {
        Self::read_as(input, (Size::of::<usize>(), ()))
    }
}

impl<Ctx: Copy, T, U: DekuWriteAs<T, Ctx>> DekuWriteAs<[T], (Size, Ctx)> for VecWithLen<U> {
    fn write_as(
        source: &[T],
        output: &mut bitvec::BitVec<bitvec::Msb0, u8>,
        (size, ctx): (Size, Ctx),
    ) -> Result<(), DekuError> {
        dbg!(&output);
        source.len().write(output, size)?;
        dbg!(&output);
        for elem in source {
            U::write_as(elem, output, ctx)?;
        }
        Ok(())
    }
}

impl<T, U: DekuWriteAs<T, ()>> DekuWriteAs<[T]> for VecWithLen<U> {
    fn write_as(
        source: &[T],
        output: &mut bitvec::BitVec<bitvec::Msb0, u8>,
        _ctx: (),
    ) -> Result<(), DekuError> {
        Self::write_as(source, output, (Size::of::<usize>(), ()))
    }
}

impl<Ctx: Copy, T, U: DekuWriteAs<T, Ctx>> DekuWriteAs<Box<[T]>, (Size, Ctx)> for VecWithLen<U> {
    fn write_as(
        source: &Box<[T]>,
        output: &mut bitvec::BitVec<bitvec::Msb0, u8>,
        ctx: (Size, Ctx),
    ) -> Result<(), DekuError> {
        Self::write_as(&**source, output, ctx)
    }
}

impl<T, U: DekuWriteAs<T, ()>> DekuWriteAs<Box<[T]>> for VecWithLen<U> {
    fn write_as(
        source: &Box<[T]>,
        output: &mut bitvec::BitVec<bitvec::Msb0, u8>,
        ctx: (),
    ) -> Result<(), DekuError> {
        Self::write_as(&**source, output, ctx)
    }
}

impl<Ctx: Copy, T, U: DekuWriteAs<T, Ctx>> DekuWriteAs<Vec<T>, (Size, Ctx)> for VecWithLen<U> {
    fn write_as(
        source: &Vec<T>,
        output: &mut bitvec::BitVec<bitvec::Msb0, u8>,
        ctx: (Size, Ctx),
    ) -> Result<(), DekuError> {
        Self::write_as(&**source, output, ctx)
    }
}

impl<T, U: DekuWriteAs<T, ()>> DekuWriteAs<Vec<T>> for VecWithLen<U> {
    fn write_as(
        source: &Vec<T>,
        output: &mut bitvec::BitVec<bitvec::Msb0, u8>,
        ctx: (),
    ) -> Result<(), DekuError> {
        Self::write_as(&**source, output, ctx)
    }
}
