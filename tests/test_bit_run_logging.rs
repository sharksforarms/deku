//! A batched run must trace every field it serves, as the reads and writes it
//! replaces each did.
//!
//! Its own binary: the logger is global, so a capture here would otherwise pick
//! up every other test in the same process.
#![cfg(all(feature = "logging", feature = "alloc", feature = "bits"))]

use deku::prelude::*;
use std::sync::Mutex;

static LINES: Mutex<Vec<String>> = Mutex::new(Vec::new());

struct Capture;

impl log::Log for Capture {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let line = format!("{}", record.args());
        if line.starts_with("Reading:") || line.starts_with("Writing:") {
            LINES.lock().unwrap().push(line);
        }
    }

    fn flush(&self) {}
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
struct Batched {
    #[deku(bits = 12)]
    a: u16,
    #[deku(bits = 4)]
    b: u8,
}

#[test]
fn a_run_traces_every_field_it_serves() {
    log::set_logger(&Capture).unwrap();
    log::set_max_level(log::LevelFilter::Trace);

    let (_, v) = Batched::from_bytes((&[0xAB, 0xCD], 0)).unwrap();
    v.to_bytes().unwrap();

    assert_eq!(
        *LINES.lock().unwrap(),
        [
            "Reading: Batched.a",
            "Reading: Batched.b",
            "Writing: Batched.a",
            "Writing: Batched.b",
        ]
    );
}
