//! Read-only PipeWire Pulse inventory. Discovery is not proof of a routed or effective AEC path.
use crate::meeting::pulse::Connection;
use pulseaudio::protocol::{ModuleInfo, Props, ServerInfo, SinkInfo, SourceInfo};
use std::ffi::CStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Unavailable,
    Unsupported,
    Inventory,
    Changed,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Pair {
    server: u32,
    module: u32,
    source: Endpoint,
    sink: Endpoint,
    group: String,
}
#[derive(Clone, PartialEq, Eq)]
struct Endpoint {
    index: u32,
    serial: u64,
    name: String,
}
impl std::fmt::Debug for Pair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EchoPair")
            .field("module", &self.module)
            .field("source", &self.source.index)
            .field("source_serial", &self.source.serial)
            .field("sink", &self.sink.index)
            .field("sink_serial", &self.sink.serial)
            .finish()
    }
}
impl Pair {
    pub fn source(&self) -> &str {
        &self.source.name
    }
    pub fn sink(&self) -> &str {
        &self.sink.name
    }
}

/// Lists candidates without opening streams, loading modules, selecting defaults or changing routes.
/// Native-only echo modules without Pulse ownership/explicit WebRTC arguments are not qualified here.
pub fn discover() -> Result<Vec<Pair>, Failure> {
    let connection = Connection::inspect().map_err(|_| Failure::Unavailable)?;
    let server = connection
        .wait(connection.client.server_info())
        .map_err(|_| Failure::Unavailable)?;
    let modules = connection
        .wait(connection.client.list_modules())
        .map_err(|_| Failure::Unavailable)?;
    let sources = connection
        .wait(connection.client.list_sources())
        .map_err(|_| Failure::Unavailable)?;
    let sinks = connection
        .wait(connection.client.list_sinks())
        .map_err(|_| Failure::Unavailable)?;
    let after = connection
        .wait(connection.client.server_info())
        .map_err(|_| Failure::Unavailable)?;
    if server.cookie != after.cookie {
        return Err(Failure::Changed);
    }
    pairs(&server, &modules, &sources, &sinks)
}

/// Rechecking a snapshot still grants no stream/route lease; callers must pin both actual streams.
pub fn present(pair: &Pair) -> Result<bool, Failure> {
    Ok(discover()?.contains(pair))
}

fn text(value: &CStr) -> Option<&str> {
    value.to_str().ok().filter(|s| !s.is_empty() && s.len() <= 1024)
}
fn property<'a>(props: &'a Props, key: &CStr) -> Option<&'a str> {
    text(CStr::from_bytes_with_nul(props.get_bytes(key)?).ok()?)
}
fn endpoint(index: u32, name: &CStr, props: &Props) -> Option<Endpoint> {
    let serial = property(props, c"object.serial")?
        .parse::<u64>()
        .ok()
        .filter(|n| *n > 0)?;
    Some(Endpoint {
        index,
        serial,
        name: text(name)?.into(),
    })
}
fn webrtc(module: &ModuleInfo) -> bool {
    if module.name.as_c_str() != c"module-echo-cancel" {
        return false;
    }
    let Some(args) = module.argument.as_ref().and_then(|s| text(s)) else {
        return false;
    };
    let mut rest = args;
    let mut method = None;
    while !rest.trim_ascii_start().is_empty() {
        rest = rest.trim_ascii_start();
        let Some((key, tail)) = rest.split_once('=') else {
            return false;
        };
        if key.is_empty()
            || !key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
        {
            return false;
        }
        // Accept a strict subset of PipeWire's module parser, not shell-style quote concatenation.
        let (value, tail) = if let Some(quote) = tail.chars().next().filter(|c| matches!(c, '\'' | '"')) {
            let Some(end) = tail[1..].find(quote) else {
                return false;
            };
            let value = &tail[1..end + 1];
            let tail = &tail[end + 2..];
            if !tail.is_empty() && !tail.starts_with(|c: char| c.is_ascii_whitespace()) {
                return false;
            }
            (value, tail)
        } else {
            let end = tail.find(|c: char| c.is_ascii_whitespace()).unwrap_or(tail.len());
            let value = &tail[..end];
            if value.contains(['\'', '"']) {
                return false;
            }
            (value, &tail[end..])
        };
        if value.contains('\\') {
            return false;
        }
        if key == "aec_method" {
            if method.is_some() {
                return false;
            }
            method = Some(value);
        }
        rest = tail;
    }
    method == Some("webrtc")
}
fn pairs(
    server: &ServerInfo,
    modules: &[ModuleInfo],
    sources: &[SourceInfo],
    sinks: &[SinkInfo],
) -> Result<Vec<Pair>, Failure> {
    if !server
        .server_name
        .as_ref()
        .and_then(|s| text(s))
        .is_some_and(|name| name.starts_with("PulseAudio (on PipeWire ") && name.ends_with(')'))
    {
        return Err(Failure::Unsupported);
    }
    if modules.len() > 1024 || sources.len() > 1024 || sinks.len() > 1024 {
        return Err(Failure::Inventory);
    }
    let mut pairs = Vec::new();
    for module in modules.iter().filter(|m| webrtc(m)) {
        let inputs: Vec<_> = sources
            .iter()
            .filter(|source| {
                source.owner_module_index == Some(module.index)
                    && source.monitor_of_sink_index.is_none()
                    && source.monitor_of_sink_name.is_none()
            })
            .collect();
        let outputs: Vec<_> = sinks
            .iter()
            .filter(|sink| sink.owner_module_index == Some(module.index))
            .collect();
        let ([source], [sink]) = (inputs.as_slice(), outputs.as_slice()) else {
            continue;
        };
        let (Some(source_id), Some(sink_id)) = (
            endpoint(source.index, &source.name, &source.props),
            endpoint(sink.index, &sink.name, &sink.props),
        ) else {
            continue;
        };
        let Some(group) = property(&source.props, c"node.group") else {
            continue;
        };
        if property(&sink.props, c"node.group") != Some(group)
            || source.name == sink.name
            || ![source.sample_spec, sink.sample_spec]
                .iter()
                .all(|s| (1..=2).contains(&s.channels) && (16_000..=96_000).contains(&s.sample_rate))
        {
            continue;
        }
        pairs.push(Pair {
            server: server.cookie,
            module: module.index,
            source: source_id,
            sink: sink_id,
            group: group.into(),
        });
    }
    Ok(pairs)
}

#[cfg(test)]
mod tests;
