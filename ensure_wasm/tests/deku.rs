#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;
use wasm_bindgen_test::*;

use ensure_wasm::deku_read;

#[wasm_bindgen_test]
fn test_read() {
    assert_eq!("DekuTest { field_a: 21, field_b: 5, count: 2, data: [190, 239] }".to_string(), deku_read("ad02beef"))
}
