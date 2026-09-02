//! cargo build --target thumbv7em-none-eabihf
#![no_std]
#![no_main]

use no_std_lib::*;

use core::panic::PanicInfo;

use cortex_m_rt::entry;

#[entry]
fn main() -> ! {
    // Initialize the allocator BEFORE you use it
    #[cfg(feature = "alloc")]
    {
        use embedded_alloc::LlffHeap as Heap;
        use core::mem::MaybeUninit;

        #[global_allocator]
        static HEAP: Heap = Heap::empty();
        const HEAP_SIZE: usize = 1024;
        static HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
    }

    // now the allocator is ready types like Box, Vec can be used.

    no_alloc_imports::rw();
    #[cfg(feature = "alloc")]
    with_alloc_imports::rw();

    loop { /* .. */ }
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}
