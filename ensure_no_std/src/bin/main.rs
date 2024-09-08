//! cargo build --target thumbv7em-none-eabihf
#![no_std]
#![no_main]

extern crate alloc;

use core::panic::PanicInfo;

use cortex_m_rt::entry;
use embedded_alloc::LlffHeap as Heap;

#[global_allocator]
static HEAP: Heap = Heap::empty();

use alloc::{format, vec, vec::Vec};
use deku::prelude::*;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuTest {
    #[deku(bits = 5)]
    field_a: u8,
    #[deku(bits = 3)]
    field_b: u8,
    count: u8,
    #[deku(count = "count", pad_bytes_after = "8")]
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
    let test_data: &[u8] = &[0b10101_101, 0x02, 0xBE, 0xEF, 0xff];
    let mut cursor = deku::no_std_io::Cursor::new(test_data);

    // Test reading
    let (_rest, val) = DekuTest::from_reader((&mut cursor, 0)).unwrap();
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
    let _val = val.to_bytes().unwrap();

    loop { /* .. */ }
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}
