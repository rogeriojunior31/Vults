//! Which vulture each session's bird is. The flock draws from the vultures of Brazil: four by a
//! hash of the session id and the season, the fifth (the king vulture) by role, so it stays rare.
//! The season is a number the app picks at start-up: a new one gives a new flock, while a session
//! keeps its bird for as long as it lives. Ids name the renderer's species
//! (`ui/src/character/flock/species.ts`).

use std::collections::BTreeMap;

use crate::{Session, SessionKey};

/// What a session's bird is drawn from.
pub const POOL: [&str; 4] = ["atratus", "aura", "burrovianus", "melambrotus"];
/// The king vulture: the oldest session of a project with at least `KING_FLOCK` sessions. The
/// crown stays with it however the sessions take turns at the front.
pub const KING: &str = "papa";
pub const KING_FLOCK: usize = 3;

/// FNV-1a over the season and the session id: stable for a session, new with each season.
fn hash(season: u64, id: &str) -> u64 {
    season
        .to_le_bytes()
        .iter()
        .chain(id.as_bytes())
        .fold(0xcbf2_9ce4_8422_2325, |h, b| {
            (h ^ u64::from(*b)).wrapping_mul(0x100_0000_01b3)
        })
}

/// The species a session draws, before any role.
pub fn drawn(season: u64, id: &str) -> &'static str {
    POOL[(hash(season, id) % POOL.len() as u64) as usize]
}

/// Every session's species. A session with no project yet (no folder seen) joins no flock.
pub fn species<'a>(
    season: u64,
    sessions: impl IntoIterator<Item = &'a Session>,
) -> BTreeMap<&'a SessionKey, &'static str> {
    let mut out = BTreeMap::new();
    let mut projects: BTreeMap<&str, Vec<&Session>> = BTreeMap::new();
    for s in sessions {
        out.insert(&s.key, drawn(season, &s.key.session_id));
        if !s.project.is_empty() {
            projects.entry(s.project.as_str()).or_default().push(s);
        }
    }
    for group in projects.values().filter(|g| g.len() >= KING_FLOCK) {
        if let Some(oldest) = group.iter().min_by_key(|s| (s.started, &s.key)) {
            out.insert(&oldest.key, KING);
        }
    }
    out
}
