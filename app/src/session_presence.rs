//! Session liveness is independent of activity: a quiet terminal can remain open for hours.
use std::collections::HashMap;
use std::time::Instant;
use vults_core::SessionKey;

pub type Candidate = (SessionKey, Instant, u32);
pub type Report = (SessionKey, Instant, u32, bool);

#[derive(Default)]
pub struct Watcher(HashMap<SessionKey, (Instant, u32, u64)>);

impl Watcher {
    pub fn read(&mut self, candidates: &[Candidate]) -> Vec<Report> {
        self.0
            .retain(|key, _| candidates.iter().any(|(k, _, _)| k == key));
        candidates
            .iter()
            .filter_map(|(key, event, pid)| {
                let alive = match probe(*pid)? {
                    None => {
                        // A vanished hook wrapper is not proof that its conversation closed.
                        // Only retire an owner this watcher has actually observed alive.
                        if !self.0.get(key).is_some_and(|(_, known_pid, _)| known_pid == pid) {
                            return None;
                        }
                        false
                    }
                    Some(birth) => {
                        let same = self.0.get(key).is_none_or(|(old_event, old_pid, old_birth)| {
                            old_event != event || old_pid != pid || *old_birth == birth
                        });
                        if same {
                            self.0.insert(key.clone(), (*event, *pid, birth));
                        }
                        same
                    }
                };
                Some((key.clone(), *event, *pid, alive))
            })
            .collect()
    }
}

/// Unknown errors never mean closed. A birth token prevents a recycled PID from keeping a ghost.
fn probe(pid: u32) -> Option<Option<u64>> {
    if pid <= 1 {
        return None;
    }
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::MetadataExt;
        let uid = std::fs::metadata("/proc/self").ok()?.uid();
        let path = format!("/proc/{pid}");
        match std::fs::metadata(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Some(None),
            Err(_) => return None,
            Ok(meta) if meta.uid() != uid => return None,
            Ok(_) => {}
        }
        let stat = match std::fs::read_to_string(format!("{path}/stat")) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Some(None),
            Err(_) => return None,
        };
        let fields: Vec<_> = stat.rsplit_once(')')?.1.split_whitespace().collect();
        if matches!(*fields.first()?, "Z" | "X" | "x") {
            return Some(None);
        }
        Some(Some(fields.get(19)?.parse().ok()?))
    }
    #[cfg(not(target_os = "linux"))]
    None
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    #[test]
    fn a_real_open_process_is_present_and_its_exit_is_confirmed() {
        let mut child = std::process::Command::new("sleep").arg("30").spawn().unwrap();
        let candidate = (
            SessionKey {
                agent: vults_core::AgentKind::Claude,
                session_id: "presence-test".into(),
            },
            Instant::now(),
            child.id(),
        );
        let mut watch = Watcher::default();
        assert!(watch.read(std::slice::from_ref(&candidate))[0].3);
        let birth = watch.0.get(&candidate.0).unwrap().2;
        watch.0.get_mut(&candidate.0).unwrap().2 = birth + 1;
        assert!(
            !watch.read(std::slice::from_ref(&candidate))[0].3,
            "PID reuse does not keep the previous owner alive"
        );
        let resumed = (
            candidate.0.clone(),
            candidate.1 + std::time::Duration::from_secs(1),
            candidate.2,
        );
        assert!(
            watch.read(&[resumed])[0].3,
            "A newer native event establishes the resumed owner"
        );
        child.kill().unwrap();
        child.wait().unwrap();
        assert!(!watch.read(&[candidate])[0].3);
        assert_eq!(probe(0), None);
        let unobserved = (
            SessionKey {
                agent: vults_core::AgentKind::Claude,
                session_id: "unobserved".into(),
            },
            Instant::now(),
            u32::MAX,
        );
        assert!(
            Watcher::default().read(&[unobserved]).is_empty(),
            "Unknown ownership never closes a session"
        );
    }
}
