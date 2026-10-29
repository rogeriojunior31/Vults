//! Numeric telemetry for sessions already known to the flock. These reads neither discover
//! sessions nor infer activity/permissions from a transcript. No credentials or message text leave.

mod other;
use crate::prices::{Catalog, Tokens};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::{File, Metadata};
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde_json::Value;
use vults_core::SessionKey;
use vults_core::metrics::{CostSource, MetricInfo, TimeKind};
use vults_protocol::AgentKind;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Snapshot {
    /// Explicit durable lifecycle state, never inferred from a quiet transcript.
    pub session_closed: bool,
    pub info: MetricInfo,
    pub partial: bool,
    pub reset_context: bool,
    pub cost_usd: Option<f64>,
    pub estimated_cost_usd: Option<f64>,
    pub cost_estimated: bool,
    pub price_checked_at: Option<i64>,
    pub elapsed_ms: Option<u64>,
    pub context_percent: Option<f64>,
    pub context_tokens: Option<u64>,
}

impl Snapshot {
    pub fn claude(payload: &Value) -> Self {
        let usage = &payload["context_window"]["current_usage"];
        let context_tokens = usage["input_tokens"].as_u64().and_then(|input| {
            input
                .checked_add(usage["cache_creation_input_tokens"].as_u64().unwrap_or(0))?
                .checked_add(usage["cache_read_input_tokens"].as_u64().unwrap_or(0))
        });
        Self {
            session_closed: false,
            partial: true,
            reset_context: payload["context_window"]
                .get("current_usage")
                .is_some_and(Value::is_null),
            info: MetricInfo {
                model: payload["model"]["id"].as_str().map(str::to_string),
                time_kind: Some(TimeKind::Execution),
                cost_source: Some(CostSource::Agent),

                ..MetricInfo::default()
            },
            cost_usd: crate::usage::claude_cost(payload),
            estimated_cost_usd: None,
            cost_estimated: false,
            price_checked_at: None,
            elapsed_ms: crate::usage::claude_elapsed(payload),
            context_percent: crate::usage::claude_context(payload),
            context_tokens,
        }
    }

    pub fn antigravity(payload: &Value) -> Self {
        let mut report = Self::claude(payload);
        report.cost_usd = payload["cost"]["total_usd"]
            .as_f64()
            .filter(|n| n.is_finite() && *n >= 0.0);
        report.info.cost_source = report.cost_usd.map(|_| CostSource::AgentEstimate);
        report.info.time_kind = report.elapsed_ms.map(|_| TimeKind::Execution);
        report
    }

    pub fn cursor(payload: &Value) -> Option<Self> {
        let tokens = payload["context_tokens"].as_u64();
        let percent = payload["context_usage_percent"]
            .as_f64()
            .filter(|n| (0.0..=100.0).contains(n));
        if tokens.is_none() && percent.is_none() {
            return None;
        }
        Some(Self {
            partial: true,
            context_tokens: tokens,
            context_percent: percent,
            info: MetricInfo {
                model: payload["model"].as_str().map(str::to_string),
                ..MetricInfo::default()
            },
            ..Self::default()
        })
    }

    /// The last completed Hermes API request: current context, not session-wide token totals.
    pub fn hermes(payload: &Value) -> Option<Self> {
        let tokens = payload["context_tokens"].as_u64();
        let elapsed_ms = payload["elapsed_ms"].as_u64();
        if tokens.is_none() && elapsed_ms.is_none() {
            return None;
        }
        let context_percent = tokens
            .zip(payload["context_limit"].as_u64())
            .filter(|(_, limit)| *limit > 0)
            .map(|(used, limit)| (used as f64 / limit as f64 * 100.0).min(100.0));
        Some(Self {
            partial: true,
            info: MetricInfo {
                time_kind: Some(TimeKind::ModelResponse),
                ..MetricInfo::default()
            },
            context_tokens: tokens,
            context_percent,
            elapsed_ms,
            ..Self::default()
        })
    }

    pub fn input(self, session: SessionKey) -> vults_core::Input {
        vults_core::Input::SessionMetrics {
            session,
            info: self.info,
            partial: self.partial,
            reset_context: self.reset_context,
            cost_usd: self.cost_usd.or(self.estimated_cost_usd),
            cost_estimated: self.cost_estimated,
            price_checked_at: self.price_checked_at,
            elapsed_ms: self.elapsed_ms,
            context_percent: self.context_percent,
            context_tokens: self.context_tokens,
        }
    }
}

pub fn readable(key: &SessionKey) -> bool {
    matches!(
        key.agent,
        AgentKind::Codex | AgentKind::OpenCode | AgentKind::Qwen | AgentKind::Copilot
    ) || key.agent == AgentKind::Other
        && ["hermes/", "kimi/", "droid/"]
            .iter()
            .any(|prefix| key.session_id.starts_with(prefix))
}

pub fn is_hermes(key: &SessionKey) -> bool {
    key.agent == AgentKind::Other
        && key
            .session_id
            .strip_prefix("hermes/")
            .is_some_and(|id| !id.is_empty())
}

fn database(path: &Path) -> Option<Connection> {
    let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).ok()?;
    db.busy_timeout(std::time::Duration::from_millis(50)).ok()?;
    Some(db)
}

/// Codex versions its index's filename. Select the newest schema without hard-coding `state_5`.
fn codex_index(home: &Path) -> Option<PathBuf> {
    std::fs::read_dir(home)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name();
            let version: u32 = name
                .to_str()?
                .strip_prefix("state_")?
                .strip_suffix(".sqlite")?
                .parse()
                .ok()?;
            Some((version, entry.path()))
        })
        .max_by_key(|(version, _)| *version)
        .map(|(_, path)| path)
}

#[derive(Debug, Default)]
struct CodexTotals {
    model: String,
    provider: String,
    previous: Option<Tokens>,
    last_sample: Option<Tokens>,
    cost: f64,
    priced: bool,
    seen_usage: bool,
    turns: BTreeMap<String, (Option<i64>, Option<u64>)>,
    current_turn: Option<String>,
}

impl CodexTotals {
    fn record(&mut self, record: &Value, prices: Option<&Catalog>) {
        let payload = &record["payload"];
        if record["type"] == "session_meta" {
            self.provider = payload["model_provider"].as_str().unwrap_or("openai").into();
        }
        if record["type"] == "turn_context" {
            self.model = payload["model"].as_str().unwrap_or("").into();
            if let Some(provider) = payload["model_provider"].as_str() {
                self.provider = provider.into();
            }
        }
        if record["type"] != "event_msg" {
            return;
        }
        match payload["type"].as_str() {
            Some("task_started" | "task_complete" | "task_completed" | "turn_aborted") => {
                let start = payload["started_at"].as_i64().filter(|n| *n >= 0);
                let id = payload["turn_id"]
                    .as_str()
                    .map(str::to_string)
                    .or_else(|| {
                        start.and_then(|start| {
                            self.turns
                                .iter()
                                .find(|(_, turn)| turn.0 == Some(start) && turn.1.is_none())
                                .map(|(id, _)| id.clone())
                        })
                    })
                    .or_else(|| self.current_turn.clone());
                let Some(id) = id else {
                    return;
                };
                if payload["type"] == "task_started" {
                    self.current_turn = Some(id.clone());
                } else if self.current_turn.as_ref() == Some(&id) {
                    self.current_turn = None;
                }
                let turn = self.turns.entry(id).or_default();
                if let Some(start) = payload["started_at"].as_i64().filter(|n| *n >= 0) {
                    turn.0 = Some(start);
                } else if payload["type"] == "task_started" && turn.0.is_none() {
                    turn.0 = record["timestamp"].as_str().and_then(crate::usage::reset_seconds);
                }
                if payload["type"] != "task_started" {
                    turn.1 = payload["duration_ms"].as_u64().or_else(|| {
                        let end = payload["completed_at"]
                            .as_i64()
                            .or_else(|| record["timestamp"].as_str().and_then(crate::usage::reset_seconds))?;
                        u64::try_from(end.checked_sub(turn.0?)?).ok()?.checked_mul(1000)
                    });
                }
            }
            Some("token_count") => {
                let info = &payload["info"];
                let Some(last) = Tokens::read(&info["last_token_usage"]) else {
                    return;
                };
                let total = Tokens::read(&info["total_token_usage"]);
                let delta = match (total, self.previous) {
                    (Some(now), Some(previous)) if now == previous => return,
                    (Some(now), Some(previous)) => now.delta(previous).unwrap_or(last),
                    (Some(_), None) => last,
                    (None, _) if self.last_sample == Some(last) => return,
                    (None, _) => last,
                };
                if !self.seen_usage {
                    self.priced = true;
                }
                self.seen_usage = true;
                self.previous = total;
                self.last_sample = Some(last);
                let cost =
                    prices.and_then(|prices| prices.estimate(&self.provider, &self.model, delta, last.input));
                if let Some(cost) = cost {
                    self.cost += cost;
                } else {
                    self.priced = false;
                }
            }
            _ => {}
        }
    }

    fn elapsed(&self, now_ms: u64) -> Option<u64> {
        if self.turns.is_empty() {
            return None;
        }
        self.turns.values().try_fold(0u64, |sum, (start, duration)| {
            let elapsed = duration
                .or_else(|| Some(now_ms.saturating_sub(u64::try_from((*start)?).ok()?.checked_mul(1000)?)))?;
            sum.checked_add(elapsed)
        })
    }
}

/// Qwen persists per-call deltas. Keep consumption separate from its latest prompt size.
#[derive(Debug, Default)]
struct QwenTotals {
    tokens: Tokens,
    seen: HashSet<String>,
    seen_usage: bool,
    priced: bool,
    cost: f64,
    elapsed_ms: Option<u64>,
    model: String,
    provider: String,
    native_provider: Option<String>,
    context: Option<u64>,
}

impl QwenTotals {
    fn record(&mut self, record: &Value, session: &str, prices: Option<&Catalog>) {
        let ledger = record["schemaVersion"] == 1 && record["sessionId"].as_str() == Some(session);
        let transcript = record["type"] == "assistant" && record.get("usageMetadata").is_some();
        if !ledger && !transcript {
            return;
        }
        // Native UUIDs identify completed responses, independently of equal token counts.
        let id = record[if ledger { "id" } else { "uuid" }].as_str();
        if let Some(id) = id
            && self.seen.contains(id)
        {
            return;
        }
        let usage = &record["usageMetadata"];
        let input = if ledger {
            record["inputTokens"].as_u64()
        } else {
            usage["promptTokenCount"].as_u64()
        };
        let output = if ledger {
            record["outputTokens"].as_u64()
        } else {
            usage["candidatesTokenCount"].as_u64()
        };
        let thoughts = if ledger {
            &record["thoughtsTokens"]
        } else {
            &usage["thoughtsTokenCount"]
        };
        let cached = if ledger {
            &record["cachedTokens"]
        } else {
            &usage["cachedContentTokenCount"]
        };
        let Some((input, output, cached)) = input
            .zip(output)
            .zip(cached.as_u64().or_else(|| cached.is_null().then_some(0)))
            .and_then(|((input, output), cached)| {
                Some((
                    input,
                    output.checked_add(thoughts.as_u64().or_else(|| thoughts.is_null().then_some(0))?)?,
                    cached,
                ))
            })
        else {
            return;
        };
        if cached > input {
            return;
        }
        let value = &usage["cacheCreationTokenCount"];
        let Some(written) = value.as_u64().or_else(|| value.is_null().then_some(0)) else {
            return;
        };
        if cached.checked_add(written).is_none_or(|n| n > input) {
            return;
        }
        let tokens = Tokens {
            input,
            output,
            cached,
            written,
        };
        let Some(sum) = self
            .tokens
            .input
            .checked_add(input)
            .zip(self.tokens.output.checked_add(output))
            .zip(self.tokens.cached.checked_add(cached))
        else {
            return;
        };
        if let Some(id) = id {
            // ponytail: dedup the latest 4096 native IDs; a disk rewrite resets all totals.
            if self.seen.len() >= 4096 {
                self.seen.clear();
            }
            self.seen.insert(id.to_string());
        }
        self.tokens = Tokens {
            input: sum.0.0,
            output: sum.0.1,
            cached: sum.1,
            written: match self.tokens.written.checked_add(written) {
                Some(n) => n,
                None => return,
            },
        };
        let model = record["model"].as_str().unwrap_or("").to_string();
        let native_provider = record["provider"].as_str().map(str::to_string);
        let provider = native_provider
            .as_deref()
            .or_else(|| prices?.provider_for(&model))
            .unwrap_or("")
            .to_string();
        let main_context = (!ledger || record["source"].as_str() == Some("main"))
            && record["context_report"].as_bool() != Some(false);
        if record["context_reset"] == true {
            self.context = None;
        }
        if main_context {
            self.context = Some(input);
            self.model = model.clone();
            self.provider = provider.clone();
            self.native_provider = native_provider;
        }
        if !self.seen_usage {
            self.priced = true;
        }
        self.seen_usage = true;
        if let Some(cost) = prices.and_then(|book| book.estimate(&provider, &model, tokens, input)) {
            self.cost += cost;
        } else {
            self.priced = false;
        }
        if ledger && let Some(duration) = record["apiDurationMs"].as_u64() {
            self.elapsed_ms = self.elapsed_ms.unwrap_or(0).checked_add(duration);
        }
    }

    fn snapshot(&self, prices: Option<&Catalog>, now: i64) -> Snapshot {
        Snapshot {
            info: MetricInfo {
                model: (!self.model.is_empty()).then(|| self.model.clone()),
                provider: self.native_provider.clone(),
                price_provider: (!self.provider.is_empty()).then(|| self.provider.clone()),
                time_kind: self.elapsed_ms.map(|_| TimeKind::ModelResponse),
                cost_source: Some(CostSource::Catalog),
                input_tokens: self.seen_usage.then_some(self.tokens.input),
                output_tokens: self.seen_usage.then_some(self.tokens.output),
                cached_tokens: self.seen_usage.then_some(self.tokens.cached),
                cache_write_tokens: self.seen_usage.then_some(self.tokens.written),
                ..MetricInfo::default()
            },
            estimated_cost_usd: prices
                .filter(|p| p.fresh(now))
                .filter(|_| self.seen_usage && self.priced)
                .map(|_| self.cost),
            cost_estimated: true,
            price_checked_at: prices.map(|p| p.checked_at),
            elapsed_ms: self.elapsed_ms,
            context_tokens: self.context,
            ..Snapshot::default()
        }
    }
}

/// Stamp the file itself: replacing a log can preserve its size and modification time.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Stamp {
    len: u64,
    modified: Option<SystemTime>,
    created: Option<SystemTime>,
    #[cfg(unix)]
    identity: (u64, u64),
    #[cfg(unix)]
    changed: (i64, i64),
}

impl Stamp {
    fn of(meta: &Metadata) -> Self {
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;
        Self {
            len: meta.len(),
            modified: meta.modified().ok(),
            created: meta.created().ok(),
            #[cfg(unix)]
            identity: (meta.dev(), meta.ino()),
            #[cfg(unix)]
            changed: (meta.ctime(), meta.ctime_nsec()),
        }
    }

    fn same_file(&self, other: &Self) -> bool {
        #[cfg(unix)]
        if self.identity != other.identity {
            return false;
        }
        self.created == other.created
    }
}

#[derive(Debug)]
struct Cached {
    path: PathBuf,
    stamp: Option<Stamp>,
    offset: u64,
    checkpoint: Vec<u8>,
    pending: Vec<u8>,
    oversized: bool,
    snapshot: Option<Snapshot>,
    totals: CodexTotals,
    qwen: QwenTotals,
    session_id: String,
}

impl Cached {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            stamp: None,
            offset: 0,
            checkpoint: Vec::new(),
            pending: Vec::new(),
            oversized: false,
            snapshot: None,
            totals: CodexTotals::default(),
            qwen: QwenTotals::default(),
            session_id: String::new(),
        }
    }

    fn read(&mut self, file: &mut File, agent: AgentKind, prices: Option<&Catalog>) -> io::Result<()> {
        let stamp = Stamp::of(&file.metadata()?);
        let rewritten = self.stamp.as_ref().is_some_and(|old| {
            !old.same_file(&stamp) || stamp.len < old.len || (stamp.len == old.len && *old != stamp)
        });
        // As in Magpie's session reader, keep a cursor plus a prefix checkpoint. A rewritten
        // suffix must not be joined to an unfinished JSON line from the previous file.
        let continued =
            !rewritten && self.offset <= stamp.len && checkpoint(file, self.offset)? == self.checkpoint;
        if !continued {
            self.offset = 0;
            self.pending.clear();
            self.oversized = false;
            self.snapshot = None;
            self.totals = CodexTotals::default();
            self.qwen = QwenTotals::default();
        }
        file.seek(SeekFrom::Start(self.offset))?;
        {
            // Read each appended byte once, including partial lines. Memory remains bounded
            // even when a tool appends megabytes of output or an image on a single JSON line.
            const MAX_RECORD: u64 = 1024 * 1024;
            let marker: &[u8] = match agent {
                AgentKind::Qwen => b"usageMetadata",
                AgentKind::Copilot => b"session.shutdown",
                AgentKind::Other => b"usage.record",
                _ => b"token_count",
            };
            let mut reader = BufReader::new((&mut *file).take(stamp.len - self.offset));
            loop {
                let count = (&mut reader)
                    .take(MAX_RECORD + 1)
                    .read_until(b'\n', &mut self.pending)?;
                if count == 0 {
                    break;
                }
                self.offset += count as u64;
                if self.pending.last() == Some(&b'\n') {
                    if !self.oversized && self.pending.len() as u64 <= MAX_RECORD {
                        let relevant = self.pending.windows(marker.len()).any(|w| w == marker)
                            || agent == AgentKind::Other && self.pending.windows(6).any(|w| w == b"forked")
                            || agent == AgentKind::Qwen
                                && self.pending.windows(13).any(|w| w == b"apiDurationMs")
                            || agent == AgentKind::Codex
                                && [
                                    "session_meta",
                                    "turn_context",
                                    "task_started",
                                    "task_complete",
                                    "turn_aborted",
                                ]
                                .iter()
                                .any(|marker| {
                                    self.pending.windows(marker.len()).any(|w| w == marker.as_bytes())
                                });
                        if relevant && let Ok(record) = serde_json::from_slice::<Value>(&self.pending) {
                            if agent == AgentKind::Codex {
                                self.totals.record(&record, prices);
                            } else if agent == AgentKind::Qwen {
                                self.qwen.record(&record, &self.session_id, prices);
                            }
                            if agent == AgentKind::Other {
                                if record["type"] == "forked" {
                                    self.qwen = QwenTotals::default();
                                }
                                if let Some(usage) = other::kimi_usage(&record) {
                                    self.qwen.record(&usage, "", prices);
                                }
                            }
                            let snapshot = match agent {
                                AgentKind::Copilot => other::copilot(&record, prices),
                                AgentKind::Other => Some(self.qwen.snapshot(prices, epoch())),
                                AgentKind::Qwen => qwen_snapshot(&record),
                                _ => codex_snapshot(&record),
                            };
                            if let Some(snapshot) = snapshot {
                                self.snapshot = Some(snapshot);
                            }
                        }
                    }
                    self.pending.clear();
                    self.oversized = false;
                } else if self.pending.len() as u64 > MAX_RECORD {
                    self.pending.clear();
                    self.oversized = true;
                }
            }
        }
        self.checkpoint = checkpoint(file, self.offset)?;
        self.stamp = Some(stamp);
        Ok(())
    }
}

fn checkpoint(file: &mut File, offset: u64) -> io::Result<Vec<u8>> {
    let mut bytes = vec![0; offset.min(256) as usize];
    file.seek(SeekFrom::Start(offset.saturating_sub(256)))?;
    file.read_exact(&mut bytes)?;
    Ok(bytes)
}

/// The worker owns this cache; unchanged transcripts are only stat'ed, never read again.
#[derive(Debug, Default)]
pub struct Reader {
    logs: HashMap<SessionKey, Cached>,
    prices: Option<Arc<Catalog>>,
    qwen_ledgers: HashMap<(SessionKey, PathBuf), Cached>,
}

impl Reader {
    pub fn set_prices(&mut self, prices: Option<Arc<Catalog>>) {
        if self
            .prices
            .as_ref()
            .zip(prices.as_ref())
            .is_some_and(|(a, b)| Arc::ptr_eq(a, b))
        {
            return;
        }
        if self.prices.as_ref().map(|p| &p.models) != prices.as_ref().map(|p| &p.models) {
            self.logs.clear();
            self.qwen_ledgers.clear();
        }
        self.prices = prices;
    }

    fn qwen_ledger(&mut self, dirs: &crate::Dirs, key: &SessionKey) -> Option<Snapshot> {
        let paths = qwen_ledger_paths(dirs)?;
        let mut totals = QwenTotals::default();
        for path in &paths {
            let cache = self
                .qwen_ledgers
                .entry((key.clone(), path.clone()))
                .or_insert_with(|| Cached::new(path.clone()));
            cache.session_id = key.session_id.clone();
            let mut file = File::open(path).ok()?;
            if cache.stamp.as_ref() != Some(&Stamp::of(&file.metadata().ok()?)) {
                cache
                    .read(&mut file, AgentKind::Qwen, self.prices.as_deref())
                    .ok()?;
            }
            let part = &cache.qwen;
            if !part.seen_usage {
                continue;
            }
            if !totals.seen_usage {
                totals.priced = true;
            }
            totals.seen_usage = true;
            totals.priced &= part.priced;
            totals.cost += part.cost;
            totals.tokens.input = totals.tokens.input.checked_add(part.tokens.input)?;
            totals.tokens.output = totals.tokens.output.checked_add(part.tokens.output)?;
            totals.tokens.cached = totals.tokens.cached.checked_add(part.tokens.cached)?;
            if let Some(ms) = part.elapsed_ms {
                totals.elapsed_ms = Some(totals.elapsed_ms.unwrap_or(0).checked_add(ms)?);
            }
            if part.context.is_some() {
                totals.context = part.context;
            }
            if !part.model.is_empty() {
                totals.model.clone_from(&part.model);
                totals.provider.clone_from(&part.provider);
                totals.native_provider.clone_from(&part.native_provider);
            }
            totals.tokens.written = totals.tokens.written.checked_add(part.tokens.written)?;
        }
        self.qwen_ledgers
            .retain(|(owner, path), _| owner != key || paths.contains(path));
        totals
            .seen_usage
            .then(|| totals.snapshot(self.prices.as_deref(), epoch()))
    }

    pub fn read(
        &mut self,
        dirs: &crate::Dirs,
        data_home: &Path,
        sessions: &[SessionKey],
    ) -> Vec<(SessionKey, Snapshot)> {
        self.logs.retain(|key, _| sessions.contains(key));
        self.qwen_ledgers.retain(|(key, _), _| sessions.contains(key));
        let codex_home = crate::codex_home(dirs);
        let codex = sessions
            .iter()
            .any(|s| s.agent == AgentKind::Codex)
            .then(|| database(&codex_index(&codex_home)?))
            .flatten();
        let opencode = sessions
            .iter()
            .any(|s| s.agent == AgentKind::OpenCode)
            .then(|| database(&data_home.join("opencode/opencode.db")))
            .flatten();
        let hermes = sessions
            .iter()
            .any(is_hermes)
            .then(|| database(&dirs.agent("HERMES_HOME", ".hermes").join("state.db")))
            .flatten();
        sessions.iter().filter_map(|key| {
            if key.agent==AgentKind::Qwen && let Some(report)=self.qwen_ledger(dirs,key) { return Some((key.clone(),report)); }
            let mut snapshot = match key.agent {
                AgentKind::Copilot | AgentKind::Other if !is_hermes(key) => {
                    let path = other::file(dirs,key)?;
                    if key.session_id.starts_with("droid/") { other::droid(&path)? }
                    else {
                        let cache = self.logs.entry(key.clone()).or_insert_with(|| Cached::new(path.clone()));
                        if cache.path != path { *cache = Cached::new(path.clone()); }
                        let mut file = File::open(&path).ok()?;
                        if cache.stamp.as_ref() != Some(&Stamp::of(&file.metadata().ok()?)) { cache.read(&mut file,key.agent,self.prices.as_deref()).ok()?; }
                        if key.agent == AgentKind::Other { cache.qwen.snapshot(self.prices.as_deref(),epoch()) }
                        else { cache.snapshot.clone().unwrap_or_default() }
                    }
                }
                AgentKind::Codex | AgentKind::Qwen => {
                    let canonical = if key.agent == AgentKind::Codex {
                        let path: String = codex.as_ref()?.query_row(
                            "SELECT rollout_path FROM threads WHERE id = ?1", [&key.session_id], |row| row.get(0),
                        ).ok()?;
                        let canonical = PathBuf::from(path).canonicalize().ok()?;
                        let home = codex_home.canonicalize().ok()?;
                        if !canonical.starts_with(home.join("sessions")) && !canonical.starts_with(home.join("archived_sessions")) { return None; }
                        canonical
                    } else {
                        let home=dirs.agent("QWEN_HOME", ".qwen");
                        let roots: Vec<_> = [home.join("projects"),home.join("tmp")].into_iter().filter_map(|p|p.canonicalize().ok()).collect();
                        let path = self.logs.get(key).filter(|cache| roots.iter().any(|root|cache.path.starts_with(root)) && cache.path.is_file())
                            .map(|cache| cache.path.clone()).or_else(|| roots.iter().find_map(|root|qwen_path(root, &key.session_id)))?;
                        let canonical = path.canonicalize().ok()?;
                        if !roots.iter().any(|root|canonical.starts_with(root)) { return None; }
                        canonical
                    };
                    let meta = std::fs::metadata(&canonical).ok()?;
                    if !meta.is_file() { return None; }
                    let cached = self.logs.entry(key.clone()).or_insert_with(|| Cached::new(canonical.clone()));
                    if cached.path != canonical {
                        *cached = Cached::new(canonical.clone());
                    }
                    cached.session_id = key.session_id.clone();
                    if cached.stamp.as_ref() != Some(&Stamp::of(&meta)) {
                        let mut file = File::open(&canonical).ok()?;
                        cached.read(&mut file, key.agent, self.prices.as_deref()).ok()?;
                    }
                    // An existing file with no usage yet (or a rewritten/cleared one) reports
                    // unknown, so an old context gauge cannot survive a reset.
                    let mut report=cached.snapshot.clone().unwrap_or_default();
                    if key.agent==AgentKind::Codex {
                        let now=SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
                        report.elapsed_ms=cached.totals.elapsed(u64::try_from(now.as_millis()).unwrap_or(u64::MAX));
                        report.info = MetricInfo {
                            model: (!cached.totals.model.is_empty()).then(|| cached.totals.model.clone()),
                            provider: (!cached.totals.provider.is_empty()).then(|| cached.totals.provider.clone()),
                            time_kind: Some(TimeKind::Execution), cost_source: Some(CostSource::Catalog),
                            input_tokens: cached.totals.previous.map(|t| t.input), output_tokens: cached.totals.previous.map(|t| t.output),
                            cached_tokens: cached.totals.previous.map(|t| t.cached),
                            cache_write_tokens: cached.totals.previous.map(|t| t.written), ..MetricInfo::default()
                        };
                        report.cost_estimated=true;
                        report.price_checked_at=self.prices.as_ref().map(|p|p.checked_at);
                        report.estimated_cost_usd=self.prices.as_ref().filter(|p|p.fresh(now.as_secs() as i64))
                            .filter(|_|cached.totals.seen_usage && cached.totals.priced).map(|_|cached.totals.cost);
                    } else {
                        report = cached.qwen.snapshot(self.prices.as_deref(), epoch());
                        if report.context_tokens.is_none() { report.context_tokens = cached.snapshot.as_ref().and_then(|s| s.context_tokens); }
                    }
                    report
                }
                AgentKind::Other if is_hermes(key) => {
                    let row = hermes.as_ref()?.query_row(
                        "SELECT actual_cost_usd, estimated_cost_usd, cost_status FROM sessions WHERE id = ?1",
                        [key.session_id.strip_prefix("hermes/")?],
                        |row| Ok((row.get::<_, Option<f64>>(0)?,row.get::<_, Option<f64>>(1)?,row.get::<_, Option<String>>(2)?)),
                    ).optional().ok()?;
                    let Some((actual, estimated, status)) = row else { return Some((key.clone(), Snapshot::default())); };
                    let mut info = hermes_info(hermes.as_ref()?, key.session_id.strip_prefix("hermes/")?);
                    let valid = |n: f64| n.is_finite() && n >= 0.0;
                    let cost = actual.filter(|n| valid(*n)).or_else(|| {
                        match status.as_deref() { Some("estimated") => estimated.filter(|n| valid(*n)), Some("included") => Some(0.0), _ => None }
                    });
                    info.cost_source = cost.map(|_| if actual.is_some() || status.as_deref() == Some("included") { CostSource::Agent } else { CostSource::AgentEstimate });
                    let session_closed=hermes.as_ref()?.query_row("SELECT ended_at FROM sessions WHERE id=?1",[key.session_id.strip_prefix("hermes/")?],|row|row.get::<_,Option<f64>>(0)).ok().flatten().is_some_and(|t|t.is_finite() && t>0.0);
                    Snapshot { session_closed, cost_usd: cost, info, ..Snapshot::default() }
                }
                AgentKind::OpenCode => {
                    let db = opencode.as_ref()?;
                    let cost = db.query_row::<f64, _, _>(
                        "SELECT cost FROM session WHERE id = ?1", [&key.session_id], |row| row.get(0),
                    ).or_else(|_| db.query_row(
                        // OpenCode 1 kept cost on messages rather than the session row.
                        "SELECT SUM(json_extract(data, '$.cost')) FROM message WHERE session_id = ?1 AND json_extract(data, '$.role') = 'assistant'",
                        [&key.session_id], |row| row.get(0),
                    )).ok().filter(|cost| cost.is_finite() && *cost >= 0.0);
                    let message: Option<String> = db.query_row(
                        "SELECT data FROM message WHERE session_id = ?1 AND json_extract(data, '$.role') = 'assistant' AND json_extract(data, '$.time.completed') IS NOT NULL ORDER BY time_created DESC, id DESC LIMIT 1",
                        [&key.session_id], |row| row.get(0),
                    ).ok();
                    let message = message.and_then(|raw| serde_json::from_str::<Value>(&raw).ok());
                    Snapshot {
                        info: opencode_info(db, &key.session_id, message.as_ref()),
                        cost_usd: cost,
                        elapsed_ms: opencode_elapsed(db, &key.session_id),
                        context_tokens: message.as_ref().and_then(opencode_tokens),
                        ..Snapshot::default()
                    }
                }
                _ => return None,
            };
            if snapshot.cost_estimated {
                snapshot.price_checked_at=self.prices.as_ref().map(|p|p.checked_at);
                if !self.prices.as_ref().is_some_and(|p|p.fresh(epoch())) { snapshot.estimated_cost_usd=None; }
            }
            Some((key.clone(), snapshot))
        }).collect()
    }
}

/// Union of completed model-response intervals. Excludes gaps/tools and overlapping responses.
/// This is response time, not a claim about the agent's full wall-clock running time.
fn opencode_elapsed(db: &Connection, session: &str) -> Option<u64> {
    let mut query = db.prepare("SELECT json_extract(data, '$.time.created'), json_extract(data, '$.time.completed')
        FROM message WHERE session_id = ?1 AND json_extract(data, '$.role') = 'assistant'
        AND json_extract(data, '$.time.created') IS NOT NULL AND json_extract(data, '$.time.completed') IS NOT NULL
        ORDER BY json_extract(data, '$.time.created')").ok()?;
    let rows = query
        .query_map([session], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })
        .ok()?;
    let mut end = 0;
    let mut total = None;
    for row in rows {
        let (start, finish) = row.ok()?;
        if start < 0 || finish < start {
            return None;
        }
        let duration = u64::try_from(finish.saturating_sub(start.max(end))).ok()?;
        total = Some(total.unwrap_or(0u64).checked_add(duration)?);
        end = end.max(finish);
    }
    total
}

fn opencode_tokens(message: &Value) -> Option<u64> {
    let tokens = &message["tokens"];
    if let Some(total) = tokens["total"].as_u64() {
        return Some(total);
    }
    // OpenCode separates reasoning from output and cache from input. Prefer its reported total.
    // https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/session/session.ts
    tokens["input"]
        .as_u64()?
        .checked_add(tokens["output"].as_u64()?)?
        .checked_add(tokens["reasoning"].as_u64().unwrap_or(0))?
        .checked_add(tokens["cache"]["read"].as_u64().unwrap_or(0))?
        .checked_add(tokens["cache"]["write"].as_u64().unwrap_or(0))
}

fn hermes_info(db: &Connection, id: &str) -> MetricInfo {
    db.query_row("SELECT model, billing_provider, input_tokens, output_tokens, cache_read_tokens, cache_write_tokens FROM sessions WHERE id=?1",[id],|row| {
        let number=|index| -> rusqlite::Result<Option<u64>> { Ok(row.get::<_,Option<i64>>(index)?.and_then(|n| n.try_into().ok())) };
        let input=number(2)?; let read=number(4)?; let written=number(5)?;
        Ok(MetricInfo { model:row.get(0)?,provider:row.get(1)?,
            input_tokens:input.zip(read).zip(written).and_then(|((i,r),w)|i.checked_add(r)?.checked_add(w)),
            output_tokens:number(3)?,cached_tokens:read,cache_write_tokens:written,..MetricInfo::default() })
    }).unwrap_or_default()
}

fn opencode_info(db: &Connection, id: &str, message: Option<&Value>) -> MetricInfo {
    let mut info = MetricInfo {
        time_kind: Some(TimeKind::ModelResponse),
        cost_source: Some(CostSource::Agent),
        ..MetricInfo::default()
    };
    if let Some(message) = message {
        info.model = message["modelID"].as_str().map(str::to_string);
        info.provider = message["providerID"].as_str().map(str::to_string);
    }
    if let Ok((input, output, cached)) = db.query_row(
        "SELECT SUM(json_extract(data, '$.tokens.input') + COALESCE(json_extract(data, '$.tokens.cache.read'),0) + COALESCE(json_extract(data, '$.tokens.cache.write'),0)), SUM(json_extract(data, '$.tokens.output') + COALESCE(json_extract(data, '$.tokens.reasoning'),0)), SUM(json_extract(data, '$.tokens.cache.read')) FROM message WHERE session_id=?1 AND json_extract(data, '$.role')='assistant' AND json_extract(data, '$.time.completed') IS NOT NULL",
        [id], |row| Ok((row.get::<_,Option<i64>>(0)?, row.get::<_,Option<i64>>(1)?, row.get::<_,Option<i64>>(2)?)),
    ) {
        info.input_tokens = input.and_then(|n| n.try_into().ok());
        info.output_tokens = output.and_then(|n| n.try_into().ok());
        info.cached_tokens = cached.and_then(|n| n.try_into().ok());
    }
    info
}

fn epoch() -> i64 {
    SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|t| t.as_secs() as i64)
        .unwrap_or(0)
}

fn qwen_ledger_paths(dirs: &crate::Dirs) -> Option<Vec<PathBuf>> {
    let root = dirs.agent("QWEN_HOME", ".qwen").join("usage");
    let canonical = root.canonicalize().ok()?;
    let mut paths: Vec<_> = std::fs::read_dir(&root)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name();
            let name = name.to_str()?;
            if !name.starts_with("token-usage-") || !name.ends_with(".jsonl") {
                return None;
            }
            let path = entry.path().canonicalize().ok()?;
            (path.starts_with(&canonical) && path.is_file()).then_some(path)
        })
        .collect();
    paths.sort();
    (!paths.is_empty()).then_some(paths)
}

/// Qwen's current JSONL layout names each chat by its session id. Discovery reads filenames,
/// not other conversations, and a known path stays cached until the session disappears.
fn qwen_path(projects: &Path, id: &str) -> Option<PathBuf> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
    {
        return None;
    }
    for entry in std::fs::read_dir(projects).ok()?.filter_map(Result::ok) {
        for folder in [entry.path().join("chats"), entry.path().join("chats/archive")] {
            let path = folder.join(format!("{id}.jsonl"));
            if path.is_file() {
                return Some(path);
            }
        }
    }
    None
}

fn qwen_snapshot(record: &Value) -> Option<Snapshot> {
    if record["type"] != "assistant" {
        return None;
    }
    let usage = record.get("usageMetadata")?;
    let tokens = usage["promptTokenCount"].as_u64()?;
    Some(Snapshot {
        context_tokens: Some(tokens),
        ..Snapshot::default()
    })
}

fn codex_snapshot(record: &Value) -> Option<Snapshot> {
    if record["type"] != "event_msg" || record["payload"]["type"] != "token_count" {
        return None;
    }
    let info = &record["payload"]["info"];
    // Cumulative tokens measure the whole conversation, not current context occupancy.
    let used = info["last_token_usage"]["total_tokens"].as_u64()?;
    let limit = info["model_context_window"].as_u64().filter(|n| *n > 0);
    Some(Snapshot {
        context_tokens: Some(used),
        context_percent: limit.map(|limit| (100.0 * used as f64 / limit as f64).min(100.0)),
        ..Snapshot::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn key(agent: AgentKind, id: &str) -> SessionKey {
        SessionKey {
            agent,
            session_id: id.into(),
        }
    }

    #[test]
    fn event_metrics_preserve_native_units_and_explicit_context_resets() {
        let native = Snapshot::antigravity(
            &json!({"cost":{"total_usd":0.0042,"total_cost_usd":99},"model":{"id":"native-model"},"context_window":{"used_percentage":50,"current_usage":{"input_tokens":3,"cache_read_input_tokens":2,"cache_creation_input_tokens":1}}}),
        );
        assert_eq!(native.cost_usd, Some(0.0042));
        assert_eq!(native.info.cost_source, Some(CostSource::AgentEstimate));
        assert_eq!(native.context_tokens, Some(6));
        assert_eq!(native.elapsed_ms, None);
        let cursor = Snapshot::cursor(&json!({"context_tokens":100,"context_usage_percent":50})).unwrap();
        assert_eq!(cursor.context_percent, Some(50.0));
        assert!(cursor.partial);
        assert_eq!(cursor.cost_usd, None);
        assert!(Snapshot::cursor(&json!({"context_tokens":-1,"context_usage_percent":101})).is_none());
        let reset = Snapshot::claude(&json!({"context_window":{"current_usage":null}}));
        assert!(reset.reset_context);
        let usage = Snapshot::claude(
            &json!({"context_window":{"total_input_tokens":999,"total_output_tokens":111,"current_usage":{"input_tokens":10}}}),
        );
        assert_eq!(
            usage.info.input_tokens, None,
            "Claude status-line context totals are not cumulative session consumption"
        );
    }

    #[test]
    fn codex_cost_uses_deltas_once_and_running_time_follows_native_turns() {
        let prices = Catalog::from_models_dev(
            &json!({"vendor":{"models":{"unit":{"cost":{"input":2,"output":10,"cache_read":0.1}}}}}),
            1000,
            None,
        )
        .unwrap();
        let mut totals = CodexTotals::default();
        totals.record(
            &json!({"type":"session_meta","payload":{"model_provider":"vendor"}}),
            Some(&prices),
        );
        totals.record(
            &json!({"type":"turn_context","payload":{"model":"unit"}}),
            Some(&prices),
        );
        let sample = |input, output, cached| {
            json!({"type":"event_msg","payload":{"type":"token_count","info":{
            "total_token_usage":{"input_tokens":input,"output_tokens":output,"cached_input_tokens":cached},
            "last_token_usage":{"input_tokens":100,"output_tokens":20,"cached_input_tokens":50}}}})
        };
        totals.record(&sample(100, 20, 50), Some(&prices));
        let first = totals.cost;
        totals.record(&sample(100, 20, 50), Some(&prices));
        assert_eq!(
            totals.cost, first,
            "Repeated cumulative snapshots do not charge twice"
        );
        totals.record(&sample(200, 40, 100), Some(&prices));
        assert!((totals.cost - 0.00061).abs() < 1e-12);
        assert!(totals.priced);
        let event = |kind, id, start, duration| json!({"type":"event_msg","payload":{"type":kind,"turn_id":id,"started_at":start,"duration_ms":duration}});
        totals.record(
            &event("task_started", json!("one"), json!(1000), json!(null)),
            None,
        );
        assert_eq!(totals.elapsed(1_005_000), Some(5000));
        totals.record(
            &event("task_complete", json!("one"), json!(1000), json!(12000)),
            None,
        );
        totals.record(
            &event("task_complete", json!("one"), json!(1000), json!(12000)),
            None,
        );
        totals.record(
            &event("task_started", json!("two"), json!(1020), json!(null)),
            None,
        );
        assert_eq!(totals.elapsed(1_030_000), Some(22000));
        totals.record(
            &event("turn_aborted", json!(null), json!(1020), json!(15000)),
            None,
        );
        assert_eq!(totals.elapsed(5_000_000), Some(27000));
        totals.record(
            &event("task_started", json!("three"), json!(1040), json!(null)),
            None,
        );
        totals.record(
            &json!({"timestamp":"1970-01-01T00:17:25Z","type":"event_msg",
            "payload":{"type":"turn_aborted","turn_id":"three"}}),
            None,
        );
        assert_eq!(
            totals.elapsed(9_000_000),
            Some(32000),
            "An abort timestamp ends the active timer"
        );
    }

    #[test]
    fn qwen_native_ledger_is_session_scoped_incremental_and_prices_only_known_models() {
        let root = std::env::temp_dir().join(format!("vults-qwen-ledger-{}", std::process::id()));
        let usage = root.join(".qwen/usage");
        std::fs::create_dir_all(&usage).unwrap();
        let file = usage.join("token-usage-2026-10.jsonl");
        let line = |id: &str, sid: &str, input| {
            json!({"schemaVersion":1,"id":id,"sessionId":sid,"source":"main","model":"unit","authType":"openai","inputTokens":input,"outputTokens":10,"cachedTokens":20,"thoughtsTokens":2,"apiDurationMs":1250}).to_string()+"\n"
        };
        std::fs::write(
            &file,
            line("a", "mine", 100) + &line("a", "mine", 100) + &line("b", "other", 900),
        )
        .unwrap();
        let mut reader = Reader::default();
        reader.set_prices(Some(Arc::new(
            Catalog::from_models_dev(
                &json!({"vendor":{"models":{"unit":{"cost":{"input":2,"output":10,"cache_read":0.1}}}}}),
                epoch(),
                None,
            )
            .unwrap(),
        )));
        let dirs = crate::Dirs::home_only(&root);
        let keys = [key(AgentKind::Qwen, "mine")];
        let first = reader.read(&dirs, &root, &keys).pop().unwrap().1;
        assert_eq!(first.elapsed_ms, Some(1250));
        assert_eq!(first.context_tokens, Some(100));
        assert_eq!(first.info.input_tokens, Some(100));
        assert_eq!(first.info.output_tokens, Some(12));
        assert_eq!(first.info.price_provider.as_deref(), Some("vendor"));
        assert!((first.estimated_cost_usd.unwrap() - 0.000282).abs() < 1e-12);
        assert_eq!(
            reader.read(&dirs, &root, &keys)[0].1,
            first,
            "Polling must not add a completed response twice"
        );
        use std::io::Write;
        File::options()
            .append(true)
            .open(&file)
            .unwrap()
            .write_all(line("c", "mine", 200).as_bytes())
            .unwrap();
        let next = reader.read(&dirs, &root, &keys).pop().unwrap().1;
        assert_eq!(next.elapsed_ms, Some(2500));
        assert_eq!(next.info.input_tokens, Some(300));
        assert_eq!(next.context_tokens, Some(200));
        let worker=json!({"schemaVersion":1,"id":"worker","sessionId":"mine","source":"reviewer","model":"unit","inputTokens":900,"outputTokens":10,"cachedTokens":0,"thoughtsTokens":0,"apiDurationMs":10}).to_string()+"\n";
        File::options()
            .append(true)
            .open(&file)
            .unwrap()
            .write_all(worker.as_bytes())
            .unwrap();
        let parallel = reader.read(&dirs, &root, &keys).pop().unwrap().1;
        assert_eq!(
            parallel.context_tokens,
            Some(200),
            "A subagent must not replace the main agent context"
        );
        assert_eq!(
            parallel.info.input_tokens,
            Some(1200),
            "Subagent usage still contributes to the native session ledger"
        );
        std::fs::write(&file, line("new", "mine", 50)).unwrap();
        assert_eq!(
            reader.read(&dirs, &root, &keys)[0].1.info.input_tokens,
            Some(50),
            "Rewritten logs reset their counters"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn qwen_usage_uses_the_known_chat_and_does_not_accumulate_repeated_reports() {
        let root = std::env::temp_dir().join(format!("vults-qwen-metrics-{}", std::process::id()));
        let chats = root.join(".qwen/projects/demo/chats");
        std::fs::create_dir_all(&chats).unwrap();
        let report=json!({"type":"assistant","usageMetadata":{"promptTokenCount":50,"cachedContentTokenCount":20,"candidatesTokenCount":9,"thoughtsTokenCount":1,"totalTokenCount":60}}).to_string()+"\n";
        std::fs::write(chats.join("q1.jsonl"), report.repeat(2)).unwrap();
        let mut reader = Reader::default();
        let dirs = crate::Dirs::home_only(&root);
        let rows = reader.read(
            &dirs,
            &root,
            &[
                key(AgentKind::Qwen, "q1"),
                key(AgentKind::Qwen, "missing"),
                key(AgentKind::Qwen, "../q1"),
            ],
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].1.context_tokens, Some(50));
        assert_eq!(rows[0].1.context_percent, None);
        assert_eq!(rows[0].1.cost_usd, None);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn hermes_cost_is_read_only_and_last_context_is_not_a_cumulative_session_total() {
        let root = std::env::temp_dir().join(format!("vults-hermes-cost-{}", std::process::id()));
        let home = root.join(".hermes");
        std::fs::create_dir_all(&home).unwrap();
        let db = Connection::open(home.join("state.db")).unwrap();
        db.execute_batch("CREATE TABLE sessions (id TEXT, actual_cost_usd REAL, estimated_cost_usd REAL, cost_status TEXT);
            INSERT INTO sessions VALUES ('estimated',NULL,0.0042,'estimated'), ('actual',0.25,0.2,'actual'),
            ('unknown',NULL,99,'unknown'),('included',NULL,NULL,'included');").unwrap();
        db.execute_batch("ALTER TABLE sessions ADD COLUMN ended_at REAL; UPDATE sessions SET ended_at=100 WHERE id='actual';").unwrap();
        drop(db);
        let dirs = crate::Dirs::home_only(&root);
        let keys: Vec<_> = ["estimated", "actual", "unknown", "included"]
            .iter()
            .map(|id| key(AgentKind::Other, &format!("hermes/{id}")))
            .collect();
        let reports = Reader::default().read(&dirs, &root, &keys);
        assert_eq!(
            reports.iter().map(|r| r.1.cost_usd).collect::<Vec<_>>(),
            vec![Some(0.0042), Some(0.25), None, Some(0.0)]
        );
        assert!(reports[1].1.session_closed);
        assert!(!reports[0].1.session_closed);
        let current =
            Snapshot::hermes(&json!({"context_tokens":420,"context_limit":1000,"elapsed_ms":1250})).unwrap();
        assert_eq!(current.context_percent, Some(42.0));
        assert_eq!(current.elapsed_ms, Some(1250));
        assert!(Snapshot::hermes(&json!({"context_tokens":-1,"elapsed_ms":true})).is_none());
        assert_eq!(
            Snapshot::hermes(&json!({"context_tokens":0,"context_limit":0}))
                .unwrap()
                .context_percent,
            None
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn opencode_response_time_unions_parallel_intervals_and_ignores_streaming_messages() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE message (session_id TEXT, data TEXT)")
            .unwrap();
        for (session, role, start, end) in [
            ("a", "assistant", 100, Some(200)),
            ("a", "assistant", 150, Some(250)),
            ("a", "assistant", 400, Some(450)),
            ("a", "assistant", 500, None),
            ("a", "user", 0, Some(9999)),
            ("other", "assistant", 0, Some(9999)),
        ] {
            db.execute(
                "INSERT INTO message VALUES (?1, ?2)",
                [
                    session.to_string(),
                    json!({"role":role,"time":{"created":start,"completed":end}}).to_string(),
                ],
            )
            .unwrap();
        }
        assert_eq!(opencode_elapsed(&db, "a"), Some(200));
        assert_eq!(opencode_elapsed(&db, "missing"), None);
        db.execute(
            "INSERT INTO message VALUES ('bad', ?1)",
            [json!({"role":"assistant","time":{"created":20,"completed":10}}).to_string()],
        )
        .unwrap();
        assert_eq!(opencode_elapsed(&db, "bad"), None);
    }

    #[test]
    fn context_is_the_last_sample_not_cumulative_and_unknown_is_not_zero() {
        let sample = |used: Value, limit: Value| {
            json!({"type":"event_msg", "payload": {
            "type":"token_count", "info": {"total_token_usage":{"total_tokens":813755},
                "last_token_usage":{"total_tokens":used},"model_context_window":limit}}})
        };
        let context = codex_snapshot(&sample(json!(90619), json!(475000))).unwrap();
        assert_eq!(context.context_tokens, Some(90619));
        assert!((context.context_percent.unwrap() - 19.077684).abs() < 0.00001);
        assert_eq!(context.cost_usd, None, "Codex does not report a monetary cost");
        assert_eq!(
            codex_snapshot(&sample(json!(0), json!(100)))
                .unwrap()
                .context_percent,
            Some(0.0)
        );
        assert_eq!(
            codex_snapshot(&sample(json!(101), json!(100)))
                .unwrap()
                .context_percent,
            Some(100.0)
        );
        assert_eq!(
            codex_snapshot(&sample(json!(10), json!(0)))
                .unwrap()
                .context_percent,
            None
        );
        assert!(codex_snapshot(&sample(json!(null), json!(100))).is_none());
        assert_eq!(Snapshot::claude(&json!({})).cost_usd, None);
        assert_eq!(Snapshot::claude(&json!({})).context_tokens, None);
        let claude = Snapshot::claude(&json!({"context_window":{"used_percentage":25,
            "current_usage":{"input_tokens":100,"cache_creation_input_tokens":200,"cache_read_input_tokens":300,"output_tokens":999}}}));
        assert_eq!(claude.context_tokens, Some(600));
        assert_eq!(claude.context_percent, Some(25.0));
        assert_eq!(
            opencode_tokens(&json!({"tokens":{"total":42,"input":1,"output":2}})),
            Some(42)
        );
    }

    #[test]
    fn read_only_sources_are_scoped_to_known_sessions_and_follow_new_reports() {
        let root = std::env::temp_dir().join(format!("vults-metrics-{}", std::process::id()));
        let codex_home = root.join("codex");
        let sessions = codex_home.join("sessions/2026/10/28");
        let data = root.join("data");
        std::fs::create_dir_all(&sessions).unwrap();
        std::fs::create_dir_all(data.join("opencode")).unwrap();
        let rollout = sessions.join("rollout-c1.jsonl");
        let sample = |used| {
            json!({"type":"event_msg","payload":{"type":"token_count","info":{
            "total_token_usage":{"total_tokens":99999},"last_token_usage":{"total_tokens":used},"model_context_window":1000}}}).to_string()+"\n"
        };
        let large_output =
            json!({"type":"response_item","payload":{"text":"x".repeat(1024 * 1024)}}).to_string() + "\n";
        std::fs::write(&rollout, sample(420) + &large_output).unwrap();
        let index = codex_home.join("state_9.sqlite");
        let db = Connection::open(&index).unwrap();
        db.execute_batch("CREATE TABLE threads (id TEXT, rollout_path TEXT)")
            .unwrap();
        db.execute(
            "INSERT INTO threads VALUES ('c1', ?1)",
            [rollout.to_str().unwrap()],
        )
        .unwrap();
        let outside = root.join("not-a-transcript.jsonl");
        std::fs::write(&outside, sample(999)).unwrap();
        db.execute(
            "INSERT INTO threads VALUES ('outside', ?1)",
            [outside.to_str().unwrap()],
        )
        .unwrap();
        drop(db);
        assert!(
            database(&index)
                .unwrap()
                .execute("DELETE FROM threads", [])
                .is_err()
        );
        let db = Connection::open(data.join("opencode/opencode.db")).unwrap();
        db.execute_batch("CREATE TABLE session (id TEXT, cost REAL); CREATE TABLE message (id TEXT, session_id TEXT, time_created INTEGER, data TEXT); INSERT INTO session VALUES ('o1', 0.125)").unwrap();
        let msg = json!({"role":"assistant","cost":0.125,"time":{"completed":42},"tokens":{
            "input":166,"output":6,"reasoning":3,"cache":{"read":11776,"write":0}}});
        db.execute(
            "INSERT INTO message VALUES ('m1', 'o1', 1, ?1)",
            [msg.to_string()],
        )
        .unwrap();
        // A streaming message with zero counters must not erase the last completed sample.
        db.execute(
            "INSERT INTO message VALUES ('m2', 'o1', 2, ?1)",
            [json!({"role":"assistant","time":{},"tokens":{"input":0,"output":0}}).to_string()],
        )
        .unwrap();
        drop(db);
        let dirs = crate::Dirs::new(&root, |name| {
            (name == "CODEX_HOME").then(|| codex_home.clone().into_os_string())
        });
        let mut reader = Reader::default();
        let c1 = key(AgentKind::Codex, "c1");
        let o1 = key(AgentKind::OpenCode, "o1");
        assert!(reader.read(&dirs, &data, &[]).is_empty());
        let known = [
            c1.clone(),
            o1.clone(),
            key(AgentKind::Codex, "missing"),
            key(AgentKind::Codex, "outside"),
        ];
        let reports = reader.read(&dirs, &data, &known);
        assert_eq!(reports.len(), 2);
        assert_eq!(reports[0].1.context_percent, Some(42.0));
        assert_eq!(reports[1].1.cost_usd, Some(0.125));
        assert_eq!(reports[1].1.context_tokens, Some(11951));
        assert_eq!(
            reports[1].1.context_percent, None,
            "No guessed model context limit"
        );
        assert_eq!(reader.read(&dirs, &data, &known), reports);
        let db = Connection::open(data.join("opencode/opencode.db")).unwrap();
        db.execute_batch("ALTER TABLE session DROP COLUMN cost").unwrap();
        drop(db);
        assert_eq!(reader.read(&dirs, &data, &known)[1].1.cost_usd, Some(0.125));
        // A partial last line is ignored until the writer finishes it.
        use std::io::Write;
        let newer = sample(123);
        let mut file = std::fs::OpenOptions::new().append(true).open(&rollout).unwrap();
        file.write_all(newer.trim_end().as_bytes()).unwrap();
        assert_eq!(reader.read(&dirs, &data, &known)[0].1.context_percent, Some(42.0));
        file.write_all(b"\n").unwrap();
        assert_eq!(reader.read(&dirs, &data, &known)[0].1.context_percent, Some(12.3));
        assert!(reader.read(&dirs, &data, &[]).is_empty());
        assert!(reader.logs.is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cursor_handles_partial_lines_large_output_replacement_and_truncation() {
        use std::io::Write;
        let root = std::env::temp_dir().join(format!("vults-metrics-cursor-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("rollout.jsonl");
        let sample = |used| {
            json!({"type":"event_msg","payload":{"type":"token_count","info":{
            "last_token_usage":{"total_tokens":used},"model_context_window":1000}}})
            .to_string()
                + "\n"
        };
        let read = |cache: &mut Cached| {
            cache
                .read(&mut File::open(&path).unwrap(), AgentKind::Codex, None)
                .unwrap()
        };
        std::fs::write(&path, sample(420)).unwrap();
        let mut cache = Cached::new(path.clone());
        read(&mut cache);
        assert_eq!(cache.snapshot.as_ref().unwrap().context_tokens, Some(420));
        let mut writer = std::fs::OpenOptions::new().append(true).open(&path).unwrap();
        let next = sample(123);
        writer.write_all(&next.as_bytes()[..40]).unwrap();
        read(&mut cache);
        assert_eq!(cache.pending.len(), 40);
        let offset = cache.offset;
        read(&mut cache);
        assert_eq!(cache.offset, offset, "An unfinished line is not read twice");
        writer.write_all(&next.as_bytes()[40..]).unwrap();
        // More than the old tail limit after a new sample must not conceal the reading.
        writer
            .write_all(b"{\"type\":\"response_item\",\"text\":\"")
            .unwrap();
        writer.write_all(&vec![b'x'; 1024 * 1024]).unwrap();
        read(&mut cache);
        assert!(cache.oversized);
        assert_eq!(cache.snapshot.as_ref().unwrap().context_tokens, Some(123));
        writer.write_all(b"\"}\n").unwrap();
        writer.write_all(sample(0).as_bytes()).unwrap();
        read(&mut cache);
        assert!(!cache.oversized && cache.pending.is_empty());
        assert_eq!(cache.snapshot.as_ref().unwrap().context_percent, Some(0.0));
        drop(writer);
        // A different file can occupy the same path, with the same size and mtime.
        let replacement = root.join("replacement");
        std::fs::write(&replacement, sample(7)).unwrap();
        let original = std::fs::metadata(&path).unwrap();
        let replacement_file = std::fs::OpenOptions::new()
            .write(true)
            .open(&replacement)
            .unwrap();
        replacement_file.set_len(original.len()).unwrap();
        replacement_file
            .set_modified(original.modified().unwrap())
            .unwrap();
        drop(replacement_file);
        std::fs::rename(&replacement, &path).unwrap();
        read(&mut cache);
        assert_eq!(cache.snapshot.as_ref().unwrap().context_tokens, Some(7));
        // A rewrite that grows in place is distinguished from a genuine append by its checkpoint.
        std::fs::write(&path, sample(999) + &" ".repeat(original.len() as usize) + "\n").unwrap();
        read(&mut cache);
        assert_eq!(cache.snapshot.as_ref().unwrap().context_tokens, Some(999));
        std::fs::write(&path, b"").unwrap();
        read(&mut cache);
        assert_eq!(cache.snapshot, None);
        assert_eq!(cache.offset, 0);
        assert!(cache.pending.is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }
}
