//! X11 dictation chords only while a turn is live; no grabs, text, key history or Wayland fallback.

use std::collections::BTreeSet;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use x11rb::connection::Connection;
use x11rb::protocol::{
    Event,
    xinput::{ConnectionExt as _, EventMask, KeyEventFlags, XIEventMask},
    xproto::ConnectionExt as _,
};
use x11rb::rust_connection::RustConnection;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Escape,
    Space,
    Other,
}

#[derive(Debug)]
pub struct Listener {
    active: Arc<AtomicBool>,
}

impl Listener {
    pub fn active(&self) -> Arc<AtomicBool> {
        self.active.clone()
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        self.active.store(false, Ordering::SeqCst);
    }
}

/// Bindings are the actual Talk and Copy-again descriptions, never their suggested defaults.
/// An unrecognized binding disables raw chords so a remapped shortcut cannot cancel itself.
/// ponytail: localized key names may be unknown; a normalized portal accelerator would extend support.
pub fn listen(
    bindings: &[String],
    live: impl Fn() -> bool + Send + 'static,
    emit: impl Fn(Key) + Send + 'static,
) -> Result<Option<Listener>, String> {
    if !supported() || !live() {
        return Ok(None);
    }
    let input =
        Input::open(bindings, None).map_err(|_| "X11 dictation chords are unavailable.".to_string())?;
    if !live() {
        return Ok(None);
    }
    start(input, live, emit).map(Some)
}

fn start(
    mut input: Input,
    live: impl Fn() -> bool + Send + 'static,
    emit: impl Fn(Key) + Send + 'static,
) -> Result<Listener, String> {
    let active = Arc::new(AtomicBool::new(true));
    let listener = Listener {
        active: active.clone(),
    };
    std::thread::Builder::new()
        .name("dictation-keys".into())
        .spawn(move || {
            pump(|| input.next(), &active, live, emit);
            // Closing this connection removes its XI2 subscriptions, including on poll failure.
            active.store(false, Ordering::SeqCst);
        })
        .map_err(|_| "The X11 dictation key listener could not start.".to_string())?;
    Ok(listener)
}

pub fn uses_space(binding: &str) -> bool {
    matches!(
        binding.rsplit('+').next().and_then(|s| keyval(s.trim())),
        Some(0x20 | 0xff80)
    )
}

fn supported() -> bool {
    supports(
        &std::env::var("XDG_SESSION_TYPE").unwrap_or_default(),
        &std::env::var("WAYLAND_DISPLAY").unwrap_or_default(),
        &std::env::var("DISPLAY").unwrap_or_default(),
    )
}

fn supports(session: &str, wayland: &str, display: &str) -> bool {
    !session.eq_ignore_ascii_case("wayland") && wayland.trim().is_empty() && !display.trim().is_empty()
}

fn pump(
    mut next: impl FnMut() -> Result<Option<Key>, ()>,
    active: &AtomicBool,
    live: impl Fn() -> bool,
    emit: impl Fn(Key),
) {
    while active.load(Ordering::SeqCst) && live() {
        match next() {
            Ok(Some(key)) => {
                if active.load(Ordering::SeqCst) && live() {
                    emit(key);
                }
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(5)),
            Err(()) => break,
        }
    }
}

struct Input {
    conn: RustConnection,
    filter: Filter,
    bindings: Vec<String>,
}

impl Input {
    fn open(bindings: &[String], display: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        if bindings.is_empty() {
            return Err("no actual shortcut bindings".into());
        }
        let (conn, screen) = x11rb::connect(display)?;
        let root = conn.setup().roots.get(screen).ok_or("no screen")?.root;
        let version = conn.xinput_xi_query_version(2, 1)?.reply()?;
        if version.major_version < 2 || (version.major_version == 2 && version.minor_version < 1) {
            return Err("XI 2.1 is required to observe keys during a shortcut grab".into());
        }
        let filter = Self::keymap(&conn, bindings)?;
        // All master devices (1), not master + slave events for the same physical press.
        conn.xinput_xi_select_events(
            root,
            &[EventMask {
                deviceid: 1,
                mask: vec![XIEventMask::RAW_KEY_PRESS | XIEventMask::RAW_KEY_RELEASE],
            }],
        )?
        .check()?;
        conn.flush()?;
        Ok(Self {
            conn,
            filter,
            bindings: bindings.to_vec(),
        })
    }

    fn keymap(conn: &RustConnection, bindings: &[String]) -> Result<Filter, Box<dyn std::error::Error>> {
        let min = conn.setup().min_keycode;
        let max = conn.setup().max_keycode;
        let count = max
            .checked_sub(min)
            .and_then(|n| n.checked_add(1))
            .ok_or("invalid keycode range")?;
        let map = conn.get_keyboard_mapping(min, count)?.reply()?;
        let modifiers = conn.get_modifier_mapping()?.reply()?;
        let held = conn.query_keymap()?.reply()?.keys;
        Filter::new(
            min,
            map.keysyms_per_keycode,
            &map.keysyms,
            &modifiers.keycodes,
            held,
            bindings,
        )
        .ok_or_else(|| "unrecognized actual shortcut binding".into())
    }

    fn next(&mut self) -> Result<Option<Key>, ()> {
        let Some(event) = self.conn.poll_for_event().map_err(|_| ())? else {
            return Ok(None);
        };
        match event {
            Event::XinputRawKeyPress(e) => Ok(self
                .filter
                .press(e.detail, e.flags.contains(KeyEventFlags::KEY_REPEAT))),
            Event::XinputRawKeyRelease(e) => {
                self.filter.release(e.detail);
                Ok(None)
            }
            Event::MappingNotify(e) => {
                if e.request == x11rb::protocol::xproto::Mapping::KEYBOARD
                    || e.request == x11rb::protocol::xproto::Mapping::MODIFIER
                {
                    // X servers also send initial map notifications: refresh instead of cancelling.
                    self.filter = Self::keymap(&self.conn, &self.bindings).map_err(|_| ())?;
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }
}

struct Chord {
    keys: BTreeSet<u8>,
    modifiers: Vec<BTreeSet<u8>>,
}
struct Filter {
    symbols: [u32; 256],
    modifiers: BTreeSet<u8>,
    held: [bool; 256],
    ignored: Vec<Chord>,
}

impl Filter {
    fn new(
        min: u8,
        width: u8,
        symbols: &[u32],
        modifiers: &[u8],
        held: [u8; 32],
        bindings: &[String],
    ) -> Option<Self> {
        if width == 0 {
            return None;
        }
        let rows: Vec<_> = symbols
            .chunks_exact(usize::from(width))
            .enumerate()
            .map(|(i, row)| (u8::try_from(usize::from(min) + i).ok(), row))
            .collect();
        let mut filter = Self {
            symbols: [0; 256],
            modifiers: modifiers.iter().copied().filter(|k| *k != 0).collect(),
            held: std::array::from_fn(|i| held[i / 8] & (1 << (i % 8)) != 0),
            ignored: Vec::new(),
        };
        for (code, row) in &rows {
            filter.symbols[usize::from((*code)?)] = row[0];
        }
        for binding in bindings {
            let parts: Vec<_> = binding.split('+').map(str::trim).collect();
            let name = if binding.ends_with("++") {
                "+"
            } else {
                *parts.last()?
            };
            let key = keyval(name)?;
            let keys: BTreeSet<_> = rows
                .iter()
                .filter(|(_, row)| row.iter().any(|sym| lower(*sym) == key))
                .filter_map(|(code, _)| *code)
                .collect();
            if keys.is_empty() {
                return None;
            }
            let end = parts.len() - if binding.ends_with("++") { 2 } else { 1 };
            let mut required = Vec::new();
            for name in &parts[..end] {
                let wanted = modifier_symbols(name)?;
                let codes: BTreeSet<_> = rows
                    .iter()
                    .filter(|(_, row)| row.iter().any(|sym| wanted.contains(sym)))
                    .filter_map(|(code, _)| *code)
                    .collect();
                if codes.is_empty() {
                    return None;
                }
                required.push(codes);
            }
            filter.ignored.push(Chord {
                keys,
                modifiers: required,
            });
        }
        Some(filter)
    }

    fn press(&mut self, code: u32, repeat: bool) -> Option<Key> {
        let code = u8::try_from(code).ok()?;
        self.held[usize::from(code)] = true;
        if repeat
            || self.modifiers.contains(&code)
            || self.ignored.iter().any(|chord| {
                chord.keys.contains(&code)
                    && chord
                        .modifiers
                        .iter()
                        .all(|group| group.iter().any(|code| self.held[usize::from(*code)]))
            })
        {
            return None;
        }
        classify(self.symbols[usize::from(code)])
    }

    fn release(&mut self, code: u32) {
        if let Ok(code) = u8::try_from(code) {
            self.held[usize::from(code)] = false;
        }
    }
}

fn lower(symbol: u32) -> u32 {
    *gtk::gdk::keys::Key::from(symbol).to_lower()
}
fn keyval(name: &str) -> Option<u32> {
    let alias = match name.to_ascii_lowercase().as_str() {
        "space" => "space",
        "esc" | "escape" => "Escape",
        "enter" | "return" => "Return",
        "backspace" => "BackSpace",
        "del" | "delete" => "Delete",
        "pgup" | "pageup" => "Prior",
        "pgdown" | "pagedown" => "Next",
        _ => name,
    };
    let mut chars = alias.chars();
    let first = chars.next()?;
    let key = if chars.next().is_none() {
        *gtk::gdk::keys::Key::from_unicode(first)
    } else {
        *gtk::gdk::keys::Key::from_name(alias)
    };
    (key != 0 && key != 0x00ff_ffff).then(|| lower(key))
}

fn modifier_symbols(name: &str) -> Option<&'static [u32]> {
    match name.to_ascii_lowercase().as_str() {
        "ctrl" | "control" => Some(&[0xffe3, 0xffe4]),
        "alt" => Some(&[0xffe9, 0xffea]),
        "shift" => Some(&[0xffe1, 0xffe2]),
        "meta" | "super" | "win" => Some(&[0xffe7, 0xffe8, 0xffeb, 0xffec]),
        "hyper" => Some(&[0xffed, 0xffee]),
        _ => None,
    }
}

fn classify(symbol: u32) -> Option<Key> {
    match symbol {
        0xff1b => Some(Key::Escape),
        0x20 | 0xff80 => Some(Key::Space),
        0 | 0xff14 | 0xff7e | 0xff7f | 0xffe1..=0xffee | 0xfe01..=0xfe13 => None,
        _ => Some(Key::Other),
    }
}

#[cfg(test)]
mod tests;
