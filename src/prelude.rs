/*! Crate prelude

[What is a prelude?](std::prelude)
*/
pub use crate::error::{DekuError, NeedSize, NeedMagic};
pub use crate::{
    deku_derive, reader::Reader, writer::Writer, DekuContainerRead, DekuContainerWrite,
    DekuEnumExt, DekuRead, DekuReader, DekuUpdate, DekuWrite, DekuWriter,
};
