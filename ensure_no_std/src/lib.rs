#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod no_alloc_imports;
#[cfg(feature = "alloc")]
pub mod with_alloc_imports;
