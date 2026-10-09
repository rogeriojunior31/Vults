//! What the UI renders. Plain data, serialized to the webview as-is.

use serde::Serialize;

use crate::{
    Activity, AgentKind, AlertLevel, Attention, Diff, Outcome, Question, SessionKey, State, Status, Terminal,
    i18n,
};

#[derive(Serialize, Clone, Debug, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
// `ts(optional)` touches only the TypeScript: the lab's older states leave those fields out.
pub struct ViewModel {
    pub sessions: Vec<SessionView>,
    pub approval: Option<ApprovalView>,
    pub alerts: Vec<AlertView>,
    /// The most any session wants the user.
    #[cfg_attr(test, ts(as = "Option<Attention>", optional))]
    pub attention: Attention,
    /// The cards that left the line most recently and how, newest first: a surface tells the user
    /// what became of the card it showed.
    #[cfg_attr(test, ts(as = "Option<Vec<EndedView>>", optional))]
    pub ended: Vec<EndedView>,
    /// Each switched-on connector's card, once it has polled.
    #[cfg_attr(test, ts(as = "Option<Vec<crate::board::BoardView>>", optional))]
    pub boards: Vec<crate::board::BoardView>,
    /// What Zeca wears today (`crate::looks`), if anything.
    #[cfg_attr(test, ts(optional = nullable))]
    pub look: Option<crate::looks::Outfit>,
    /// The session the user put in front, if any.
    #[cfg_attr(test, ts(optional = nullable))]
    pub focus: Option<SessionRef>,
    /// The session in front by [`State::front`]'s rule: the card's, the user's, the first.
    #[cfg_attr(test, ts(optional = nullable))]
    pub front: Option<SessionRef>,
    /// Do not disturb: surfaces make no sound. Cards still show.
    #[cfg_attr(test, ts(as = "Option<bool>", optional))]
    pub dnd: bool,
    /// The screen is locked: surfaces rest (no animation, no timers).
    #[cfg_attr(test, ts(as = "Option<bool>", optional))]
    pub locked: bool,
    /// "While you were away", until dismissed.
    #[cfg_attr(test, ts(optional = nullable))]
    pub digest: Option<crate::away::DigestView>,
}

/// One session, by its key on the wire (`SessionView`'s `agent` and `id`).
#[derive(Serialize, Clone, Debug, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct SessionRef {
    #[cfg_attr(test, ts(as = "ts::AgentKind"))]
    pub agent: AgentKind,
    pub id: String,
}

impl From<&SessionKey> for SessionRef {
    fn from(k: &SessionKey) -> Self {
        SessionRef {
            agent: k.agent,
            id: k.session_id.clone(),
        }
    }
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct EndedView {
    pub request: String,
    #[cfg_attr(test, ts(as = "ts::AgentKind"))]
    pub agent: AgentKind,
    pub session: String,
    pub outcome: Outcome,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct AlertView {
    pub key: String,
    /// New each time the news arrives: the island sounds an alert once per `seq`.
    pub seq: u64,
    pub connector: String,
    pub level: AlertLevel,
    pub title: String,
    pub detail: String,
    /// The alert opens something when clicked.
    pub link: bool,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
// `ts(optional)`: as on `ViewModel`.
pub struct SessionView {
    pub id: String,
    #[cfg_attr(test, ts(as = "ts::AgentKind"))]
    pub agent: AgentKind,
    /// Another tool's name (`AgentKind::Other`), taken from its session id: `<name>/<id>`.
    #[cfg_attr(test, ts(optional = nullable))]
    pub agent_name: Option<String>,
    pub project: String,
    /// The project folder, where the chat works when this session is in front.
    pub cwd: Option<String>,
    pub status: Status,
    /// What the status asks of the user.
    pub attention: Attention,
    /// The card first in line is this session's, and its status still waits on it: the island
    /// shows that card, with this session in front.
    pub card: bool,
    /// A card of this session waits, first in line or behind another.
    #[cfg_attr(test, ts(as = "Option<bool>", optional))]
    pub waiting: bool,
    /// *Open terminal* can bring it forward (`platform::jump`): its multiplexer's pane, or on KDE
    /// its window. Elsewhere a quick action offers its folder instead (ADR 0011).
    #[cfg_attr(test, ts(as = "Option<bool>", optional))]
    pub raise: bool,
    /// Its project is muted: no sound for it here, no desktop notification.
    #[cfg_attr(test, ts(as = "Option<bool>", optional))]
    pub muted: bool,
    /// Its project is pinned: it comes first on the wire.
    #[cfg_attr(test, ts(as = "Option<bool>", optional))]
    pub pinned: bool,
    /// Working with no news for a while: its bird is flagged (`crate::silence`).
    #[cfg_attr(test, ts(optional = nullable))]
    pub silent: Option<crate::silence::Silence>,
    pub activity: Option<Activity>,
    pub step: Option<String>,
    /// The latest steps, oldest first, for the island's step ticker.
    pub steps: Vec<String>,
    /// What each of `steps` changed, when it is a finished edit; the full diff comes from
    /// [`State::diff`] by its step number.
    #[cfg_attr(test, ts(as = "Option<Vec<Option<DiffSummary>>>", optional))]
    pub diffs: Vec<Option<DiffSummary>>,
    /// How many steps the session has taken so far.
    pub step_count: u32,
    pub subagents: u32,
    /// The question, the last reply or the error that goes with the status.
    pub note: Option<String>,
    /// The editor whose terminal the session runs in ("Cursor", "VS Code").
    pub editor: Option<&'static str>,
    /// Its bird's species, by the renderer's id (`crate::flock`, `ui/src/character/flock/species.ts`).
    /// Zeca keeps his own.
    pub species: &'static str,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
/// A finished edit's counts; the island gets its lines from `Bridge.stepDiff` by `step`.
pub struct DiffSummary {
    pub step: u32,
    pub added: u32,
    pub removed: u32,
    pub files: usize,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ApprovalView {
    pub request: String,
    #[cfg_attr(test, ts(as = "ts::AgentKind"))]
    pub agent: AgentKind,
    /// The session that asked, to put it in front.
    pub session: String,
    pub project: String,
    pub tool: String,
    pub target: String,
    /// The agent's own words for the action ("Run the test suite").
    pub description: Option<String>,
    /// The whole command, when `target` had to cut it.
    pub full: Option<String>,
    /// `target` is only the start of the call: no Always for it.
    pub cut: bool,
    /// Lines an edit adds and removes; both 0 when it is not an edit.
    pub added: u32,
    pub removed: u32,
    /// A question card's questions, in order; empty for a permission.
    pub questions: Vec<Question>,
    /// How many permissions and questions wait, this one included.
    pub queue: usize,
    /// The reminders it has earned by waiting (the attention ladder): one more sound each.
    #[cfg_attr(test, ts(as = "Option<u32>", optional))]
    pub reminders: u32,
}

impl State {
    /// The diff of a session's step, while the step is kept.
    pub fn diff(&self, key: &SessionKey, step: u32) -> Option<&Diff> {
        let s = self.sessions.get(key)?;
        s.diffs.iter().find(|(n, _)| *n == step).map(|(_, d)| d)
    }

    /// The sessions every surface shows, in their order: pinned projects first, then the first to
    /// arrive first. Hidden projects are left out unless a card of theirs waits. Next and previous
    /// move along it.
    pub fn ordered(&self) -> Vec<&crate::Session> {
        let mut sessions: Vec<_> = self.sessions.values().filter(|s| self.visible(s)).collect();
        sessions.sort_by(|a, b| {
            (!self.prefs(a).pin)
                .cmp(&!self.prefs(b).pin)
                .then_with(|| a.started.cmp(&b.started))
                .then_with(|| a.key.cmp(&b.key))
        });
        sessions
    }

    /// The session whose card is first in line, while its status still waits on it: the card is
    /// what the user must see. A question in the terminal is not the card: only one asked here is.
    pub(crate) fn card_session(&self) -> Option<&SessionKey> {
        let p = self.pending.front()?;
        let s = self.sessions.get(&p.session)?;
        crate::shows(s, p).then_some(&p.session)
    }

    /// Who is in front: the session whose card waits, else the one the user chose, else the first.
    /// Work alone never moves it: the birds would trade places with every prompt. Every surface
    /// agrees on it (ADR 0008).
    pub fn front(&self) -> Option<&SessionKey> {
        self.card_session()
            .or(self.focus.as_ref())
            .or_else(|| self.ordered().first().map(|s| &s.key))
    }

    pub fn view(&self) -> ViewModel {
        let sessions = self.ordered();
        let species = crate::flock::species(self.flock, self.season, self.sessions.values());
        let card = self.card_session();
        let sessions: Vec<SessionView> = sessions
            .into_iter()
            .map(|s| SessionView {
                id: s.key.session_id.clone(),
                agent: s.key.agent,
                agent_name: (s.key.agent == AgentKind::Other)
                    .then(|| s.key.session_id.split_once('/').map(|(name, _)| name.to_string()))
                    .flatten(),
                project: s.project.clone(),
                cwd: s.cwd.clone(),
                status: s.status,
                // A loud quiet bird is worth a glance on every surface (ADR 0008), not news.
                attention: match (s.status, s.watch.level) {
                    (Status::Working, Some(crate::silence::Silence::Loud)) => Attention::Info,
                    (status, _) => status.attention(),
                },
                card: card == Some(&s.key),
                waiting: self
                    .pending
                    .iter()
                    .any(|p| p.session == s.key && crate::shows(s, p)),
                raise: raises(&s.terminal),
                muted: self.prefs(s).mute,
                pinned: self.prefs(s).pin,
                silent: s.watch.level.filter(|_| s.status == Status::Working),
                activity: s.activity,
                step: steps(self, s).pop(),
                steps: steps(self, s),
                diffs: diffs(s),
                step_count: s.step_count,
                subagents: s.subagents,
                note: s.note.clone(),
                editor: editor(&s.terminal),
                species: species[&s.key],
            })
            .collect();
        ViewModel {
            attention: sessions
                .iter()
                .map(|s| s.attention)
                .max()
                .unwrap_or(Attention::Quiet),
            sessions,
            approval: self.pending.front().map(|p| ApprovalView {
                request: p.request.0.clone(),
                agent: p.session.agent,
                session: p.session.session_id.clone(),
                project: self
                    .sessions
                    .get(&p.session)
                    .map(|s| s.project.clone())
                    .unwrap_or_default(),
                tool: p.tool.clone(),
                target: p.target.clone(),
                description: p.ask.description.clone(),
                full: p.ask.full.clone(),
                cut: p.ask.cut,
                added: p.ask.added,
                removed: p.ask.removed,
                questions: p.questions.clone(),
                queue: self.pending.len(),
                reminders: p.reminders,
            }),
            ended: self
                .ended
                .iter()
                .map(|e| EndedView {
                    request: e.request.0.clone(),
                    agent: e.session.agent,
                    session: e.session.session_id.clone(),
                    outcome: e.outcome,
                })
                .collect(),
            alerts: self
                .alerts
                .iter()
                .map(|a| AlertView {
                    key: a.key.clone(),
                    seq: a.seq,
                    connector: a.connector.clone(),
                    level: a.level,
                    title: a.title.clone(),
                    detail: a.detail.clone(),
                    link: a.url.is_some(),
                })
                .collect(),
            boards: crate::board::view(self),
            look: self.outfit.worn(self.today),
            focus: self.focus.as_ref().map(SessionRef::from),
            front: self.front().map(SessionRef::from),
            dnd: self.dnd_until.is_some(),
            locked: self.locked,
            digest: crate::away::view(self),
        }
    }
}

/// VS Code and its forks all set `TERM_PROGRAM=vscode`; the binary behind git's askpass names the fork.
pub(crate) fn editor(t: &Terminal) -> Option<&'static str> {
    let env = |k: &str| t.env.get(k).map(String::as_str);
    if env("TERM_PROGRAM") != Some("vscode") && env("VSCODE_PID").is_none() {
        return None;
    }
    let binary = env("VSCODE_GIT_ASKPASS_NODE")
        .and_then(|p| p.rsplit('/').next())
        .unwrap_or_default()
        .to_ascii_lowercase();
    Some(if binary.contains("cursor") || env("CURSOR_TRACE_ID").is_some() {
        "Cursor"
    } else {
        "VS Code"
    })
}

/// What `platform::jump` can act on: a multiplexer's pane (herdr, tmux, wezterm, kitty with its
/// socket), or, with the agent's process known, a window on KDE or in an X11 session. Elsewhere
/// on Wayland `jump` still tries an XWayland window, but that is not promised (ADR 0011). Keep in
/// step with `platform::jump::methods`.
fn raises(t: &Terminal) -> bool {
    let has = |k: &str| t.env.get(k).is_some_and(|v| !v.trim().is_empty());
    let env = |k: &str| t.env.get(k).map_or("", String::as_str);
    let pane = ["HERDR_WORKSPACE_ID", "TMUX_PANE", "WEZTERM_PANE"]
        .into_iter()
        .any(has)
        || (has("KITTY_WINDOW_ID") && has("KITTY_LISTEN_ON"));
    let kde = env("XDG_CURRENT_DESKTOP")
        .split(':')
        .any(|d| d.eq_ignore_ascii_case("kde"));
    let x11 = env("XDG_SESSION_TYPE").eq_ignore_ascii_case("x11")
        || (!has("XDG_SESSION_TYPE") && !has("WAYLAND_DISPLAY") && has("DISPLAY"));
    pane || (t.pid.is_some() && (kde || x11))
}

/// The session's kept steps as text, oldest first; those a rule allowed say so.
fn steps(state: &State, s: &crate::Session) -> Vec<String> {
    // Step numbers count from 1; the kept ones are the last `steps.len()`.
    let first = s.step_count + 1 - s.steps.len() as u32;
    s.steps
        .iter()
        .enumerate()
        .map(|(i, st)| {
            let text = i18n::step(state.lang, st.activity, &st.tool, st.detail.as_deref());
            if s.ruled.contains(&(first + i as u32)) {
                i18n::ruled_step(state.lang, &text)
            } else {
                text
            }
        })
        .collect()
}

/// One per kept step, in the same order as [`steps`].
fn diffs(s: &crate::Session) -> Vec<Option<DiffSummary>> {
    let first = s.step_count + 1 - s.steps.len() as u32;
    (0..s.steps.len() as u32)
        .map(|i| {
            let n = first + i;
            s.diffs.iter().find(|(m, _)| *m == n).map(|(_, d)| DiffSummary {
                step: n,
                added: d.added(),
                removed: d.removed(),
                files: d.files.len(),
            })
        })
        .collect()
}

/// `ui/src/view.gen.ts`: these types as TypeScript, so the island cannot drift from them.
#[cfg(test)]
mod ts {
    use serde::Serialize;
    use ts_rs::{Config, TS};

    use super::*;
    use crate::board::{BoardView, Checks, Group, RowView, Verdict};
    use crate::{Choice, FileDiff, Hunk, looks::Outfit};

    const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ui/src/view.gen.ts");
    const REGEN: &str = "VULTS_REGEN=1 cargo test -p vults-core view_ts";

    /// `AgentKind` lives in the protocol crate, which stays free of ts-rs: a twin, held to it below.
    #[derive(Serialize, TS)]
    #[serde(rename_all = "lowercase")]
    pub(super) enum AgentKind {
        Claude,
        Codex,
        Gemini,
        OpenCode,
        Other,
    }

    #[test]
    fn agent_kind_twin_serializes_the_same() {
        use vults_protocol::AgentKind as Real;
        for real in [
            Real::Claude,
            Real::Codex,
            Real::Gemini,
            Real::OpenCode,
            Real::Other,
        ] {
            // Exhaustive: a new agent fails to build here until the twin has it.
            let twin = match real {
                Real::Claude => AgentKind::Claude,
                Real::Codex => AgentKind::Codex,
                Real::Gemini => AgentKind::Gemini,
                Real::OpenCode => AgentKind::OpenCode,
                Real::Other => AgentKind::Other,
            };
            assert_eq!(
                serde_json::to_string(&real).expect("agent kind"),
                serde_json::to_string(&twin).expect("twin")
            );
        }
    }

    fn typescript() -> String {
        // `seq` is a u64 but stays far below 2^53: a plain number, as JSON delivers it.
        let cfg = Config::new().with_large_int("number");
        let decls = [
            AgentKind::decl(&cfg),
            Status::decl(&cfg),
            Attention::decl(&cfg),
            Activity::decl(&cfg),
            AlertLevel::decl(&cfg),
            Outfit::decl(&cfg),
            ViewModel::decl(&cfg),
            SessionView::decl(&cfg),
            DiffSummary::decl(&cfg),
            ApprovalView::decl(&cfg),
            EndedView::decl(&cfg),
            SessionRef::decl(&cfg),
            Outcome::decl(&cfg),
            crate::silence::Silence::decl(&cfg),
            crate::away::DigestView::decl(&cfg),
            Question::decl(&cfg),
            Choice::decl(&cfg),
            AlertView::decl(&cfg),
            BoardView::decl(&cfg),
            RowView::decl(&cfg),
            Group::decl(&cfg),
            Checks::decl(&cfg),
            Verdict::decl(&cfg),
            Diff::decl(&cfg),
            FileDiff::decl(&cfg),
            Hunk::decl(&cfg),
        ];
        let mut out = format!("// Generated from crates/core by `{REGEN}`. Do not edit.\n");
        for d in decls {
            out.push('\n');
            // A declaration starts with `type`, or with its doc comment and then `type`.
            let d = match d.strip_prefix("type ") {
                Some(rest) => format!("export type {rest}"),
                None => d.replacen("\ntype ", "\nexport type ", 1),
            };
            // ts-rs leaves trailing spaces; an editor trimming them must not make the file stale.
            for line in d.lines() {
                out.push_str(line.trim_end());
                out.push('\n');
            }
        }
        out
    }

    #[test]
    fn view_ts_is_up_to_date() {
        let ts = typescript();
        if std::env::var_os("VULTS_REGEN").is_some() {
            std::fs::write(PATH, &ts).expect("write ui/src/view.gen.ts");
        }
        let on_disk = std::fs::read_to_string(PATH).unwrap_or_default();
        assert!(on_disk == ts, "ui/src/view.gen.ts is stale: run `{REGEN}`");
    }
}
