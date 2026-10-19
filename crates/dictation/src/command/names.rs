//! Pure name matching and time-bounded clarification; resolving a name grants no authority.
use std::time::Duration;

use super::{MAX_INPUT_BYTES, normalize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NameKind {
    Project,
    Agent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name {
    pub id: String,
    pub name: String,
    pub kind: NameKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    Found(Name),
    Ambiguous(Vec<Name>),
    Unknown,
}

#[derive(Debug)]
struct Entry {
    name: Name,
    key: String,
}

/// An owned snapshot, rebuilt when project/agent names or saved replacements change.
#[derive(Debug)]
pub struct NameResolver {
    entries: Vec<Entry>,
    aliases: Vec<(String, String)>,
}

impl NameResolver {
    /// Only replacements targeting a complete known name become aliases; aliases never chain.
    pub fn new(names: Vec<Name>, replacements: &[(String, String)]) -> Self {
        let entries: Vec<_> = unique(names)
            .into_iter()
            .map(|name| Entry {
                key: normalize(&name.name),
                name,
            })
            .filter(|entry| !entry.key.is_empty())
            .collect();
        let aliases = replacements
            .iter()
            .filter_map(|(from, to)| {
                let from = query_key(from)?;
                let to = normalize(to);
                entries.iter().any(|entry| entry.key == to).then_some((from, to))
            })
            .collect();
        Self { entries, aliases }
    }

    /// Alias, exact name, single whole word, then nearest Unicode Levenshtein match.
    /// Queries of at most four normalized characters allow one edit; longer queries allow two.
    pub fn resolve(&self, text: &str, kind: Option<NameKind>) -> Resolution {
        self.resolve_where(text, |name| kind.is_none_or(|kind| kind == name.kind))
    }

    fn resolve_where(&self, text: &str, includes: impl Fn(&Name) -> bool) -> Resolution {
        let Some(query) = query_key(text) else {
            return Resolution::Unknown;
        };
        let targets: Vec<_> = self
            .aliases
            .iter()
            .filter(|(from, _)| *from == query)
            .map(|(_, to)| to)
            .collect();
        // An explicit alias must not fall through to an unrelated in-scope name.
        if !targets.is_empty() {
            return resolution(
                self.entries
                    .iter()
                    .filter(|entry| includes(&entry.name) && targets.contains(&&entry.key))
                    .map(|entry| entry.name.clone())
                    .collect(),
            );
        }
        let entries: Vec<_> = self
            .entries
            .iter()
            .filter(|entry| includes(&entry.name))
            .collect();
        let exact: Vec<_> = entries
            .iter()
            .filter(|entry| entry.key == query)
            .map(|entry| entry.name.clone())
            .collect();
        if !exact.is_empty() {
            return resolution(exact);
        }
        let single = !query.contains(' ');
        if single {
            let words: Vec<_> = entries
                .iter()
                .filter(|entry| entry.key.split(' ').any(|word| word == query))
                .map(|entry| entry.name.clone())
                .collect();
            if !words.is_empty() {
                return resolution(words);
            }
        }
        let query: Vec<_> = query.chars().collect();
        let limit = if query.len() <= 4 { 1 } else { 2 };
        let mut best = limit + 1;
        let mut matches = Vec::new();
        for entry in entries {
            let mut distance = bounded_distance(&query, &entry.key.chars().collect::<Vec<_>>(), limit);
            if single {
                for word in entry.key.split(' ') {
                    let word_distance = bounded_distance(&query, &word.chars().collect::<Vec<_>>(), limit);
                    distance = match (distance, word_distance) {
                        (Some(a), Some(b)) => Some(a.min(b)),
                        (a, b) => a.or(b),
                    };
                }
            }
            if let Some(distance) = distance {
                if distance < best {
                    best = distance;
                    matches.clear();
                }
                if distance == best {
                    matches.push(entry.name.clone());
                }
            }
        }
        resolution(matches)
    }
}

fn query_key(text: &str) -> Option<String> {
    if text.len() > MAX_INPUT_BYTES || text.chars().any(|ch| ch.is_control() && !ch.is_whitespace()) {
        return None;
    }
    let key = normalize(text);
    (!key.is_empty()).then_some(key)
}

fn unique(mut names: Vec<Name>) -> Vec<Name> {
    names.sort_by(|a, b| (a.kind, &a.id, &a.name).cmp(&(b.kind, &b.id, &b.name)));
    names.dedup_by(|a, b| a.kind == b.kind && a.id == b.id);
    names
}

fn resolution(names: Vec<Name>) -> Resolution {
    let names = unique(names);
    match names.as_slice() {
        [] => Resolution::Unknown,
        [name] => Resolution::Found(name.clone()),
        _ => Resolution::Ambiguous(names),
    }
}

/// Only the diagonal band can produce a distance within the limit.
fn bounded_distance(a: &[char], b: &[char], limit: usize) -> Option<usize> {
    if a.len().abs_diff(b.len()) > limit {
        return None;
    }
    if a.is_empty() {
        return (b.len() <= limit).then_some(b.len());
    }
    if b.is_empty() {
        return (a.len() <= limit).then_some(a.len());
    }
    let infinity = limit + 1;
    let mut previous = vec![infinity; b.len() + 1];
    let mut row = previous.clone();
    for (j, value) in previous.iter_mut().enumerate().take(limit + 1) {
        *value = j;
    }
    for i in 1..=a.len() {
        let low = i.saturating_sub(limit).max(1);
        let high = b.len().min(i + limit);
        row[low - 1] = if low == 1 { i.min(infinity) } else { infinity };
        if high < b.len() {
            row[high + 1] = infinity;
        }
        let mut minimum = infinity;
        for j in low..=high {
            row[j] = (previous[j] + 1)
                .min(row[j - 1] + 1)
                .min(previous[j - 1] + usize::from(a[i - 1] != b[j - 1]))
                .min(infinity);
            minimum = minimum.min(row[j]);
        }
        if minimum > limit {
            return None;
        }
        std::mem::swap(&mut previous, &mut row);
    }
    (previous[b.len()] <= limit).then_some(previous[b.len()])
}

const CLARIFICATION_WINDOW: Duration = Duration::from_secs(90);

/// Original options with an injected monotonic clock; unsuccessful replies never renew it.
#[derive(Debug)]
pub struct Clarification {
    choices: Vec<Name>,
    started: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    Resolution(Resolution),
    Expired,
    Stale,
    Inactive,
}

impl Clarification {
    pub fn new(choices: Vec<Name>, now: Duration) -> Option<Self> {
        let choices = unique(choices);
        (choices.len() >= 2).then_some(Self {
            choices,
            started: now,
        })
    }

    pub fn candidates(&self, now: Duration) -> &[Name] {
        if self.in_window(now) { &self.choices } else { &[] }
    }

    pub fn cancel(&mut self) {
        self.choices.clear();
    }

    pub fn answer(&mut self, text: &str, now: Duration, current: &NameResolver) -> Reply {
        if self.choices.is_empty() {
            return Reply::Inactive;
        }
        if !self.in_window(now) {
            self.cancel();
            return Reply::Expired;
        }
        // A changed choice invalidates the whole question, never just the missing option.
        if self.choices.iter().any(|choice| {
            !current.entries.iter().any(|entry| {
                entry.name.kind == choice.kind
                    && entry.name.id == choice.id
                    && entry.key == normalize(&choice.name)
            })
        }) {
            self.cancel();
            return Reply::Stale;
        }
        let result = current.resolve_where(text, |name| {
            self.choices
                .iter()
                .any(|choice| choice.kind == name.kind && choice.id == name.id)
        });
        if matches!(result, Resolution::Found(_)) {
            self.cancel();
        }
        Reply::Resolution(result)
    }

    fn in_window(&self, now: Duration) -> bool {
        now.checked_sub(self.started)
            .is_some_and(|elapsed| elapsed <= CLARIFICATION_WINDOW)
    }
}

#[cfg(test)]
mod tests {
    use super::bounded_distance;

    fn full_distance(a: &[char], b: &[char]) -> usize {
        let mut row: Vec<_> = (0..=b.len()).collect();
        for (i, a) in a.iter().enumerate() {
            let mut diagonal = row[0];
            row[0] = i + 1;
            for (j, b) in b.iter().enumerate() {
                let above = row[j + 1];
                row[j + 1] = (above + 1).min(row[j] + 1).min(diagonal + usize::from(a != b));
                diagonal = above;
            }
        }
        row[b.len()]
    }

    #[test]
    fn bounded_distance_matches_full_unicode_matrix() {
        let mut words = vec![Vec::new()];
        for _ in 0..4 {
            let previous = words.clone();
            for word in previous {
                for ch in ['a', 'b', '界'] {
                    let mut next = word.clone();
                    next.push(ch);
                    words.push(next);
                }
            }
            words.sort();
            words.dedup();
        }
        for a in &words {
            for b in &words {
                let distance = full_distance(a, b);
                for limit in 0..=2 {
                    assert_eq!(
                        bounded_distance(a, b, limit),
                        (distance <= limit).then_some(distance),
                        "{a:?} {b:?} {limit}"
                    );
                }
            }
        }
    }
}
