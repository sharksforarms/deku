/*! Crate prelude

[What is a prelude?](std::prelude)
*/
pub use crate::error::DekuError;

pub use crate::error::NeedSize;
pub use crate::{
    deku_derive, reader::Reader, writer::Writer, DekuContainerRead, DekuContainerWrite,
    DekuEnumExt, DekuRead, DekuReader, DekuUpdate, DekuWrite, DekuWriter,
};
