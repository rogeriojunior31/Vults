#![allow(clippy::unwrap_used)]
use std::path::{Path, PathBuf};
use std::process::Command;
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/meeting-eval")
        .join(name)
}
#[test]
fn executable_scores_both_language_tags_without_audio_and_rejects_foreign_clocks() {
    let binary = env!("CARGO_BIN_EXE_vults-meeting-eval");
    for (language, expected) in [("en", 0.0), ("pt", 0.4)] {
        let result = Command::new(binary)
            .arg(fixture(&format!("{language}-reference.json")))
            .arg(fixture(&format!("{language}-prediction.json")))
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(report["der"], expected);
        assert_eq!(report["language"], language);
        assert_eq!(report["annotation"], "synthetic");
        assert_eq!(report["productionQualified"], false);
    }
    let mismatch = Command::new(binary)
        .arg(fixture("en-reference.json"))
        .arg(fixture("pt-prediction.json"))
        .output()
        .unwrap();
    assert!(!mismatch.status.success());
    assert!(mismatch.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&mismatch.stderr)
            .contains("same schema, recording, source, WAV hash and duration")
    );
    let usage = Command::new(binary).output().unwrap();
    assert!(!usage.status.success());
    assert!(usage.stdout.is_empty());
}
