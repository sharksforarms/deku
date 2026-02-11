use deku::prelude::*;

#[derive(Debug, DekuRead, DekuWrite, PartialEq)]
pub struct Test {
    #[deku(update_also, update_ctx = "self.val.len() as u16, 0")]
    hdr: Hdr,

    #[deku(count = "hdr.length")]
    val: Vec<u8>,

    #[deku(update_also)]
    no_update_ctx: NoUpdateCtx,

    #[deku(update_with = "Self::custom")]
    num: u8,

    #[deku(update_with = "Self::other_custom", update_ctx = "0x10u8,")]
    other_num: (u8, u32),
}

impl Test {
    fn custom(num: &mut u8, _ctx: ()) -> Result<(), DekuError> {
        *num = 1;

        Ok(())
    }

    fn other_custom(num: &mut (u8, u32), ctx: (u8,)) -> Result<(), DekuError> {
        *num = (0xf0, ctx.0 as u32);

        Ok(())
    }
}

#[derive(Debug, DekuRead, DekuWrite, PartialEq)]
#[deku(update_ctx = "val_len: u16, _na: u8")]
struct Hdr {
    #[deku(update = "val_len")]
    length: u8,
}

#[derive(Debug, DekuRead, DekuWrite, PartialEq)]
struct NoUpdateCtx {
    #[deku(update = "0xff")]
    val: u8,
}

fn main() {
    let mut test = Test {
        hdr: Hdr { length: 2 },
        val: vec![1, 2],
        no_update_ctx: NoUpdateCtx { val: 0 },
        num: 0,
        other_num: (0, 0),
    };

    test.val = vec![1, 2, 3];
    test.update(()).unwrap();

    let expected = Test {
        hdr: Hdr { length: 3 },
        val: test.val.clone(),
        no_update_ctx: NoUpdateCtx { val: 0xff },
        num: 1,
        other_num: (0xf0, 0x10),
    };
    assert_eq!(expected, test);
}
