//! A field with `assert` or `assert_eq` joins a run, which checks the
//! assertion once it has read the field and before it writes it. The
//! errors are those of the field's own read and write.
#![cfg(all(feature = "alloc", feature = "bits"))]

use deku::prelude::*;

macro_rules! checked {
    ($name:ident, $($extra:tt)*) => {
        #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
        #[deku(endian = "big")]
        struct $name {
            #[deku(bits = 6, assert_eq = "18" $($extra)*)]
            opcode: u8,
            #[deku(bits = 24 $($extra)*)]
            offset: u32,
            #[deku(bits = 2, assert = "*link <= 1" $($extra)*)]
            link: u8,
        }
    };
}

checked!(Batched,);
checked!(Unbatched, , seek_from_current = "0");

macro_rules! checked_lsb {
    ($name:ident, $($extra:tt)*) => {
        #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
        #[deku(endian = "little", bit_order = "lsb")]
        struct $name {
            #[deku(bits = 24 $($extra)*)]
            offset: u32,
            #[deku(bits = 8, assert_eq = "0xeb" $($extra)*)]
            opcode: u8,
        }
    };
}

checked_lsb!(LsbBatched,);
checked_lsb!(LsbUnbatched, , seek_from_current = "0");

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
struct Bytes {
    #[deku(assert_eq = "0xcafe")]
    magic: u16,
    #[deku(assert = "*length < 100")]
    length: u32,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
struct BytesUnbatched {
    #[deku(assert_eq = "0xcafe", seek_from_current = "0")]
    magic: u16,
    #[deku(assert = "*length < 100", seek_from_current = "0")]
    length: u32,
}

fn wire(len: usize, seed: u32) -> Vec<u8> {
    let mut x = seed;
    (0..len)
        .map(|_| {
            x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (x >> 24) as u8
        })
        .collect()
}

/// Each read gives the value or the error the field's own reads give.
#[test]
fn an_assertion_in_a_run_accepts_and_refuses_as_its_field() {
    for seed in 1..20_000u32 {
        let mut data = wire(4, seed);
        // Make about half the words pass the opcode.
        if seed % 2 == 0 {
            data[0] = 18 << 2 | (data[0] & 3);
        }
        let batched = Batched::from_bytes((&data, 0)).map(|(_, v)| v);
        let plain = Unbatched::from_bytes((&data, 0)).map(|(_, v)| v);
        assert_eq!(
            format!("{batched:?}").replace("Batched", ""),
            format!("{plain:?}").replace("Unbatched", ""),
            "seed {seed}"
        );

        if seed % 2 == 0 {
            data[3] = 0xeb;
        }
        let batched = LsbBatched::from_bytes((&data, 0)).map(|(_, v)| v);
        let plain = LsbUnbatched::from_bytes((&data, 0)).map(|(_, v)| v);
        assert_eq!(
            format!("{batched:?}").replace("Batched", ""),
            format!("{plain:?}").replace("Unbatched", ""),
            "seed {seed}"
        );

        let mut bytes = wire(6, seed);
        if seed % 2 == 0 {
            bytes[..2].copy_from_slice(&[0xca, 0xfe]);
            bytes[2..5].fill(0);
        }
        let batched = Bytes::from_bytes((&bytes, 0)).map(|(_, v)| v);
        let plain = BytesUnbatched::from_bytes((&bytes, 0)).map(|(_, v)| v);
        assert_eq!(
            format!("{batched:?}").replace("Batched", ""),
            format!("{plain:?}").replace("Unbatched", ""),
            "seed {seed}"
        );
    }
}

#[test]
fn an_assertion_in_a_run_refuses_a_write_as_its_field() {
    let good = Batched {
        opcode: 18,
        offset: 5,
        link: 1,
    };
    assert_eq!(
        good.to_bytes().unwrap(),
        Unbatched {
            opcode: 18,
            offset: 5,
            link: 1
        }
        .to_bytes()
        .unwrap()
    );
    for (opcode, link) in [(17, 1), (18, 2)] {
        let batched = Batched {
            opcode,
            offset: 5,
            link,
        }
        .to_bytes();
        let plain = Unbatched {
            opcode,
            offset: 5,
            link,
        }
        .to_bytes();
        assert!(batched.is_err());
        assert_eq!(
            format!("{batched:?}").replace("Batched", ""),
            format!("{plain:?}").replace("Unbatched", "")
        );
    }
    let batched = LsbBatched {
        offset: 1,
        opcode: 0xea,
    }
    .to_bytes();
    let plain = LsbUnbatched {
        offset: 1,
        opcode: 0xea,
    }
    .to_bytes();
    assert!(batched.is_err());
    assert_eq!(
        format!("{batched:?}").replace("Batched", ""),
        format!("{plain:?}").replace("Unbatched", "")
    );
}
