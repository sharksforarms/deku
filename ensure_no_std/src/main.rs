//! cargo build --target thumbv7em-none-eabihf
#![no_std]
#![no_main]

extern crate alloc;

use no_std_lib::*;

use core::panic::PanicInfo;

use cortex_m_rt::entry;
use embedded_alloc::LlffHeap as Heap;

#[global_allocator]
static HEAP: Heap = Heap::empty();

#[entry]
fn main() -> ! {
    // Initialize the allocator BEFORE you use it
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 1024;
        static HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
    }

    // now the allocator is ready types like Box, Vec can be used.

    no_alloc_imports::rw();
    with_alloc_imports::rw();

    loop { /* .. */ }
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}
