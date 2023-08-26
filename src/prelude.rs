/*! Crate prelude

[What is a prelude?](std::prelude)
*/
pub use crate::error::{DekuError, NeedSize};
pub use crate::{
    deku_derive, reader::Reader, DekuContainerRead, DekuContainerWrite, DekuEnumExt, DekuRead,
    DekuReader, DekuUpdate, DekuWrite,
};
