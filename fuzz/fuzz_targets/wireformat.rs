#![no_main]

use libfuzzer_sys::fuzz_target;

#[path = "wireformat_body.rs"]
mod body;

fuzz_target!(|data: &[u8]| {
    body::exercise(data);
});
