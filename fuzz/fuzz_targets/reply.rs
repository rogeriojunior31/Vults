//! Any bytes the hook could read back as the app's answer: never a panic (rule 1).
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|line: &[u8]| {
    let _ = serde_json::from_slice::<vults_protocol::Reply>(line);
});
