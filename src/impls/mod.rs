mod bool;
mod ipaddr;
mod nonzero;
mod option;
mod primitive;
mod slice;
mod tuple;
mod unit;

#[cfg(feature = "alloc")]
mod vec;

#[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
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
