//! Several JSON configs reviewed together; each file retains the normal drift check and backup.
use crate::{Error, Preview};
use serde_json::Value;
use std::path::PathBuf;
use std::time::SystemTime;

fn plans(files: &[(PathBuf, Value, Value)]) -> Result<Vec<Preview>, Error> {
    files
        .iter()
        .map(|(path, before, next)| {
            let mut changed = false;
            let plan = crate::preview(path, |current| {
                changed = current != before;
                if changed { current.clone() } else { next.clone() }
            })?;
            if changed {
                return Err(Error::Changed { path: path.clone() });
            }
            Ok(plan)
        })
        .collect()
}
fn bundle(files: &[(PathBuf, Value, Value)], plans: &[Preview]) -> Preview {
    let mut bytes = Vec::new();
    for ((path, _, _), plan) in files.iter().zip(plans) {
        let path = path.as_os_str().as_encoded_bytes();
        bytes.extend_from_slice(&(path.len() as u64).to_le_bytes());
        bytes.extend_from_slice(path);
        bytes.extend_from_slice(plan.fingerprint.as_bytes());
    }
    Preview {
        fingerprint: crate::fingerprint(&bytes),
        diff: plans
            .iter()
            .map(|p| p.diff.as_str())
            .collect::<Vec<_>>()
            .join("\n"),
    }
}
pub fn preview(files: &[(PathBuf, Value, Value)]) -> Result<Preview, Error> {
    Ok(bundle(files, &plans(files)?))
}
pub fn apply(
    files: &[(PathBuf, Value, Value)],
    seen: &str,
    now: SystemTime,
) -> Result<Option<PathBuf>, Error> {
    let plans = plans(files)?;
    if bundle(files, &plans).fingerprint != seen {
        return Err(Error::Changed {
            path: files.first().map(|f| f.0.clone()).unwrap_or_default(),
        });
    }
    let mut first = None;
    for ((path, _, next), plan) in files.iter().zip(plans) {
        let backup = crate::apply(path, &plan.fingerprint, |_| next.clone(), now)?;
        first = first.or(backup);
    }
    Ok(first)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn drift_in_second_config_rejects_before_writing_the_first() {
        let root = std::env::temp_dir().join(format!("vults-json-bundle-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let a = root.join("a.json");
        let b = root.join("b.json");
        std::fs::write(&a, "{}").unwrap();
        std::fs::write(&b, "{}").unwrap();
        let files = vec![
            (
                a.clone(),
                serde_json::json!({}),
                serde_json::json!({"hooks":true}),
            ),
            (
                b.clone(),
                serde_json::json!({}),
                serde_json::json!({"statusLine":true}),
            ),
        ];
        let seen = preview(&files).unwrap().fingerprint;
        std::fs::write(&b, "{\"custom\":true}").unwrap();
        assert!(matches!(
            apply(&files, &seen, SystemTime::now()),
            Err(Error::Changed { .. })
        ));
        assert_eq!(std::fs::read_to_string(a).unwrap(), "{}");
        std::fs::remove_dir_all(root).unwrap();
    }
}
