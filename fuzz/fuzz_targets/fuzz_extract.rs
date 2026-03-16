#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let opts = cloak_core::EmbedOptions::default();
    let _ = cloak_core::extract(data, "fuzz-passphrase", None, &opts);
});
