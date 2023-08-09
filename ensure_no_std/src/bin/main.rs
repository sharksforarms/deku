//! cargo build --target thumbv7em-none-eabihf
#![no_std]
#![no_main]

extern crate alloc;

use core::panic::PanicInfo;

use cortex_m_rt::entry;
use embedded_alloc::Heap;

#[global_allocator]
static HEAP: Heap = Heap::empty();

use alloc::{format, vec, vec::Vec};
use deku::prelude::*;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuTest {
    #[deku(bits = "5")]
    field_a: u8,
    #[deku(bits = "3")]
    field_b: u8,
    count: u8,
    #[deku(count = "count")]
    data: Vec<u8>,
}

#[entry]
fn main() -> ! {
    // Initialize the allocator BEFORE you use it
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 1024;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
    }

    // now the allocator is ready types like Box, Vec can be used.

    #[allow(clippy::unusual_byte_groupings)]
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

    loop { /* .. */ }
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}
