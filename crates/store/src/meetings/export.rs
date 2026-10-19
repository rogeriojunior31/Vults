//! Pure exporters: subtitles always select one source clock, never an invented mixed timeline.
use std::fmt::Write;

use vults_core::i18n::{Lang, meeting_export, meeting_voice};

use super::{Meeting, Source};
use crate::Error;

#[derive(Clone, Copy, Debug)]
pub enum Subtitle {
    Srt,
    Vtt,
}

fn label(meeting: &Meeting, source: Source, speaker: Option<u32>, lang: Lang) -> String {
    match speaker {
        None => meeting_export(lang).unknown.into(),
        Some(id) => meeting
            .speakers
            .iter()
            .find(|s| s.source == source && s.id == id)
            .and_then(|s| s.name.clone())
            .unwrap_or_else(|| meeting_voice(lang, id.saturating_add(1))),
    }
}

fn time(ms: u32, separator: char) -> String {
    format!(
        "{:02}:{:02}:{:02}{separator}{:03}",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1000 % 60,
        ms % 1000
    )
}

fn plain(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn markdown_text(value: &str) -> String {
    let mut out = String::new();
    for c in plain(value).chars() {
        if c.is_ascii_punctuation() {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

pub fn subtitles(meeting: &Meeting, source: Source, format: Subtitle, lang: Lang) -> Result<String, Error> {
    meeting.validate()?;
    let separator = match format {
        Subtitle::Srt => ',',
        Subtitle::Vtt => '.',
    };
    let mut out = match format {
        Subtitle::Srt => String::new(),
        Subtitle::Vtt => "WEBVTT\n\n".into(),
    };
    for (index, segment) in meeting
        .track(source)
        .segments
        .iter()
        .filter(|s| s.start_ms < s.end_ms)
        .enumerate()
    {
        let _ = writeln!(
            out,
            "{}\n{} --> {}\n{}: {}\n",
            index + 1,
            time(segment.start_ms, separator),
            time(segment.end_ms, separator),
            html(&plain(&label(meeting, source, segment.speaker, lang))),
            html(&plain(&segment.text))
        );
    }
    if out.ends_with("\n\n") {
        out.pop();
    }
    Ok(out)
}

pub fn markdown(meeting: &Meeting, lang: Lang) -> Result<String, Error> {
    meeting.validate()?;
    let labels = meeting_export(lang);
    let mut out = format!("# {}\n", markdown_text(&meeting.title));
    if let Some(notes) = &meeting.notes {
        let _ = write!(
            out,
            "\n## {}\n\n{}\n",
            labels.summary,
            markdown_text(&notes.summary)
        );
        if !notes.key_points.is_empty() {
            let _ = write!(out, "\n## {}\n\n", labels.points);
            for point in &notes.key_points {
                let _ = writeln!(out, "- {}", markdown_text(point));
            }
        }
        if !notes.todos.is_empty() {
            let _ = write!(out, "\n## {}\n\n", labels.todos);
            for todo in &notes.todos {
                let _ = write!(out, "- [ ] {}", markdown_text(&todo.text));
                for metadata in [&todo.owner, &todo.due].into_iter().flatten() {
                    let _ = write!(out, " — {}", markdown_text(metadata));
                }
                out.push('\n');
            }
        }
    }
    for (source, heading) in [
        (Source::Microphone, labels.microphone),
        (Source::System, labels.system),
    ] {
        let _ = write!(out, "\n## {heading}\n\n");
        for segment in &meeting.track(source).segments {
            let _ = writeln!(
                out,
                "- [{}–{}] **{}:** {}",
                time(segment.start_ms, '.'),
                time(segment.end_ms, '.'),
                markdown_text(&label(meeting, source, segment.speaker, lang)),
                markdown_text(&segment.text)
            );
        }
    }
    Ok(out)
}
