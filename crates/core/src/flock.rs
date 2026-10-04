//! Which vulture each session's bird is. The flock draws by a hash of the session id and the
//! season from the pool the user chose (Brazil's vultures by default); the king vulture comes by
//! role, so it stays rare. The season is a number the app picks at start-up: a new one gives a new
//! flock, while a session keeps its bird for as long as it lives. Ids name the renderer's species
//! (`ui/src/character/flock/species.ts`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{Session, SessionKey};

/// Brazil's vultures, the king aside.
pub const POOL: [&str; 4] = ["atratus", "aura", "burrovianus", "melambrotus"];
/// The vultures of the Americas: Brazil's, and the two condors.
const AMERICAS: [&str; 6] = [
    "atratus",
    "aura",
    "burrovianus",
    "melambrotus",
    "vultur",
    "gymnogyps",
];
/// Every vulture in the world, the king aside.
const WORLD: [&str; 22] = [
    "atratus",
    "aura",
    "burrovianus",
    "melambrotus",
    "vultur",
    "gymnogyps",
    "neophron",
    "gypaetus",
    "gypohierax",
    "necrosyrtes",
    "gyps-fulvus",
    "gyps-rueppelli",
    "gyps-coprotheres",
    "gyps-himalayensis",
    "gyps-africanus",
    "gyps-indicus",
    "gyps-tenuirostris",
    "gyps-bengalensis",
    "aegypius",
    "torgos",
    "sarcogyps",
    "trigonoceps",
];

/// Where the flock draws from: the user's choice in the settings.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Flock {
    #[default]
    Brazil,
    Americas,
    World,
}

impl Flock {
    pub fn pool(self) -> &'static [&'static str] {
        match self {
            Flock::Brazil => &POOL,
            Flock::Americas => &AMERICAS,
            Flock::World => &WORLD,
        }
    }
}
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

/// The splitmix64 finalizer. FNV's low bits depend only on the inputs' low bits, so `% 4` on the
/// raw hash would give every set of sessions just four flocks, whatever the season.
fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// The species a session draws from a pool, before any role.
pub fn drawn(pool: &[&'static str], season: u64, id: &str) -> &'static str {
    pool[(mix(hash(season, id)) % pool.len() as u64) as usize]
}

/// Every session's species. A session with no project yet (no folder seen) joins no flock.
pub fn species<'a>(
    flock: Flock,
    season: u64,
    sessions: impl IntoIterator<Item = &'a Session>,
) -> BTreeMap<&'a SessionKey, &'static str> {
    let mut out = BTreeMap::new();
    let mut projects: BTreeMap<&str, Vec<&Session>> = BTreeMap::new();
    for s in sessions {
        out.insert(&s.key, drawn(flock.pool(), season, &s.key.session_id));
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
