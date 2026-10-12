//! Codex runs a hook only once the user trusted it (`/hooks` in its TUI). For a scratch home
//! only, this asks Codex's own app-server for the hooks it sees and trusts each with the hash it
//! reports. Vults itself never writes `trusted_hash` (CLAUDE.md, rule 3).

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::{Value, json};

pub fn trust(codex: &str, home: &Path, cwd: &Path) -> Result<(), String> {
    let mut child = Command::new(codex)
        .arg("app-server")
        .env("HOME", home)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("{codex}: {e}"))?;
    let result = (|| {
        let mut stdin = child.stdin.take().ok_or("no stdin")?;
        let mut lines = BufReader::new(child.stdout.take().ok_or("no stdout")?).lines();
        let mut call = |id: u64, method: &str, params: Value| -> Result<Value, String> {
            writeln!(
                stdin,
                "{}",
                json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
            )
            .map_err(|e| e.to_string())?;
            for line in lines.by_ref() {
                let line = line.map_err(|e| e.to_string())?;
                // Codex may print other lines first; only its JSON answer counts.
                if let Ok(m) = serde_json::from_str::<Value>(&line)
                    && m["id"] == json!(id)
                {
                    return Ok(m);
                }
            }
            Err("the app-server closed".into())
        };
        call(
            1,
            "initialize",
            json!({ "clientInfo": { "name": "vults-spike", "version": "0" } }),
        )?;
        let listed = call(2, "hooks/list", json!({ "cwds": [cwd] }))?;
        let hooks: Vec<&Value> = listed
            .pointer("/result/data")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|e| e["hooks"].as_array())
            .flatten()
            .collect();
        let mut toml = String::from("\n[features]\nhooks = true\n\n[hooks.state]\n");
        for h in &hooks {
            toml += &format!(
                "\n[hooks.state.{}]\ntrusted_hash = {}\n",
                h["key"], h["currentHash"]
            );
        }
        let config = home.join(".codex").join("config.toml");
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&config)
            .map_err(|e| e.to_string())?;
        f.write_all(toml.as_bytes()).map_err(|e| e.to_string())?;
        println!("{} hooks trusted in {}", hooks.len(), config.display());
        Ok(())
    })();
    let _ = child.kill();
    let _ = child.wait();
    result
}
