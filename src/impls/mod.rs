mod bool;
mod nonzero;
mod option;
mod primitive;
mod slice;
mod tuple;
mod unit;
mod vec;

#[cfg(feature = "std")]
mod cow;

#[cfg(feature = "std")]
mod cstring;

#[cfg(feature = "std")]
mod ipaddr;

#[cfg(feature = "alloc")]
mod boxed;
