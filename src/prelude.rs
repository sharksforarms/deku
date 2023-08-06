/*! Crate prelude

[What is a prelude?](std::prelude)
*/
pub use crate::error::{DekuError, NeedSize};
pub use crate::{
    container::Container, deku_derive, DekuContainerRead, DekuContainerWrite, DekuEnumExt,
    DekuRead, DekuReader, DekuUpdate, DekuWrite,
};
