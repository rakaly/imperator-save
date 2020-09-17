#![no_main]
use libfuzzer_sys::fuzz_target;
use std::io::Cursor;

fuzz_target!(|data: &[u8]| {
    let _ = imperator_save::ImperatorExtractor::extract_header(data);
    let _ = imperator_save::ImperatorExtractor::extract_save(Cursor::new(data));
});
