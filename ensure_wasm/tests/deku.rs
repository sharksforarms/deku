#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;
use wasm_bindgen_test::*;

use ensure_wasm::*;

use deku::prelude::*;

#[wasm_bindgen_test]
fn test_read() {
    assert_eq!(
        DekuTest {
            field_a: 0b10101,
            field_b: 0b101,
            field_c: 0xBE
        },
        deku_read(&mut [0b10101_101, 0xBE])
    )
}

#[wasm_bindgen_test]
fn test_write() {
    assert_eq!(
        vec![0b10101_101, 0xBE],
        DekuTest {
            field_a: 0b10101,
            field_b: 0b101,
            field_c: 0xBE
        }
        .to_bytes()
        .unwrap()
    )
}
