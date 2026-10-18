use std::ops::Range;
use unicode_normalization::char::is_combining_mark;

const LIMIT: usize = 256 * 1024;
pub const TOO_LARGE: &str = "Dictation text processing exceeded its size limit.";

#[derive(Debug, Default, Clone)]
pub struct Rules {
    pub words: Vec<String>,
    pub replacements: Vec<(String, String)>,
    pub snippets: Vec<(String, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Snippet,
    Replacement,
    Spelling,
    SpokenCode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub kind: Kind,
    pub rule: String,
    pub heard: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub text: String,
    pub change: Option<Change>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Processed {
    pub segments: Vec<Segment>,
    pub words_changed: bool,
    pub code_changed: bool,
}
impl Processed {
    pub fn text(&self) -> String {
        self.segments.iter().map(|s| s.text.as_str()).collect()
    }
}

#[derive(Clone)]
struct Atom {
    ch: Option<char>,
    tag: Option<usize>,
}
struct Tag {
    change: Change,
    text: String,
}

fn word(c: char) -> bool {
    c.is_alphanumeric() || is_combining_mark(c)
}
fn separator(c: char) -> bool {
    !word(c) && !c.is_control()
}
fn fold(c: char) -> impl Iterator<Item = char> {
    c.to_lowercase().flat_map(char::to_uppercase)
}

fn pattern(source: &str) -> Vec<Option<char>> {
    let chars: Vec<char> = source.trim().chars().flat_map(fold).collect();
    if chars.iter().any(|c| c.is_control()) {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut at = 0;
    while at < chars.len() {
        let mut end = at;
        while end < chars.len() && separator(chars[end]) {
            end += 1;
        }
        if end > at && at > 0 && end < chars.len() && word(chars[at - 1]) && word(chars[end]) {
            out.push(None);
            at = end;
        } else {
            out.push(Some(chars[at]));
            at += 1;
        }
    }
    out
}

fn matches(atoms: &[Atom], source: &str) -> Vec<Range<usize>> {
    let needle = pattern(source);
    if needle.is_empty() {
        return Vec::new();
    }
    let mut hay = Vec::new();
    for (index, atom) in atoms.iter().enumerate() {
        if let Some(c) = atom.ch {
            hay.extend(fold(c).map(|c| (Some(c), index)));
        } else {
            hay.push((None, index));
        }
    }
    let mut found = Vec::new();
    let mut start = 0;
    while start < hay.len() {
        if start > 0 && (hay[start - 1].1 == hay[start].1 || hay[start - 1].0.is_some_and(word)) {
            start += 1;
            continue;
        }
        let mut end = start;
        let matched = needle.iter().all(|part| match part {
            Some(c) if hay.get(end).is_some_and(|h| h.0 == Some(*c)) => {
                end += 1;
                true
            }
            None => {
                let before = end;
                while hay.get(end).is_some_and(|h| h.0.is_some_and(separator)) {
                    end += 1;
                }
                end > before
            }
            _ => false,
        });
        if matched && (end == hay.len() || (hay[end].1 != hay[end - 1].1 && !hay[end].0.is_some_and(word))) {
            found.push(hay[start].1..hay[end - 1].1 + 1);
            start = end;
        } else {
            start += 1;
        }
    }
    found
}

fn render(atoms: &[Atom], tags: &[Tag]) -> String {
    let mut text = String::new();
    for atom in atoms {
        if let Some(ch) = atom.ch {
            text.push(ch);
        } else {
            text.push_str(&tags[atom.tag.expect("snippet tag")].text);
        }
    }
    text
}

fn apply(
    atoms: &mut Vec<Atom>,
    tags: &mut Vec<Tag>,
    source: &str,
    to: &str,
    kind: Kind,
) -> Result<(), &'static str> {
    let ranges = matches(atoms, source);
    if ranges.is_empty() {
        return Ok(());
    }
    let mut size = render(atoms, tags).len();
    for range in ranges.into_iter().rev() {
        let heard = render(&atoms[range.clone()], tags);
        if heard == to && kind != Kind::Snippet {
            continue;
        }
        size = size - heard.len() + to.len();
        if size > LIMIT {
            return Err(TOO_LARGE);
        }
        let tag = tags.len();
        tags.push(Tag {
            change: Change {
                kind,
                rule: source.into(),
                heard,
            },
            text: to.into(),
        });
        // Opaque snippet marks cannot collide with literal input such as "⟦0⟧".
        let replacement = if kind == Kind::Snippet {
            vec![Atom {
                ch: None,
                tag: Some(tag),
            }]
        } else {
            to.chars()
                .map(|ch| Atom {
                    ch: Some(ch),
                    tag: Some(tag),
                })
                .collect()
        };
        atoms.splice(range, replacement);
    }
    Ok(())
}

impl Rules {
    pub fn apply(&self, text: &str, spoken_code: bool) -> Result<Processed, &'static str> {
        if text.len() > LIMIT {
            return Err(TOO_LARGE);
        }
        let mut atoms: Vec<Atom> = text
            .chars()
            .map(|ch| Atom {
                ch: Some(ch),
                tag: None,
            })
            .collect();
        let mut tags = Vec::new();
        // ponytail: scan once per rule; index phrases only if measured vocabulary size needs it.
        for (from, to) in &self.snippets {
            apply(&mut atoms, &mut tags, from, to, Kind::Snippet)?;
        }
        for (from, to) in &self.replacements {
            apply(&mut atoms, &mut tags, from, to, Kind::Replacement)?;
        }
        for spelling in &self.words {
            if spelling.chars().filter(|c| c.is_alphanumeric()).count() >= 2
                && spelling
                    .chars()
                    .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '\''))
            {
                apply(&mut atoms, &mut tags, spelling, spelling, Kind::Spelling)?;
            }
        }
        let words_changed = render(&atoms, &tags) != text;
        let mut code_edits = Vec::new();
        if spoken_code {
            let mut start = 0;
            while start < atoms.len() {
                if atoms[start].ch.is_none() {
                    start += 1;
                    continue;
                }
                let end = start + atoms[start..].iter().take_while(|a| a.ch.is_some()).count();
                let heard = render(&atoms[start..end], &tags);
                let boundaries: Vec<usize> = heard
                    .char_indices()
                    .map(|(at, _)| at)
                    .chain(std::iter::once(heard.len()))
                    .collect();
                for (range, symbol) in super::code_changes(&heard) {
                    let left = start
                        + boundaries
                            .binary_search(&range.start)
                            .expect("character boundary");
                    let right = start + boundaries.binary_search(&range.end).expect("character boundary");
                    code_edits.push((left..right, symbol));
                }
                start = end;
            }
        }
        let code_changed = !code_edits.is_empty();
        for (range, symbol) in code_edits.into_iter().rev() {
            let tag = tags.len();
            tags.push(Tag {
                change: Change {
                    kind: Kind::SpokenCode,
                    rule: "spoken-code".into(),
                    heard: render(&atoms[range.clone()], &tags),
                },
                text: symbol.into(),
            });
            atoms.splice(
                range,
                [Atom {
                    ch: Some(symbol),
                    tag: Some(tag),
                }],
            );
        }
        let mut segments: Vec<Segment> = Vec::new();
        let mut previous = None;
        for atom in atoms {
            if segments.is_empty() || atom.tag != previous {
                segments.push(Segment {
                    text: String::new(),
                    change: atom.tag.map(|id| tags[id].change.clone()),
                });
            }
            let segment = segments.last_mut().expect("segment");
            if let Some(ch) = atom.ch {
                segment.text.push(ch);
            } else {
                segment.text.push_str(&tags[atom.tag.expect("snippet tag")].text);
            }
            previous = atom.tag;
        }
        Ok(Processed {
            segments,
            words_changed,
            code_changed,
        })
    }
}
