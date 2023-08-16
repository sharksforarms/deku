/*!
    Based on https://github.com/rustwasm/wasm-pack-template
*/
use wasm_bindgen::prelude::*;

pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

use deku::prelude::*;

#[wasm_bindgen]
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
pub struct DekuTest {
    #[deku(bits = "5")]
    pub field_a: u8,
    #[deku(bits = "3")]
    pub field_b: u8,
    pub field_c: u8,
}

#[wasm_bindgen]
pub fn deku_read(input: &mut [u8]) -> DekuTest {
    let (_rest, val) = DekuTest::from_bytes((input, 0)).unwrap();

    val
}

#[wasm_bindgen]
pub fn deku_write(input: &DekuTest) -> Vec<u8> {
    input.to_bytes().unwrap()
}
