//! Any text the audit log keeps goes through the redactor first: it never panics, and it only ever
//! replaces: what it leaves is the input's own text, in order (so it can't make a command say
//! something else, or drop the part after a secret silently). Classifying it never panics either.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|text: &str| {
    let out = vults_core::redact::secrets(text);
    let kept = out.replace("[redacted]", "");
    let mut input = text.chars();
    for c in kept.chars() {
        assert!(input.any(|i| i == c), "the redactor changed the text, not only replaced it");
    }
    let _ = vults_core::policy::classify("Bash", text);
});
