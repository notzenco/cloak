#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = cloak_core::crypto::decrypt(data, "fuzz-passphrase");
});
