mod bool;
mod ipaddr;
mod nonzero;
mod option;
mod primitive;
mod slice;
mod tuple;
mod unit;
mod vec;

#[cfg(feature = "alloc")]
mod arc;

#[cfg(feature = "alloc")]
mod cow;

#[cfg(feature = "alloc")]
mod cstring;

#[cfg(feature = "std")]
mod hashmap;

#[cfg(feature = "std")]
mod hashset;

#[cfg(feature = "alloc")]
mod boxed;
