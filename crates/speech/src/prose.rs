//! What of a streaming reply is said aloud: its prose. Code blocks and inline code are skipped
//! (Zeca never reads a command or a path aloud), a link keeps its words, and markup goes. Works on
//! pieces as they stream: a fence may open in one and close three later.

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum State {
    #[default]
    Text,
    /// Inside `code`, opened by this many backticks.
    Inline(usize),
    /// Inside a fenced block opened by this many backticks.
    Fence(usize),
    /// Inside a link's `(url)`, this deep in parentheses.
    Url(usize),
}

#[derive(Debug)]
pub struct Prose {
    state: State,
    /// Backticks seen and not yet read: how many tells inline code from a fence.
    ticks: usize,
    /// Whether those backticks opened their line.
    ticks_open_line: bool,
    /// Only whitespace so far on this line of the reply (code included): a fence opens or closes
    /// only there.
    blank: bool,
    /// The line so far while it may still be a list marker or a heading (`- `, `1. `, `## `).
    start: Option<String>,
    /// The last character given out (none yet: `\0`), to end a line that has no punctuation.
    last: char,
    /// A space is due before the next word.
    space: bool,
    /// A `]` just went by: a `(` now opens the link's address.
    bracket: bool,
}

impl Default for Prose {
    fn default() -> Self {
        Self {
            state: State::Text,
            ticks: 0,
            ticks_open_line: false,
            blank: true,
            start: Some(String::new()),
            last: '\0',
            space: false,
            bracket: false,
        }
    }
}

impl Prose {
    /// The speakable part of `text`, given what came before it.
    pub fn push(&mut self, text: &str) -> String {
        let mut out = String::new();
        for c in text.chars() {
            if c == '`' {
                if self.ticks == 0 {
                    self.ticks_open_line = self.blank;
                }
                self.ticks += 1;
                continue;
            }
            self.ticks(&mut out);
            self.char(c, &mut out);
            self.blank = c == '\n' || (self.blank && c.is_whitespace());
        }
        out
    }

    /// The end of the reply.
    pub fn finish(&mut self) -> String {
        let mut out = String::new();
        self.ticks(&mut out);
        if let Some(start) = self.start.take()
            && self.state == State::Text
        {
            self.line(&start, &mut out);
        }
        *self = Self::default();
        out
    }

    /// Reads a run of backticks once a character that is not one comes.
    fn ticks(&mut self, out: &mut String) {
        let n = std::mem::take(&mut self.ticks);
        if n == 0 {
            return;
        }
        self.state = match self.state {
            State::Fence(open) if n >= open && self.ticks_open_line => State::Text,
            State::Fence(open) => State::Fence(open),
            State::Inline(open) if n == open => State::Text,
            State::Inline(open) => State::Inline(open),
            _ if n >= 3 && self.ticks_open_line => State::Fence(n),
            _ => State::Inline(n),
        };
        // A code span in the middle of a line still separates the words around it.
        if self.state == State::Text {
            self.emit(' ', out);
        }
        if let Some(start) = self.start.as_mut() {
            start.push(' ');
        }
    }

    fn char(&mut self, c: char, out: &mut String) {
        if c == '\n' {
            // Inline code and a link's address end with their line: an unclosed one never
            // silences the rest of the reply.
            if matches!(self.state, State::Inline(_) | State::Url(_)) {
                self.state = State::Text;
            }
            if self.state == State::Text {
                if let Some(start) = self.start.take() {
                    self.line(&start, out);
                }
                // A line that ends with no punctuation (a heading, a list item) still ends.
                if self.last != '\0' && !".!?:;,\u{2026}".contains(self.last) {
                    self.emit('.', out);
                }
                self.emit(' ', out);
            }
            self.start = Some(String::new());
            return;
        }
        match self.state {
            State::Fence(_) | State::Inline(_) => {
                if let Some(start) = self.start.as_mut() {
                    start.push(' ');
                }
            }
            State::Url(depth) => {
                self.state = match c {
                    '(' => State::Url(depth + 1),
                    ')' if depth <= 1 => State::Text,
                    ')' => State::Url(depth - 1),
                    _ => State::Url(depth),
                };
            }
            State::Text => {
                if let Some(start) = self.start.as_mut() {
                    start.push(c);
                    // Still possibly a marker: wait for the rest of it.
                    if c.is_whitespace() || "#>-*+.)0123456789".contains(c) {
                        return;
                    }
                    let start = self.start.take().unwrap_or_default();
                    self.line(&start, out);
                } else {
                    self.text(&c.to_string(), out);
                }
            }
        }
    }

    /// The start of a line: a list marker, a heading's or a quote's goes.
    fn line(&mut self, text: &str, out: &mut String) {
        let mut rest = text.trim_start();
        // `## `, `> `, `- `, `* `, `+ `, `12. `, `3) `.
        let marker = rest.split_once(char::is_whitespace).map(|(m, _)| m);
        if let Some(m) = marker {
            let numbered = m.len() > 1
                && m[..m.len() - 1].chars().all(|c| c.is_ascii_digit())
                && m.ends_with(['.', ')']);
            if m.chars().all(|c| c == '#')
                || m.chars().all(|c| c == '>')
                || ["-", "*", "+"].contains(&m)
                || numbered
            {
                rest = rest[m.len()..].trim_start();
            }
        }
        self.text(rest, out);
    }

    /// Text outside code: markup goes.
    fn text(&mut self, text: &str, out: &mut String) {
        for c in text.chars() {
            if self.bracket && c == '(' {
                self.bracket = false;
                self.state = State::Url(1);
                continue;
            }
            self.bracket = c == ']';
            if self.state != State::Text {
                continue;
            }
            match c {
                '*' | '[' | ']' | '#' | '<' | '>' | '~' => {}
                '_' | '|' => self.emit(' ', out),
                _ => self.emit(c, out),
            }
        }
    }

    /// Spaces wait for the next word: one at a time, none to start, none before punctuation.
    fn emit(&mut self, c: char, out: &mut String) {
        if c.is_whitespace() {
            self.space = self.last != '\0';
            return;
        }
        if std::mem::take(&mut self.space) && !".,!?;:".contains(c) {
            out.push(' ');
        }
        out.push(c);
        self.last = c;
    }
}

/// Words no voice should spell out: web addresses.
pub fn without_urls(sentence: &str) -> String {
    sentence
        .split_whitespace()
        .filter(|w| !w.contains("://") && !w.starts_with("www."))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fed in pieces of `size` characters.
    fn speak(text: &str, size: usize) -> String {
        let mut p = Prose::default();
        let chars: Vec<char> = text.chars().collect();
        let mut out = String::new();
        for piece in chars.chunks(size) {
            out.push_str(&p.push(&piece.iter().collect::<String>()));
        }
        out.push_str(&p.finish());
        out.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn code_is_never_read_aloud() {
        let reply = "Run this:\n\n```bash\nrm -rf target && cargo test\n```\n\nThen `git push` it.";
        for size in [1, 2, 5, 200] {
            assert_eq!(speak(reply, size), "Run this: Then it.", "{size}");
        }
        assert_eq!(speak("Use ``a ` b`` here.", 1), "Use here.");
        // A fence inside a longer one, or backticks inside a code line, keep the block open.
        let nested = "Like this:\n````md\n```sh\nrm -rf /\n```\necho ```done```\n````\nThat is it.";
        for size in [1, 4, 200] {
            assert_eq!(speak(nested, size), "Like this: That is it.", "{size}");
        }
        // An unclosed backtick ends with its line.
        assert_eq!(speak("A lone ` tick\nstill heard.", 1), "A lone. still heard.");
    }

    #[test]
    fn markup_goes_and_links_keep_their_words() {
        assert_eq!(
            speak(
                "## Summary\n\n- **Fixed** the [parser](https://x.dev/a_(b)) bug\n2. Added _tests_\n",
                3
            ),
            "Summary. Fixed the parser bug. Added tests."
        );
        assert_eq!(speak("> Quoted line\nand more", 1), "Quoted line. and more");
        // A number that is not a list marker stays.
        assert_eq!(speak("42 files changed.", 1), "42 files changed.");
    }

    #[test]
    fn web_addresses_are_dropped() {
        assert_eq!(
            without_urls("See https://example.com/x for www.foo.org details."),
            "See for details."
        );
    }
}
