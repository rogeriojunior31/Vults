//! Which vulture each session's bird is. A project's sessions are one flock, and one breed: the
//! species its folder draws from the pool the user chose (Brazil's vultures by default), the same
//! at every start. The projects on the wire take different species while the pool has some left,
//! and a project keeps its species while any of its sessions lives. A session with no folder yet
//! draws by its own id and the season, a number the app picks at start-up. The king vulture comes
//! by role (the oldest of a flock of three or more), so it stays rare. The user may choose a
//! project's species from every one but the king's (`ProjectPrefs::species`): it wins over the
//! draw, and the drawn flocks keep clear of it. Ids name the renderer's species
//! (`ui/src/character/flock/species.ts`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{ProjectPrefs, Session, SessionKey};

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

/// The flock a session belongs to: its project's folder. None before a folder is seen.
pub fn flock_of(s: &Session) -> Option<&str> {
    s.cwd.as_deref().filter(|c| !c.is_empty())
}

/// A species the user may choose for a project: any but the king's, which stays a role.
pub fn chosen(id: &str) -> Option<&'static str> {
    WORLD.iter().copied().find(|s| *s == id)
}

/// The species chosen for a folder, when it is one ([`chosen`]).
fn chosen_for(projects: &BTreeMap<String, ProjectPrefs>, folder: &str) -> Option<&'static str> {
    projects.get(folder)?.species.as_deref().and_then(chosen)
}

/// The breed a folder draws from a pool: the same at every start.
pub fn breed(pool: &[&'static str], folder: &str) -> &'static str {
    drawn(pool, 0, folder)
}

/// Brings each drawn flock's species up to date with the sessions on the wire: a flock that left
/// forgets its own, one whose species left the pool draws again, and a new one takes its breed,
/// or the next species of the pool no flock has while there is one. Flocks that arrived first
/// choose first; a flock never changes species while it lives. A flock whose species the user
/// chose draws nothing, and the others keep clear of it.
pub fn keep<'a>(
    breeds: &mut BTreeMap<String, &'static str>,
    flock: Flock,
    projects: &BTreeMap<String, ProjectPrefs>,
    sessions: impl IntoIterator<Item = &'a Session>,
) {
    let pool = flock.pool();
    let mut arrived: BTreeMap<&str, std::time::Instant> = BTreeMap::new();
    for s in sessions {
        if let Some(folder) = flock_of(s) {
            let first = arrived.entry(folder).or_insert(s.started);
            *first = (*first).min(s.started);
        }
    }
    let picked: Vec<&'static str> = arrived
        .keys()
        .filter_map(|folder| chosen_for(projects, folder))
        .collect();
    arrived.retain(|folder, _| chosen_for(projects, folder).is_none());
    breeds.retain(|folder, species| arrived.contains_key(folder.as_str()) && pool.contains(species));
    let mut new: Vec<(&str, std::time::Instant)> = arrived
        .into_iter()
        .filter(|(folder, _)| !breeds.contains_key(*folder))
        .collect();
    new.sort_by_key(|&(folder, first)| (first, folder));
    for (folder, _) in new {
        let drawn = breed(pool, folder);
        let start = pool.iter().position(|s| *s == drawn).unwrap_or(0);
        let free = (0..pool.len())
            .map(|i| pool[(start + i) % pool.len()])
            .find(|s| !breeds.values().chain(&picked).any(|taken| taken == s));
        breeds.insert(folder.to_string(), free.unwrap_or(drawn));
    }
}

/// Every session's species: the one chosen for its project, else its flock's breed (`breeds`,
/// from [`keep`]), or, with no folder yet, its own draw.
pub fn species<'a>(
    flock: Flock,
    season: u64,
    breeds: &BTreeMap<String, &'static str>,
    projects: &BTreeMap<String, ProjectPrefs>,
    sessions: impl IntoIterator<Item = &'a Session>,
) -> BTreeMap<&'a SessionKey, &'static str> {
    let mut out = BTreeMap::new();
    let mut flocks: BTreeMap<&str, Vec<&Session>> = BTreeMap::new();
    for s in sessions {
        let folder = flock_of(s);
        let own = || drawn(flock.pool(), season, &s.key.session_id);
        out.insert(
            &s.key,
            folder
                .and_then(|f| chosen_for(projects, f).or_else(|| breeds.get(f).copied()))
                .unwrap_or_else(own),
        );
        if let Some(folder) = folder {
            flocks.entry(folder).or_default().push(s);
        }
    }
    for group in flocks.values().filter(|g| g.len() >= KING_FLOCK) {
        if let Some(oldest) = group.iter().min_by_key(|s| (s.started, &s.key)) {
            out.insert(&oldest.key, KING);
        }
    }
    out
}
