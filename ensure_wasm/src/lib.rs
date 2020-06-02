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

#[wasm_bindgen]
pub fn deku_read(input: &str) -> String {
    let data = hex::decode(input).unwrap();
    let (_rest, val) = DekuTest::from_bytes((&data, 0)).unwrap();

    return format!("{:?}", val);
}
