#![allow(missing_docs)]

pub trait FromBeBytes: Sized {
    type Bytes;
    #[allow(dead_code)]
    fn from_be_bytes(_: Self::Bytes) -> Self;
}

macro_rules! implFromBeBytes {(
    $($T:ident),* $(,)?) => ($(
        impl FromBeBytes for $T {
            type Bytes = [u8; ::core::mem::size_of::<$T>()];
            fn from_be_bytes (bytes: Self::Bytes) -> Self {
                #![deny(unconditional_recursion)]
                Self::from_be_bytes(bytes)
            }
        }
   )*
)}

implFromBeBytes![u8, u16, u32, usize, u64, u128, i8, i16, i32, isize, i64, i128, f32, f64,];

/// Converts value to native endian
///
/// Input is assumed to be little endian and the result is swapped if
/// target is big endian.
#[macro_export]
macro_rules! native_endian {
    ($num:expr) => {{
        #[cfg(target_endian = "little")]
        let res = $num;

        #[cfg(target_endian = "big")]
        let res = {
            let mut val = $num;
            let bytes = val.to_le_bytes();
            val = $crate::test_common::FromBeBytes::from_be_bytes(bytes);

            val
        };

        res
    }};
}
