//! Implementations of DekuRead and DekuWrite for [T; N] where 0 < N <= 32

// Forked out DekuRead and DekuWrite impls for [T; N] into a separate file
// since the list of impls is long

use super::{DekuRead, DekuWrite};
use crate::error::DekuError;
use bitvec::prelude::*;
pub use deku_derive::*;

/*
Note: They are all generic because those type are not really required a context. They forward context
to inner types.
 */
macro_rules! ImplDekuSliceTraits {
    ($typ:ty, $count:expr) => {
        impl<Ctx: Copy> DekuRead<Ctx> for [$typ; $count]
        where
            $typ: DekuRead<Ctx>,
        {
            fn read(
                input: &BitSlice<Msb0, u8>,
                ctx: Ctx,
            ) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError>
            where
                Self: Sized,
            {
                let mut slice: [$typ; $count] = Default::default();
                let mut rest = input;
                for i in 0..$count {
                    let (new_rest, value) = <$typ>::read(rest, ctx)?;
                    slice[i] = value;
                    rest = new_rest;
                }

                Ok((rest, slice))
            }
        }

        impl<Ctx: Copy> DekuWrite<Ctx> for [$typ; $count]
        where
            $typ: DekuWrite<Ctx>,
        {
            fn write(&self, ctx: Ctx) -> Result<BitVec<Msb0, u8>, DekuError> {
                let mut acc = BitVec::new();

                for v in self {
                    let r = v.write(ctx)?;
                    acc.extend(r);
                }

                Ok(acc)
            }
        }
    };
}

/*
Generate the list with:

```python
TYPES = [
    'u8', 'u16', 'u32', 'u64', 'u128', 'usize',
    'i8', 'i16', 'i32', 'i64', 'i128', 'isize',
    'f32', 'f64',
]
MAX_SIZE = 32

for typ in TYPES:
    for size in range(1, MAX_SIZE + 1):
        print(f"ImplDekuSliceTraits!({typ}, {size});")
```
*/

ImplDekuSliceTraits!(u8, 1);
ImplDekuSliceTraits!(u8, 2);
ImplDekuSliceTraits!(u8, 3);
ImplDekuSliceTraits!(u8, 4);
ImplDekuSliceTraits!(u8, 5);
ImplDekuSliceTraits!(u8, 6);
ImplDekuSliceTraits!(u8, 7);
ImplDekuSliceTraits!(u8, 8);
ImplDekuSliceTraits!(u8, 9);
ImplDekuSliceTraits!(u8, 10);
ImplDekuSliceTraits!(u8, 11);
ImplDekuSliceTraits!(u8, 12);
ImplDekuSliceTraits!(u8, 13);
ImplDekuSliceTraits!(u8, 14);
ImplDekuSliceTraits!(u8, 15);
ImplDekuSliceTraits!(u8, 16);
ImplDekuSliceTraits!(u8, 17);
ImplDekuSliceTraits!(u8, 18);
ImplDekuSliceTraits!(u8, 19);
ImplDekuSliceTraits!(u8, 20);
ImplDekuSliceTraits!(u8, 21);
ImplDekuSliceTraits!(u8, 22);
ImplDekuSliceTraits!(u8, 23);
ImplDekuSliceTraits!(u8, 24);
ImplDekuSliceTraits!(u8, 25);
ImplDekuSliceTraits!(u8, 26);
ImplDekuSliceTraits!(u8, 27);
ImplDekuSliceTraits!(u8, 28);
ImplDekuSliceTraits!(u8, 29);
ImplDekuSliceTraits!(u8, 30);
ImplDekuSliceTraits!(u8, 31);
ImplDekuSliceTraits!(u8, 32);
ImplDekuSliceTraits!(u16, 1);
ImplDekuSliceTraits!(u16, 2);
ImplDekuSliceTraits!(u16, 3);
ImplDekuSliceTraits!(u16, 4);
ImplDekuSliceTraits!(u16, 5);
ImplDekuSliceTraits!(u16, 6);
ImplDekuSliceTraits!(u16, 7);
ImplDekuSliceTraits!(u16, 8);
ImplDekuSliceTraits!(u16, 9);
ImplDekuSliceTraits!(u16, 10);
ImplDekuSliceTraits!(u16, 11);
ImplDekuSliceTraits!(u16, 12);
ImplDekuSliceTraits!(u16, 13);
ImplDekuSliceTraits!(u16, 14);
ImplDekuSliceTraits!(u16, 15);
ImplDekuSliceTraits!(u16, 16);
ImplDekuSliceTraits!(u16, 17);
ImplDekuSliceTraits!(u16, 18);
ImplDekuSliceTraits!(u16, 19);
ImplDekuSliceTraits!(u16, 20);
ImplDekuSliceTraits!(u16, 21);
ImplDekuSliceTraits!(u16, 22);
ImplDekuSliceTraits!(u16, 23);
ImplDekuSliceTraits!(u16, 24);
ImplDekuSliceTraits!(u16, 25);
ImplDekuSliceTraits!(u16, 26);
ImplDekuSliceTraits!(u16, 27);
ImplDekuSliceTraits!(u16, 28);
ImplDekuSliceTraits!(u16, 29);
ImplDekuSliceTraits!(u16, 30);
ImplDekuSliceTraits!(u16, 31);
ImplDekuSliceTraits!(u16, 32);
ImplDekuSliceTraits!(u32, 1);
ImplDekuSliceTraits!(u32, 2);
ImplDekuSliceTraits!(u32, 3);
ImplDekuSliceTraits!(u32, 4);
ImplDekuSliceTraits!(u32, 5);
ImplDekuSliceTraits!(u32, 6);
ImplDekuSliceTraits!(u32, 7);
ImplDekuSliceTraits!(u32, 8);
ImplDekuSliceTraits!(u32, 9);
ImplDekuSliceTraits!(u32, 10);
ImplDekuSliceTraits!(u32, 11);
ImplDekuSliceTraits!(u32, 12);
ImplDekuSliceTraits!(u32, 13);
ImplDekuSliceTraits!(u32, 14);
ImplDekuSliceTraits!(u32, 15);
ImplDekuSliceTraits!(u32, 16);
ImplDekuSliceTraits!(u32, 17);
ImplDekuSliceTraits!(u32, 18);
ImplDekuSliceTraits!(u32, 19);
ImplDekuSliceTraits!(u32, 20);
ImplDekuSliceTraits!(u32, 21);
ImplDekuSliceTraits!(u32, 22);
ImplDekuSliceTraits!(u32, 23);
ImplDekuSliceTraits!(u32, 24);
ImplDekuSliceTraits!(u32, 25);
ImplDekuSliceTraits!(u32, 26);
ImplDekuSliceTraits!(u32, 27);
ImplDekuSliceTraits!(u32, 28);
ImplDekuSliceTraits!(u32, 29);
ImplDekuSliceTraits!(u32, 30);
ImplDekuSliceTraits!(u32, 31);
ImplDekuSliceTraits!(u32, 32);
ImplDekuSliceTraits!(u64, 1);
ImplDekuSliceTraits!(u64, 2);
ImplDekuSliceTraits!(u64, 3);
ImplDekuSliceTraits!(u64, 4);
ImplDekuSliceTraits!(u64, 5);
ImplDekuSliceTraits!(u64, 6);
ImplDekuSliceTraits!(u64, 7);
ImplDekuSliceTraits!(u64, 8);
ImplDekuSliceTraits!(u64, 9);
ImplDekuSliceTraits!(u64, 10);
ImplDekuSliceTraits!(u64, 11);
ImplDekuSliceTraits!(u64, 12);
ImplDekuSliceTraits!(u64, 13);
ImplDekuSliceTraits!(u64, 14);
ImplDekuSliceTraits!(u64, 15);
ImplDekuSliceTraits!(u64, 16);
ImplDekuSliceTraits!(u64, 17);
ImplDekuSliceTraits!(u64, 18);
ImplDekuSliceTraits!(u64, 19);
ImplDekuSliceTraits!(u64, 20);
ImplDekuSliceTraits!(u64, 21);
ImplDekuSliceTraits!(u64, 22);
ImplDekuSliceTraits!(u64, 23);
ImplDekuSliceTraits!(u64, 24);
ImplDekuSliceTraits!(u64, 25);
ImplDekuSliceTraits!(u64, 26);
ImplDekuSliceTraits!(u64, 27);
ImplDekuSliceTraits!(u64, 28);
ImplDekuSliceTraits!(u64, 29);
ImplDekuSliceTraits!(u64, 30);
ImplDekuSliceTraits!(u64, 31);
ImplDekuSliceTraits!(u64, 32);
ImplDekuSliceTraits!(u128, 1);
ImplDekuSliceTraits!(u128, 2);
ImplDekuSliceTraits!(u128, 3);
ImplDekuSliceTraits!(u128, 4);
ImplDekuSliceTraits!(u128, 5);
ImplDekuSliceTraits!(u128, 6);
ImplDekuSliceTraits!(u128, 7);
ImplDekuSliceTraits!(u128, 8);
ImplDekuSliceTraits!(u128, 9);
ImplDekuSliceTraits!(u128, 10);
ImplDekuSliceTraits!(u128, 11);
ImplDekuSliceTraits!(u128, 12);
ImplDekuSliceTraits!(u128, 13);
ImplDekuSliceTraits!(u128, 14);
ImplDekuSliceTraits!(u128, 15);
ImplDekuSliceTraits!(u128, 16);
ImplDekuSliceTraits!(u128, 17);
ImplDekuSliceTraits!(u128, 18);
ImplDekuSliceTraits!(u128, 19);
ImplDekuSliceTraits!(u128, 20);
ImplDekuSliceTraits!(u128, 21);
ImplDekuSliceTraits!(u128, 22);
ImplDekuSliceTraits!(u128, 23);
ImplDekuSliceTraits!(u128, 24);
ImplDekuSliceTraits!(u128, 25);
ImplDekuSliceTraits!(u128, 26);
ImplDekuSliceTraits!(u128, 27);
ImplDekuSliceTraits!(u128, 28);
ImplDekuSliceTraits!(u128, 29);
ImplDekuSliceTraits!(u128, 30);
ImplDekuSliceTraits!(u128, 31);
ImplDekuSliceTraits!(u128, 32);
ImplDekuSliceTraits!(usize, 1);
ImplDekuSliceTraits!(usize, 2);
ImplDekuSliceTraits!(usize, 3);
ImplDekuSliceTraits!(usize, 4);
ImplDekuSliceTraits!(usize, 5);
ImplDekuSliceTraits!(usize, 6);
ImplDekuSliceTraits!(usize, 7);
ImplDekuSliceTraits!(usize, 8);
ImplDekuSliceTraits!(usize, 9);
ImplDekuSliceTraits!(usize, 10);
ImplDekuSliceTraits!(usize, 11);
ImplDekuSliceTraits!(usize, 12);
ImplDekuSliceTraits!(usize, 13);
ImplDekuSliceTraits!(usize, 14);
ImplDekuSliceTraits!(usize, 15);
ImplDekuSliceTraits!(usize, 16);
ImplDekuSliceTraits!(usize, 17);
ImplDekuSliceTraits!(usize, 18);
ImplDekuSliceTraits!(usize, 19);
ImplDekuSliceTraits!(usize, 20);
ImplDekuSliceTraits!(usize, 21);
ImplDekuSliceTraits!(usize, 22);
ImplDekuSliceTraits!(usize, 23);
ImplDekuSliceTraits!(usize, 24);
ImplDekuSliceTraits!(usize, 25);
ImplDekuSliceTraits!(usize, 26);
ImplDekuSliceTraits!(usize, 27);
ImplDekuSliceTraits!(usize, 28);
ImplDekuSliceTraits!(usize, 29);
ImplDekuSliceTraits!(usize, 30);
ImplDekuSliceTraits!(usize, 31);
ImplDekuSliceTraits!(usize, 32);
ImplDekuSliceTraits!(i8, 1);
ImplDekuSliceTraits!(i8, 2);
ImplDekuSliceTraits!(i8, 3);
ImplDekuSliceTraits!(i8, 4);
ImplDekuSliceTraits!(i8, 5);
ImplDekuSliceTraits!(i8, 6);
ImplDekuSliceTraits!(i8, 7);
ImplDekuSliceTraits!(i8, 8);
ImplDekuSliceTraits!(i8, 9);
ImplDekuSliceTraits!(i8, 10);
ImplDekuSliceTraits!(i8, 11);
ImplDekuSliceTraits!(i8, 12);
ImplDekuSliceTraits!(i8, 13);
ImplDekuSliceTraits!(i8, 14);
ImplDekuSliceTraits!(i8, 15);
ImplDekuSliceTraits!(i8, 16);
ImplDekuSliceTraits!(i8, 17);
ImplDekuSliceTraits!(i8, 18);
ImplDekuSliceTraits!(i8, 19);
ImplDekuSliceTraits!(i8, 20);
ImplDekuSliceTraits!(i8, 21);
ImplDekuSliceTraits!(i8, 22);
ImplDekuSliceTraits!(i8, 23);
ImplDekuSliceTraits!(i8, 24);
ImplDekuSliceTraits!(i8, 25);
ImplDekuSliceTraits!(i8, 26);
ImplDekuSliceTraits!(i8, 27);
ImplDekuSliceTraits!(i8, 28);
ImplDekuSliceTraits!(i8, 29);
ImplDekuSliceTraits!(i8, 30);
ImplDekuSliceTraits!(i8, 31);
ImplDekuSliceTraits!(i8, 32);
ImplDekuSliceTraits!(i16, 1);
ImplDekuSliceTraits!(i16, 2);
ImplDekuSliceTraits!(i16, 3);
ImplDekuSliceTraits!(i16, 4);
ImplDekuSliceTraits!(i16, 5);
ImplDekuSliceTraits!(i16, 6);
ImplDekuSliceTraits!(i16, 7);
ImplDekuSliceTraits!(i16, 8);
ImplDekuSliceTraits!(i16, 9);
ImplDekuSliceTraits!(i16, 10);
ImplDekuSliceTraits!(i16, 11);
ImplDekuSliceTraits!(i16, 12);
ImplDekuSliceTraits!(i16, 13);
ImplDekuSliceTraits!(i16, 14);
ImplDekuSliceTraits!(i16, 15);
ImplDekuSliceTraits!(i16, 16);
ImplDekuSliceTraits!(i16, 17);
ImplDekuSliceTraits!(i16, 18);
ImplDekuSliceTraits!(i16, 19);
ImplDekuSliceTraits!(i16, 20);
ImplDekuSliceTraits!(i16, 21);
ImplDekuSliceTraits!(i16, 22);
ImplDekuSliceTraits!(i16, 23);
ImplDekuSliceTraits!(i16, 24);
ImplDekuSliceTraits!(i16, 25);
ImplDekuSliceTraits!(i16, 26);
ImplDekuSliceTraits!(i16, 27);
ImplDekuSliceTraits!(i16, 28);
ImplDekuSliceTraits!(i16, 29);
ImplDekuSliceTraits!(i16, 30);
ImplDekuSliceTraits!(i16, 31);
ImplDekuSliceTraits!(i16, 32);
ImplDekuSliceTraits!(i32, 1);
ImplDekuSliceTraits!(i32, 2);
ImplDekuSliceTraits!(i32, 3);
ImplDekuSliceTraits!(i32, 4);
ImplDekuSliceTraits!(i32, 5);
ImplDekuSliceTraits!(i32, 6);
ImplDekuSliceTraits!(i32, 7);
ImplDekuSliceTraits!(i32, 8);
ImplDekuSliceTraits!(i32, 9);
ImplDekuSliceTraits!(i32, 10);
ImplDekuSliceTraits!(i32, 11);
ImplDekuSliceTraits!(i32, 12);
ImplDekuSliceTraits!(i32, 13);
ImplDekuSliceTraits!(i32, 14);
ImplDekuSliceTraits!(i32, 15);
ImplDekuSliceTraits!(i32, 16);
ImplDekuSliceTraits!(i32, 17);
ImplDekuSliceTraits!(i32, 18);
ImplDekuSliceTraits!(i32, 19);
ImplDekuSliceTraits!(i32, 20);
ImplDekuSliceTraits!(i32, 21);
ImplDekuSliceTraits!(i32, 22);
ImplDekuSliceTraits!(i32, 23);
ImplDekuSliceTraits!(i32, 24);
ImplDekuSliceTraits!(i32, 25);
ImplDekuSliceTraits!(i32, 26);
ImplDekuSliceTraits!(i32, 27);
ImplDekuSliceTraits!(i32, 28);
ImplDekuSliceTraits!(i32, 29);
ImplDekuSliceTraits!(i32, 30);
ImplDekuSliceTraits!(i32, 31);
ImplDekuSliceTraits!(i32, 32);
ImplDekuSliceTraits!(i64, 1);
ImplDekuSliceTraits!(i64, 2);
ImplDekuSliceTraits!(i64, 3);
ImplDekuSliceTraits!(i64, 4);
ImplDekuSliceTraits!(i64, 5);
ImplDekuSliceTraits!(i64, 6);
ImplDekuSliceTraits!(i64, 7);
ImplDekuSliceTraits!(i64, 8);
ImplDekuSliceTraits!(i64, 9);
ImplDekuSliceTraits!(i64, 10);
ImplDekuSliceTraits!(i64, 11);
ImplDekuSliceTraits!(i64, 12);
ImplDekuSliceTraits!(i64, 13);
ImplDekuSliceTraits!(i64, 14);
ImplDekuSliceTraits!(i64, 15);
ImplDekuSliceTraits!(i64, 16);
ImplDekuSliceTraits!(i64, 17);
ImplDekuSliceTraits!(i64, 18);
ImplDekuSliceTraits!(i64, 19);
ImplDekuSliceTraits!(i64, 20);
ImplDekuSliceTraits!(i64, 21);
ImplDekuSliceTraits!(i64, 22);
ImplDekuSliceTraits!(i64, 23);
ImplDekuSliceTraits!(i64, 24);
ImplDekuSliceTraits!(i64, 25);
ImplDekuSliceTraits!(i64, 26);
ImplDekuSliceTraits!(i64, 27);
ImplDekuSliceTraits!(i64, 28);
ImplDekuSliceTraits!(i64, 29);
ImplDekuSliceTraits!(i64, 30);
ImplDekuSliceTraits!(i64, 31);
ImplDekuSliceTraits!(i64, 32);
ImplDekuSliceTraits!(i128, 1);
ImplDekuSliceTraits!(i128, 2);
ImplDekuSliceTraits!(i128, 3);
ImplDekuSliceTraits!(i128, 4);
ImplDekuSliceTraits!(i128, 5);
ImplDekuSliceTraits!(i128, 6);
ImplDekuSliceTraits!(i128, 7);
ImplDekuSliceTraits!(i128, 8);
ImplDekuSliceTraits!(i128, 9);
ImplDekuSliceTraits!(i128, 10);
ImplDekuSliceTraits!(i128, 11);
ImplDekuSliceTraits!(i128, 12);
ImplDekuSliceTraits!(i128, 13);
ImplDekuSliceTraits!(i128, 14);
ImplDekuSliceTraits!(i128, 15);
ImplDekuSliceTraits!(i128, 16);
ImplDekuSliceTraits!(i128, 17);
ImplDekuSliceTraits!(i128, 18);
ImplDekuSliceTraits!(i128, 19);
ImplDekuSliceTraits!(i128, 20);
ImplDekuSliceTraits!(i128, 21);
ImplDekuSliceTraits!(i128, 22);
ImplDekuSliceTraits!(i128, 23);
ImplDekuSliceTraits!(i128, 24);
ImplDekuSliceTraits!(i128, 25);
ImplDekuSliceTraits!(i128, 26);
ImplDekuSliceTraits!(i128, 27);
ImplDekuSliceTraits!(i128, 28);
ImplDekuSliceTraits!(i128, 29);
ImplDekuSliceTraits!(i128, 30);
ImplDekuSliceTraits!(i128, 31);
ImplDekuSliceTraits!(i128, 32);
ImplDekuSliceTraits!(isize, 1);
ImplDekuSliceTraits!(isize, 2);
ImplDekuSliceTraits!(isize, 3);
ImplDekuSliceTraits!(isize, 4);
ImplDekuSliceTraits!(isize, 5);
ImplDekuSliceTraits!(isize, 6);
ImplDekuSliceTraits!(isize, 7);
ImplDekuSliceTraits!(isize, 8);
ImplDekuSliceTraits!(isize, 9);
ImplDekuSliceTraits!(isize, 10);
ImplDekuSliceTraits!(isize, 11);
ImplDekuSliceTraits!(isize, 12);
ImplDekuSliceTraits!(isize, 13);
ImplDekuSliceTraits!(isize, 14);
ImplDekuSliceTraits!(isize, 15);
ImplDekuSliceTraits!(isize, 16);
ImplDekuSliceTraits!(isize, 17);
ImplDekuSliceTraits!(isize, 18);
ImplDekuSliceTraits!(isize, 19);
ImplDekuSliceTraits!(isize, 20);
ImplDekuSliceTraits!(isize, 21);
ImplDekuSliceTraits!(isize, 22);
ImplDekuSliceTraits!(isize, 23);
ImplDekuSliceTraits!(isize, 24);
ImplDekuSliceTraits!(isize, 25);
ImplDekuSliceTraits!(isize, 26);
ImplDekuSliceTraits!(isize, 27);
ImplDekuSliceTraits!(isize, 28);
ImplDekuSliceTraits!(isize, 29);
ImplDekuSliceTraits!(isize, 30);
ImplDekuSliceTraits!(isize, 31);
ImplDekuSliceTraits!(isize, 32);
ImplDekuSliceTraits!(f32, 1);
ImplDekuSliceTraits!(f32, 2);
ImplDekuSliceTraits!(f32, 3);
ImplDekuSliceTraits!(f32, 4);
ImplDekuSliceTraits!(f32, 5);
ImplDekuSliceTraits!(f32, 6);
ImplDekuSliceTraits!(f32, 7);
ImplDekuSliceTraits!(f32, 8);
ImplDekuSliceTraits!(f32, 9);
ImplDekuSliceTraits!(f32, 10);
ImplDekuSliceTraits!(f32, 11);
ImplDekuSliceTraits!(f32, 12);
ImplDekuSliceTraits!(f32, 13);
ImplDekuSliceTraits!(f32, 14);
ImplDekuSliceTraits!(f32, 15);
ImplDekuSliceTraits!(f32, 16);
ImplDekuSliceTraits!(f32, 17);
ImplDekuSliceTraits!(f32, 18);
ImplDekuSliceTraits!(f32, 19);
ImplDekuSliceTraits!(f32, 20);
ImplDekuSliceTraits!(f32, 21);
ImplDekuSliceTraits!(f32, 22);
ImplDekuSliceTraits!(f32, 23);
ImplDekuSliceTraits!(f32, 24);
ImplDekuSliceTraits!(f32, 25);
ImplDekuSliceTraits!(f32, 26);
ImplDekuSliceTraits!(f32, 27);
ImplDekuSliceTraits!(f32, 28);
ImplDekuSliceTraits!(f32, 29);
ImplDekuSliceTraits!(f32, 30);
ImplDekuSliceTraits!(f32, 31);
ImplDekuSliceTraits!(f32, 32);
ImplDekuSliceTraits!(f64, 1);
ImplDekuSliceTraits!(f64, 2);
ImplDekuSliceTraits!(f64, 3);
ImplDekuSliceTraits!(f64, 4);
ImplDekuSliceTraits!(f64, 5);
ImplDekuSliceTraits!(f64, 6);
ImplDekuSliceTraits!(f64, 7);
ImplDekuSliceTraits!(f64, 8);
ImplDekuSliceTraits!(f64, 9);
ImplDekuSliceTraits!(f64, 10);
ImplDekuSliceTraits!(f64, 11);
ImplDekuSliceTraits!(f64, 12);
ImplDekuSliceTraits!(f64, 13);
ImplDekuSliceTraits!(f64, 14);
ImplDekuSliceTraits!(f64, 15);
ImplDekuSliceTraits!(f64, 16);
ImplDekuSliceTraits!(f64, 17);
ImplDekuSliceTraits!(f64, 18);
ImplDekuSliceTraits!(f64, 19);
ImplDekuSliceTraits!(f64, 20);
ImplDekuSliceTraits!(f64, 21);
ImplDekuSliceTraits!(f64, 22);
ImplDekuSliceTraits!(f64, 23);
ImplDekuSliceTraits!(f64, 24);
ImplDekuSliceTraits!(f64, 25);
ImplDekuSliceTraits!(f64, 26);
ImplDekuSliceTraits!(f64, 27);
ImplDekuSliceTraits!(f64, 28);
ImplDekuSliceTraits!(f64, 29);
ImplDekuSliceTraits!(f64, 30);
ImplDekuSliceTraits!(f64, 31);
ImplDekuSliceTraits!(f64, 32);

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;

    #[cfg(target_endian = "little")]
    static IS_LE: bool = true;

    #[cfg(target_endian = "big")]
    static IS_LE: bool = false;

    #[rstest(input,input_is_le,expected,expected_rest,
        case::normal_le([0xDD, 0xCC, 0xBB, 0xAA].as_ref(), IS_LE, [0xCCDD, 0xAABB], bits![Msb0, u8;]),
        case::normal_be([0xDD, 0xCC, 0xBB, 0xAA].as_ref(), !IS_LE, [0xDDCC, 0xBBAA], bits![Msb0, u8;]),
    )]
    fn test_bit_read(
        input: &[u8],
        input_is_le: bool,
        expected: [u16; 2],
        expected_rest: &BitSlice<Msb0, u8>,
    ) {
        let bit_slice = input.bits::<Msb0>();

        let (rest, res_read) = <[u16; 2]>::read(bit_slice, input_is_le).unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, rest);
    }

    #[rstest(input,output_is_le,expected,
        case::normal_le([0xDDCC, 0xBBAA], IS_LE, vec![0xCC, 0xDD, 0xAA, 0xBB]),
        case::normal_be([0xDDCC, 0xBBAA], !IS_LE, vec![0xDD, 0xCC, 0xBB, 0xAA]),
    )]
    fn test_bit_write(input: [u16; 2], output_is_le: bool, expected: Vec<u8>) {
        let res_write = input.write(output_is_le).unwrap().into_vec();
        assert_eq!(expected, res_write);
    }
}
