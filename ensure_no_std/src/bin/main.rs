//! Based on https://github.com/rustwasm/wee_alloc/tree/master/example
//! Run with `cargo +nightly run --release`

#![no_std]
#![no_main]
#![feature(core_intrinsics, lang_items, alloc_error_handler)]

extern crate alloc;
extern crate wee_alloc;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// Need to provide a tiny `panic` implementation for `#![no_std]`.
// This translates into an `unreachable` instruction that will
// raise a `trap` the WebAssembly execution if we panic at runtime.
#[panic_handler]
#[no_mangle]
pub fn panic(_info: &::core::panic::PanicInfo) -> ! {
    unsafe {
        ::core::intrinsics::abort();
    }
}

// Need to provide an allocation error handler which just aborts
// the execution with trap.
#[alloc_error_handler]
#[no_mangle]
pub extern "C" fn oom(_: ::core::alloc::Layout) -> ! {
    unsafe {
        ::core::intrinsics::abort();
    }
}

// Needed for non-wasm targets.
#[lang = "eh_personality"]
#[no_mangle]
pub extern "C" fn eh_personality() {}

use alloc::{vec, vec::Vec};
use deku::prelude::*;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuTest {
    #[deku(bits = "5")]
    field_a: u8,
    #[deku(bits = "3")]
    field_b: u8,
    count: u8,
    #[deku(len = "count")]
    data: Vec<u8>,
}

#[no_mangle]
pub extern "C" fn main() -> ! {
    loop {
        let test_data: Vec<u8> = vec![0b10101_101, 0x02, 0xBE, 0xEF];

        // Test reading
        let (_rest, val) = DekuTest::from_bytes((&test_data, 0)).unwrap();
        assert_eq!(
            DekuTest {
                field_a: 0b10101,
                field_b: 0b101,
                count: 0x02,
                data: vec![0xBE, 0xEF]
            },
            val
        );

        // Test writing
        let val = val.to_bytes().unwrap();
        assert_eq!(test_data, val);
    }
}
