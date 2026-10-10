//! Any bytes a hook (or anything else on the socket) could send: decoding never panics, and an
//! event that decodes encodes back to one that decodes the same.
#![no_main]

use libfuzzer_sys::fuzz_target;
use vults_protocol::{decode_event, encode};

fuzz_target!(|line: &[u8]| {
    if let Ok(event) = decode_event(line) {
        let wire = encode(&event);
        let again = decode_event(&wire[..wire.len() - 1]).expect("an encoded event decodes");
        assert_eq!(again, event);
    }
});
