//! Local output controls through fixed pactl arguments; transcripts never become process arguments.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Mute(bool),
    Volume(u8),
    Louder,
    Quieter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Unavailable,
    Changed,
    Cancelled,
    Invalid,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Status {
    pub muted: bool,
    pub percent: u32,
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use serde_json::Value;
    use std::{future::Future, path::PathBuf, process::Stdio, time::Duration};
    use tokio::{io::AsyncReadExt, process::Command};

    const NORMAL: u64 = 65_536;
    const LIMIT: u64 = 1_048_576;
    trait Io: Sync {
        fn request(&self, args: &[String]) -> impl Future<Output = Result<Vec<u8>, Error>> + Send;
    }
    #[derive(Debug)]
    struct Backend {
        program: PathBuf,
        server: String,
    }
    impl Io for Backend {
        async fn request(&self, args: &[String]) -> Result<Vec<u8>, Error> {
            let mut child = Command::new(&self.program)
                .args(["--format=json", "--server", &self.server])
                .args(args)
                .env("LC_ALL", "C")
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .kill_on_drop(true)
                .spawn()
                .map_err(|_| Error::Unavailable)?;
            tokio::time::timeout(Duration::from_secs(2), async {
                let mut bytes = Vec::new();
                child
                    .stdout
                    .take()
                    .ok_or(Error::Failed)?
                    .take(LIMIT + 1)
                    .read_to_end(&mut bytes)
                    .await
                    .map_err(|_| Error::Failed)?;
                if bytes.len() as u64 > LIMIT {
                    return Err(Error::Invalid);
                }
                if !child.wait().await.map_err(|_| Error::Failed)?.success() {
                    return Err(Error::Failed);
                }
                Ok(bytes)
            })
            .await
            .map_err(|_| Error::Failed)?
        }
    }

    fn local_server(value: &str) -> Result<String, Error> {
        let path = value.strip_prefix("unix:").unwrap_or(value);
        if !path.starts_with('/')
            || path.len() > 4096
            || !path
                .bytes()
                .all(|ch| ch.is_ascii_alphanumeric() || b"/_-.".contains(&ch))
        {
            return Err(Error::Unavailable);
        }
        Ok(format!("unix:{path}"))
    }
    fn backend() -> Result<Backend, Error> {
        use std::os::unix::fs::PermissionsExt;
        let program = std::env::var_os("PATH")
            .into_iter()
            .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
            .map(|dir| dir.join("pactl"))
            .find(|path| {
                path.metadata()
                    .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
            })
            .ok_or(Error::Unavailable)?;
        let server = match std::env::var("PULSE_SERVER") {
            Ok(server) => local_server(&server)?,
            Err(std::env::VarError::NotPresent) => {
                let runtime = std::env::var_os("XDG_RUNTIME_DIR").ok_or(Error::Unavailable)?;
                let socket = PathBuf::from(runtime).join("pulse/native");
                local_server(&format!("unix:{}", socket.to_str().ok_or(Error::Unavailable)?))?
            }
            Err(_) => return Err(Error::Unavailable),
        };
        Ok(Backend { program, server })
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Identity {
        cookie: String,
        server: String,
        name: String,
        index: u32,
        channels: Vec<String>,
        serial: Option<String>,
        port: Option<String>,
    }
    #[derive(Debug)]
    struct Snapshot {
        identity: Identity,
        muted: bool,
        volume: Vec<u32>,
    }
    #[derive(PartialEq, Eq)]
    struct Server {
        cookie: String,
        server: String,
        default: String,
    }
    fn server(bytes: &[u8]) -> Result<Server, Error> {
        let value: Value = serde_json::from_slice(bytes).map_err(|_| Error::Invalid)?;
        if value["is_local"].as_str() != Some("yes") {
            return Err(Error::Unavailable);
        }
        let cookie = value["cookie"].as_str().ok_or(Error::Invalid)?;
        if cookie.len() != 9
            || cookie.as_bytes()[4] != b':'
            || !cookie
                .bytes()
                .enumerate()
                .all(|(i, ch)| i == 4 || ch.is_ascii_hexdigit())
        {
            return Err(Error::Invalid);
        }
        Ok(Server {
            cookie: cookie.into(),
            server: local_server(value["server_string"].as_str().ok_or(Error::Invalid)?)?,
            default: value["default_sink_name"]
                .as_str()
                .filter(|name| !name.is_empty())
                .ok_or(Error::Unavailable)?
                .into(),
        })
    }
    fn optional_string(value: &Value) -> Result<Option<String>, Error> {
        match value {
            Value::Null => Ok(None),
            Value::String(value) => Ok(Some(value.clone())),
            _ => Err(Error::Invalid),
        }
    }
    fn sink(bytes: &[u8], server: Server) -> Result<Snapshot, Error> {
        let value: Value = serde_json::from_slice(bytes).map_err(|_| Error::Invalid)?;
        let sinks = value.as_array().ok_or(Error::Invalid)?;
        let mut selected = sinks
            .iter()
            .filter(|sink| sink["name"].as_str() == Some(&server.default));
        let sink = selected.next().ok_or(Error::Unavailable)?;
        if selected.next().is_some() {
            return Err(Error::Invalid);
        }
        let index = sink["index"]
            .as_u64()
            .and_then(|index| u32::try_from(index).ok())
            .filter(|index| *index != u32::MAX)
            .ok_or(Error::Invalid)?;
        let channels: Vec<String> = sink["channel_map"]
            .as_str()
            .ok_or(Error::Invalid)?
            .split(',')
            .map(|name| name.trim().to_owned())
            .collect();
        if channels.is_empty() || channels.len() > 32 || channels.iter().any(String::is_empty) {
            return Err(Error::Invalid);
        }
        let unique: std::collections::BTreeSet<_> = channels.iter().collect();
        if unique.len() != channels.len()
            || sink["volume"]
                .as_object()
                .is_none_or(|values| values.len() != channels.len())
        {
            return Err(Error::Invalid);
        }
        let volume: Vec<u32> = channels
            .iter()
            .map(|channel| {
                sink["volume"][channel]["value"]
                    .as_u64()
                    .and_then(|v| u32::try_from(v).ok())
                    .filter(|v| *v <= i32::MAX as u32)
                    .ok_or(Error::Invalid)
            })
            .collect::<Result<_, _>>()?;
        Ok(Snapshot {
            identity: Identity {
                cookie: server.cookie,
                server: server.server,
                name: server.default,
                index,
                channels,
                serial: optional_string(&sink["properties"]["object.serial"])?,
                port: optional_string(&sink["active_port"])?,
            },
            muted: sink["mute"].as_bool().ok_or(Error::Invalid)?,
            volume,
        })
    }
    async fn snapshot(io: &impl Io, allowed: &(impl Fn() -> bool + Sync)) -> Result<Snapshot, Error> {
        if !allowed() {
            return Err(Error::Cancelled);
        }
        let first = server(&io.request(&["info".into()]).await?)?;
        if !allowed() {
            return Err(Error::Cancelled);
        }
        let sinks = io.request(&["list".into(), "sinks".into()]).await?;
        if !allowed() {
            return Err(Error::Cancelled);
        }
        let second = server(&io.request(&["info".into()]).await?)?;
        if first != second {
            return Err(Error::Changed);
        }
        sink(&sinks, first)
    }
    fn percent(volume: &[u32]) -> u32 {
        let max = u64::from(*volume.iter().max().unwrap_or(&0));
        ((max * 100 + NORMAL / 2) / NORMAL) as u32
    }
    fn volumes(current: &[u32], level: u8) -> Vec<u32> {
        let max = u64::from(*current.iter().max().unwrap_or(&0));
        let target = u64::from(level) * NORMAL / 100;
        current
            .iter()
            .map(|value| {
                ((u64::from(*value) * target + max / 2)
                    .checked_div(max)
                    .unwrap_or(target))
                .min(NORMAL) as u32
            })
            .collect()
    }
    async fn apply(
        io: &impl Io,
        pinned: &Identity,
        action: Action,
        allowed: &(impl Fn() -> bool + Sync),
    ) -> Result<Status, Error> {
        if matches!(action, Action::Volume(level) if level > 100) {
            return Err(Error::Invalid);
        }
        let before = snapshot(io, allowed).await?;
        if &before.identity != pinned {
            return Err(Error::Changed);
        }
        let mut expected = None;
        let args = match action {
            Action::Mute(muted) => vec![
                "set-sink-mute".into(),
                pinned.index.to_string(),
                if muted { "1".into() } else { "0".into() },
            ],
            _ => {
                let level = match action {
                    Action::Volume(level) => level,
                    Action::Louder => percent(&before.volume).saturating_add(10).min(100) as u8,
                    Action::Quieter => percent(&before.volume).saturating_sub(10).min(100) as u8,
                    _ => unreachable!(),
                };
                let volume = volumes(&before.volume, level);
                let mut args = vec!["set-sink-volume".into(), pinned.index.to_string()];
                args.extend(volume.iter().map(u32::to_string));
                expected = Some(volume);
                args
            }
        };
        if !allowed() {
            return Err(Error::Cancelled);
        }
        io.request(&args).await?;
        let after = snapshot(io, allowed).await?;
        if &after.identity != pinned {
            return Err(Error::Changed);
        }
        if let Action::Mute(muted) = action
            && after.muted != muted
        {
            return Err(Error::Failed);
        }
        if let Some(expected) = expected
            && (after.muted != before.muted
                || after.volume.iter().any(|value| u64::from(*value) > NORMAL)
                || expected
                    .iter()
                    .zip(&after.volume)
                    .any(|(expected, actual)| expected.abs_diff(*actual) > 1))
        {
            return Err(Error::Failed);
        }
        Ok(Status {
            muted: after.muted,
            percent: percent(&after.volume),
        })
    }

    pub struct Target {
        backend: Backend,
        identity: Identity,
    }
    impl std::fmt::Debug for Target {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("OutputTarget")
                .field("index", &self.identity.index)
                .finish_non_exhaustive()
        }
    }
    pub async fn pin(allowed: impl Fn() -> bool + Sync) -> Result<Target, Error> {
        if !allowed() {
            return Err(Error::Cancelled);
        }
        let backend = backend()?;
        let snapshot = snapshot(&backend, &allowed).await?;
        Ok(Target {
            backend,
            identity: snapshot.identity,
        })
    }
    impl Target {
        pub async fn apply(
            &self,
            action: Action,
            allowed: impl Fn() -> bool + Sync,
        ) -> Result<Status, Error> {
            apply(&self.backend, &self.identity, action, &allowed).await
        }
    }

    #[cfg(test)]
    mod tests;
}

#[cfg(target_os = "linux")]
pub use linux::{Target, pin};
