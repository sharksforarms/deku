/*! Crate prelude

[What is a prelude?](std::prelude)
*/
pub use crate::error::{DekuError, NeedSize};
pub use crate::{
    deku_derive, DekuContainerRead, DekuContainerWrite, DekuEnumExt, DekuRead, DekuUpdate,
    DekuWrite,
};
