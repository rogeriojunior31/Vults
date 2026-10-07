//! Portuguese phonemes from the system's `espeak-ng`, run as a separate program: plain text in,
//! IPA text out. It is GPL-3.0, so it is never linked, loaded or shipped with us (ADR 0013); the
//! user installs it.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

/// A clause takes 10 to 15 ms; anything near this is a stuck program.
const TIMEOUT: Duration = Duration::from_secs(3);

/// What to tell the user when Portuguese has no espeak-ng.
pub const MISSING: &str = "Portuguese needs espeak-ng";

/// `espeak-ng` on the PATH, if installed.
pub fn find() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join("espeak-ng"))
        .find(|p| p.is_file())
}

/// Kokoro's letters for espeak's tied diphthongs and affricates (as misaki maps them).
const MERGES: &[(&str, &str)] = &[
    ("a^\u{26a}", "I"),
    ("a^\u{28a}", "W"),
    ("d^z", "\u{2a3}"),
    ("d^\u{292}", "\u{2a4}"),
    ("e^\u{26a}", "A"),
    ("o^\u{28a}", "O"),
    ("\u{259}^\u{28a}", "Q"),
    ("s^s", "S"),
    ("t^s", "\u{2a6}"),
    ("t^\u{283}", "\u{2a7}"),
    ("\u{254}^\u{26a}", "Y"),
];

/// IPA for `text` in espeak's `voice` (`pt-br`). Clause by clause, so the punctuation, which
/// espeak drops and Kokoro reads as pauses and intonation, stays in place.
pub fn ipa(bin: &Path, voice: &str, text: &str) -> Result<String, String> {
    let mut out = String::new();
    let mut clause = String::new();
    for c in text.chars().chain(std::iter::once('\n')) {
        if ",.!?;:\n".contains(c) {
            if !clause.trim().is_empty() {
                out.push_str(run(bin, voice, clause.trim())?.trim());
            }
            if c != '\n' {
                out.push(c);
                out.push(' ');
            }
            clause.clear();
        } else {
            clause.push(c);
        }
    }
    Ok(out.trim().to_string())
}

fn run(bin: &Path, voice: &str, text: &str) -> Result<String, String> {
    // The text goes on stdin: a sentence starting with "-" is never read as an option.
    let mut child = Command::new(bin)
        .args(["-q", "--ipa", "--tie=^", "-v", voice, "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("can't run espeak-ng: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|e| format!("espeak-ng: {e}"))?;
    }
    let mut stdout = child.stdout.take().ok_or("espeak-ng: no output")?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut s = String::new();
        let _ = tx.send(stdout.read_to_string(&mut s).map(|_| s));
    });
    let Ok(read) = rx.recv_timeout(TIMEOUT) else {
        let _ = child.kill();
        let _ = child.wait();
        return Err("espeak-ng did not answer".into());
    };
    let status = child.wait().map_err(|e| e.to_string())?;
    let text = read.map_err(|e| format!("espeak-ng: {e}"))?;
    if !status.success() {
        return Err(format!("espeak-ng failed ({status})"));
    }
    let mut p = text.replace('\n', " ");
    for (tied, letter) in MERGES {
        p = p.replace(tied, letter);
    }
    Ok(p.replace('^', ""))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    /// A stand-in espeak-ng: a shell script with `body`.
    fn mock(name: &str, body: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("speech-espeak-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let bin = dir.join("espeak-ng");
        std::fs::write(&bin, format!("#!/bin/sh\n{body}\n")).expect("write");
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        bin
    }

    #[test]
    fn each_clause_goes_through_espeak_and_the_punctuation_stays() {
        // Echoes the voice and what came in, with a tied affricate to merge.
        let bin = mock("echo", r#"printf '%s t^\312\203 ' "$5"; cat"#);
        let out = ipa(&bin, "pt-br", "Oi, -eu sou o Zeca.").expect("ipa");
        assert_eq!(out, "pt-br \u{2a7} Oi, pt-br \u{2a7} -eu sou o Zeca.");
    }

    #[test]
    fn a_stuck_or_failing_espeak_is_an_error() {
        let slow = mock("slow", "sleep 10");
        let started = std::time::Instant::now();
        assert!(ipa(&slow, "pt-br", "Oi.").is_err());
        assert!(started.elapsed() < Duration::from_secs(6));
        let failing = mock("fail", "exit 1");
        assert!(ipa(&failing, "pt-br", "Oi.").is_err());
        assert!(ipa(Path::new("/nonexistent/espeak-ng"), "pt-br", "Oi.").is_err());
    }
}
