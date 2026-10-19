//! The Settings-only diff/backup/click path for the question-only MCP entry.
const MANAGED: &str = "VULTS_QUESTIONS_MANAGED";
use crate::mcp::{QUESTIONS_ARG as ARG, QUESTIONS_NAME as NAME};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Mutex,
    time::SystemTime,
};
use tauri::{AppHandle, Manager};
use vults_agent_config as config;

#[derive(Default)]
pub(crate) struct Plans(Mutex<BTreeMap<String, Plan>>);
struct Plan {
    path: PathBuf,
    program: PathBuf,
    install: bool,
    preview: config::Preview,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Status {
    agent: String,
    path: String,
    installed: bool,
    error: Option<String>,
}
#[derive(Serialize)]
pub(crate) struct Preview {
    diff: String,
    fingerprint: String,
}
fn target(agent: &str) -> Result<PathBuf, String> {
    let dirs = crate::paths::agent_dirs();
    match agent {
        "claude" => Ok(dirs
            .var("CLAUDE_CONFIG_DIR")
            .unwrap_or(dirs.home())
            .join(".claude.json")),
        "codex" => Ok(vults_agents::codex_home(&dirs).join("config.toml")),
        _ => Err("Question setup supports Claude Code and Codex on Linux.".into()),
    }
}
fn ours(entry: &Value, program: &Path) -> bool {
    (entry["command"].as_str() == program.to_str() || entry["env"][MANAGED] == "1")
        && entry["args"] == json!([ARG])
}
fn json_change(current: &Value, program: &Path, install: bool) -> Result<Value, String> {
    let mut next = current.clone();
    if let Some(servers) = next.get("mcpServers")
        && !servers.is_object()
    {
        return Err("The mcpServers entry is not an object. Nothing was changed.".into());
    }
    if let Some(entry) = next.get("mcpServers").and_then(|s| s.get(NAME))
        && !ours(entry, program)
    {
        return Err("A different server already uses vults-questions. Nothing was changed.".into());
    }
    if install {
        if next.get("mcpServers").is_none() {
            next["mcpServers"] = json!({});
        }
        next["mcpServers"][NAME] = json!({"type":"stdio","command":program,"args":[ARG],"env":{MANAGED:"1"}});
    } else if let Some(servers) = next.get_mut("mcpServers").and_then(Value::as_object_mut)
        && servers.shift_remove(NAME).is_some()
        && servers.is_empty()
    {
        next.as_object_mut()
            .expect("parsed object")
            .shift_remove("mcpServers");
    }
    Ok(next)
}
fn toml_change(
    doc: &mut config::toml::toml_edit::DocumentMut,
    program: &Path,
    install: bool,
) -> Result<(), String> {
    use config::toml::toml_edit::{Array, Item, Table, value};
    if let Some(servers) = doc.get("mcp_servers")
        && !servers.is_table()
    {
        return Err("The mcp_servers entry is not a table. Nothing was changed.".into());
    }
    if let Some(entry) = doc.get("mcp_servers").and_then(|s| s.get(NAME)) {
        let ours = (entry.get("command").and_then(Item::as_str) == program.to_str()
            || entry
                .get("env")
                .and_then(|env| env.get(MANAGED))
                .and_then(Item::as_str)
                == Some("1"))
            && entry
                .get("args")
                .and_then(Item::as_array)
                .is_some_and(|args| args.len() == 1 && args.get(0).and_then(|a| a.as_str()) == Some(ARG));
        if !ours {
            return Err("A different server already uses vults-questions. Nothing was changed.".into());
        }
    }
    if install {
        if doc.get("mcp_servers").is_none() {
            doc["mcp_servers"] = Item::Table(Table::new());
        }
        let mut entry = Table::new();
        entry["command"] = value(program.to_str().ok_or("The app path is not UTF-8.")?);
        let mut args = Array::new();
        args.push(ARG);
        entry["args"] = value(args);
        entry["tool_timeout_sec"] = value(150);
        let mut env = Table::new();
        env[MANAGED] = value("1");
        entry["env"] = Item::Table(env);
        doc["mcp_servers"][NAME] = Item::Table(entry);
    } else if let Some(servers) = doc.get_mut("mcp_servers").and_then(Item::as_table_mut)
        && servers.remove(NAME).is_some()
        && servers.is_empty()
    {
        doc.remove("mcp_servers");
    }
    Ok(())
}
fn preview_at(agent: &str, path: &Path, program: &Path, install: bool) -> Result<config::Preview, String> {
    program.to_str().ok_or("The app path is not UTF-8.")?;
    let mut failure = None;
    let preview = if agent == "codex" {
        config::toml::preview(path, |doc| {
            failure = toml_change(doc, program, install).err();
        })
    } else {
        config::preview(path, |current| match json_change(current, program, install) {
            Ok(next) => next,
            Err(error) => {
                failure = Some(error);
                current.clone()
            }
        })
    }
    .map_err(|e| e.to_string())?;
    if let Some(error) = failure {
        return Err(error);
    }
    Ok(preview)
}
fn apply_at(agent: &str, plan: Plan) -> Result<Option<PathBuf>, String> {
    // Revalidate the exact plan before using the library's fingerprint/dated-backup/atomic write.
    let current = preview_at(agent, &plan.path, &plan.program, plan.install)?;
    if current != plan.preview {
        return Err("The configuration changed. Review a new diff.".into());
    }
    if agent == "codex" {
        config::toml::apply(
            &plan.path,
            &plan.preview.fingerprint,
            |doc| {
                // The same bytes passed validation; no other part of the document is edited.
                let _ = toml_change(doc, &plan.program, plan.install);
            },
            SystemTime::now(),
        )
    } else {
        config::apply(
            &plan.path,
            &plan.preview.fingerprint,
            |current| json_change(current, &plan.program, plan.install).unwrap_or_else(|_| current.clone()),
            SystemTime::now(),
        )
    }
    .map_err(|e| e.to_string())
}
pub(crate) fn clear(app: &AppHandle) {
    if let Some(state) = app.try_state::<Plans>()
        && let Ok(mut plans) = state.0.lock()
    {
        plans.clear();
    }
}
#[tauri::command]
pub(crate) fn question_install_status(window: tauri::Webview) -> Result<Vec<Status>, String> {
    crate::settings_page(&window)?;
    let program = crate::mcp::program();
    Ok(["claude", "codex"]
        .into_iter()
        .map(|agent| {
            let path = target(agent).expect("supported question agent");
            let (installed, error) = match preview_at(agent, &path, &program, false) {
                Ok(p) => (!p.is_noop(), None),
                Err(error) => (false, Some(error)),
            };
            Status {
                agent: agent.into(),
                path: path.display().to_string(),
                installed,
                error,
            }
        })
        .collect())
}
#[tauri::command]
pub(crate) fn question_install_preview(
    app: AppHandle,
    window: tauri::Webview,
    agent: String,
    install: bool,
) -> Result<Preview, String> {
    crate::settings_page(&window)?;
    let state = app.state::<Plans>();
    let mut plans = state.0.lock().map_err(|_| "Question setup is busy.")?;
    plans.remove(&agent);
    if install && !super::external_enabled(&app) {
        return Err("Enable agent questions before installing.".into());
    }
    let path = target(&agent)?;
    let program = crate::mcp::program();
    let preview = preview_at(&agent, &path, &program, install)?;
    let view = Preview {
        diff: preview.diff.clone(),
        fingerprint: preview.fingerprint.clone(),
    };
    plans.insert(
        agent,
        Plan {
            path,
            program,
            install,
            preview,
        },
    );
    Ok(view)
}
#[tauri::command]
pub(crate) fn question_install_apply(
    app: AppHandle,
    window: tauri::Webview,
    agent: String,
    install: bool,
    fingerprint: String,
) -> Result<Option<String>, String> {
    crate::settings_page(&window)?;
    let plan = app
        .state::<Plans>()
        .0
        .lock()
        .map_err(|_| "Question setup is busy.")?
        .remove(&agent)
        .ok_or("Review the configuration diff before applying.")?;
    if plan.install != install || plan.preview.fingerprint != fingerprint {
        return Err("Review the configuration diff again before applying.".into());
    }
    if install && !super::external_enabled(&app) {
        return Err("Agent questions are off. Nothing was changed.".into());
    }
    apply_at(&agent, plan).map(|backup| backup.map(|p| p.display().to_string()))
}

#[cfg(test)]
mod tests;
