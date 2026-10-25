//! A specialist has its own CLI home; subscription credentials are supplied only in memory.
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use vults_agents::driver::Spec;

const LOGIN: &str = "Sign in to Codex with ChatGPT in your terminal, then try again.";

pub(crate) fn auth_path() -> Result<PathBuf, String> {
    let vars = vults_agents::shell_env();
    let value = |name: &str| {
        vars.iter()
            .find(|(key, _)| key == name)
            .map(|(_, v)| v.clone())
            .or_else(|| std::env::var_os(name))
            .filter(|value| !value.is_empty())
    };
    let home = value("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| value("HOME").map(|p| PathBuf::from(p).join(".codex")))
        .ok_or(LOGIN)?;
    if !home.is_absolute() {
        return Err(LOGIN.into());
    }
    Ok(home.join("auth.json"))
}

pub(crate) fn credentials(path: &Path) -> Result<Value, String> {
    use std::io::Read;
    let mut bytes = Vec::new();
    match std::fs::File::open(path) {
        Ok(file) => {
            file.take(128 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| LOGIN)?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            use sha2::{Digest, Sha256};
            let home = path.parent().ok_or(LOGIN)?;
            let canonical = home.canonicalize().unwrap_or_else(|_| home.to_path_buf());
            let hash = format!("{:x}", Sha256::digest(canonical.to_string_lossy().as_bytes()));
            let account = format!("cli|{}", &hash[..16]);
            bytes = vults_secrets::codex_login(&account)
                .map_err(|_| LOGIN)?
                .ok_or(LOGIN)?
                .into_bytes();
        }
        Err(_) => return Err(LOGIN.into()),
    }
    if bytes.len() > 128 * 1024 {
        return Err(LOGIN.into());
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| LOGIN)?;
    let token = value["tokens"]["access_token"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or(LOGIN)?;
    let account = value["tokens"]["account_id"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or(LOGIN)?;
    // Never transfer the refresh token or an API key, and never persist a duplicate login.
    Ok(json!({"type":"chatgptAuthTokens","accessToken":token,"chatgptAccountId":account}))
}

pub(crate) fn identity(auth: &Value) -> Result<String, String> {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use sha2::{Digest, Sha256};
    let account = auth["chatgptAccountId"]
        .as_str()
        .filter(|v| !v.is_empty())
        .ok_or(LOGIN)?;
    let payload = auth["accessToken"]
        .as_str()
        .and_then(|v| v.split('.').nth(1))
        .ok_or(LOGIN)?;
    let claims: Value =
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload).map_err(|_| LOGIN)?).map_err(|_| LOGIN)?;
    let auth = &claims["https://api.openai.com/auth"];
    let subject = auth["chatgpt_user_id"]
        .as_str()
        .or_else(|| auth["user_id"].as_str())
        .or_else(|| claims["sub"].as_str())
        .filter(|v| !v.is_empty())
        .ok_or(LOGIN)?;
    Ok(format!(
        "{:x}",
        Sha256::digest(format!("{account}\0{subject}").as_bytes())
    ))
}
pub(crate) fn runtime(spec: &Spec, auth: &Value) -> Result<Spec, String> {
    let root = spec
        .state_dir
        .as_ref()
        .filter(|p| p.is_absolute())
        .ok_or("The Codex specialist runtime is unavailable.")?;
    let mut isolated = spec.clone();
    isolated.state_dir = Some(root.join("accounts").join(identity(auth)?));
    Ok(isolated)
}

pub(crate) fn session_owner(spec: &Spec, session: &str, create: bool) -> Result<(), String> {
    use std::io::Write;
    const LOST: &str = "This Codex conversation belongs to another login or its local runtime is missing. Start a new conversation.";
    if session.is_empty()
        || session.len() > 100
        || !session
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return Err(LOST.into());
    }
    let root = spec.state_dir.as_ref().ok_or(LOST)?.join("threads");
    let path = root.join(session);
    if create {
        std::fs::create_dir_all(&root).map_err(|_| LOST)?;
        let mut options = std::fs::OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&path) {
            Ok(mut file) => {
                file.write_all(b"1")
                    .and_then(|_| file.sync_all())
                    .map_err(|_| LOST)?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(_) => return Err(LOST.into()),
        }
    }
    if !std::fs::symlink_metadata(&path)
        .is_ok_and(|metadata| metadata.file_type().is_file() && metadata.len() == 1)
        || std::fs::read(&path).map_err(|_| LOST)? != b"1"
    {
        return Err(LOST.into());
    }
    Ok(())
}

pub(crate) fn qualify(config: &Value, has_service: bool) -> Result<(), String> {
    let config = &config["config"];
    let servers = config["mcp_servers"].as_object();
    if servers.is_some_and(|servers| {
        servers
            .keys()
            .any(|name| name != "vults-specialist" || !has_service)
    }) {
        return Err("The managed Codex runtime contains external configuration.".into());
    }
    if config["project_doc_max_bytes"] != 0 || config["cli_auth_credentials_store"] != "ephemeral" {
        return Err("The managed Codex runtime contains external configuration.".into());
    }
    Ok(())
}

pub(crate) fn prepare(spec: &Spec, command: &mut Command) -> Result<(), String> {
    let root = spec
        .state_dir
        .as_ref()
        .filter(|p| p.is_absolute())
        .ok_or("The Codex specialist runtime is unavailable.")?;
    for name in ["config.toml", "hooks.json", "rules"] {
        if std::fs::symlink_metadata(root.join(name)).is_ok() {
            return Err("The managed Codex runtime contains external configuration.".into());
        }
    }
    let home = root.join("home");
    for dir in [root, &home] {
        std::fs::create_dir_all(dir).map_err(|_| "Could not prepare the Codex specialist runtime.")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
                .map_err(|_| "Could not protect the Codex specialist runtime.")?;
        }
    }
    command.env_clear();
    if let Some(path) = vults_agents::shell_env()
        .into_iter()
        .find(|(k, _)| k == "PATH")
        .map(|(_, v)| v)
        .or_else(|| std::env::var_os("PATH"))
    {
        command.env("PATH", path);
    }
    command
        .env("HOME", &home)
        .env("CODEX_HOME", root)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("XDG_DATA_HOME", home.join(".local/share"));
    command.arg("--strict-config");
    for option in [
        "cli_auth_credentials_store=\"ephemeral\"",
        "analytics.enabled=false",
        "project_doc_max_bytes=0",
        "skills.include_instructions=false",
        "features.hooks=false",
        "web_search=\"disabled\"",
    ] {
        command.arg("-c").arg(option);
    }
    if let Some(path) = &spec.mcp_config {
        let bytes = std::fs::read(path).map_err(|_| "Could not read the specialist tool configuration.")?;
        if bytes.len() > 16384 {
            return Err("The specialist tool configuration is invalid.".into());
        }
        let config: Value =
            serde_json::from_slice(&bytes).map_err(|_| "The specialist tool configuration is invalid.")?;
        let server = &config["mcpServers"]["vults-specialist"];
        let executable = server["command"]
            .as_str()
            .ok_or("The specialist tool executable is unavailable.")?;
        let args = server["args"]
            .as_array()
            .filter(|a| a.len() == 2 && a[0] == "--specialist-mcp" && a[1].is_string())
            .ok_or("The specialist tool arguments are invalid.")?;
        command.arg("-c").arg(format!(
            "mcp_servers.vults-specialist.command={}",
            json!(executable)
        ));
        command
            .arg("-c")
            .arg(format!("mcp_servers.vults-specialist.args={}", json!(args)));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn subscription_auth_is_memory_only_and_does_not_transfer_other_secrets() {
        let dir = std::env::temp_dir().join(format!("vults-codex-auth-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("source.json");
        std::fs::write(&file,r#"{"OPENAI_API_KEY":"DO_NOT_TRANSFER","tokens":{"access_token":"ACCESS","refresh_token":"DO_NOT_TRANSFER","account_id":"ACCOUNT"}}"#).unwrap();
        assert_eq!(
            credentials(&file).unwrap(),
            json!({"type":"chatgptAuthTokens","accessToken":"ACCESS","chatgptAccountId":"ACCOUNT"})
        );
        let valid = json!({"config":{"project_doc_max_bytes":0,"cli_auth_credentials_store":"ephemeral","mcp_servers":{}}});
        assert!(qualify(&valid, false).is_ok());
        let mut external = valid.clone();
        external["config"]["mcp_servers"]["outside"] = json!({"command":"false"});
        assert!(qualify(&external, true).is_err());
        let mut spec = Spec::in_dir(dir.clone());
        spec.state_dir = Some(dir.join("managed"));
        let mut cmd = Command::new("codex");
        prepare(&spec, &mut cmd).unwrap();
        assert!(
            cmd.as_std()
                .get_envs()
                .any(|(k, v)| k == "CODEX_HOME" && v == Some(spec.state_dir.as_ref().unwrap().as_os_str()))
        );
        assert!(!spec.state_dir.as_ref().unwrap().join("auth.json").exists());
        std::fs::write(spec.state_dir.as_ref().unwrap().join("config.toml"), "unwanted").unwrap();
        assert!(prepare(&spec, &mut Command::new("codex")).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn sessions_are_bound_to_the_cli_principal_and_workspace_not_rotating_tokens() {
        use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
        let auth = |user: &str, account: &str, signature: &str| json!({"chatgptAccountId":account,"accessToken":format!("e30.{}.{signature}",URL_SAFE_NO_PAD.encode(json!({"https://api.openai.com/auth":{"chatgpt_user_id":user}}).to_string()))});
        let first = auth("user-a", "workspace", "one");
        let renewed = auth("user-a", "workspace", "two");
        let switched = auth("user-b", "workspace", "three");
        assert_eq!(identity(&first).unwrap(), identity(&renewed).unwrap());
        assert_ne!(identity(&first).unwrap(), identity(&switched).unwrap());
        assert_ne!(
            identity(&first).unwrap(),
            identity(&auth("user-a", "another-workspace", "one")).unwrap()
        );
        let root = std::env::temp_dir().join(format!("vults-codex-owner-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut spec = Spec::in_dir(root.clone());
        spec.state_dir = Some(root.clone());
        let original = runtime(&spec, &first).unwrap();
        let other = runtime(&spec, &switched).unwrap();
        assert_ne!(original.state_dir, other.state_dir);
        session_owner(&original, "owned-thread", true).unwrap();
        session_owner(&original, "owned-thread", false).unwrap();
        assert!(session_owner(&other, "owned-thread", false).is_err());
        assert!(session_owner(&original, "../foreign", true).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    #[ignore = "Requires the installed Codex CLI; never starts a model."]
    async fn installed_cli_ignores_external_and_untrusted_project_configuration() {
        use std::process::Stdio;
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        let root = std::env::temp_dir().join(format!("vults-codex-native-isolation-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let project = root.join("project");
        std::fs::create_dir_all(project.join(".codex")).unwrap();
        std::fs::write(
            project.join(".codex/config.toml"),
            "developer_instructions=\"PROJECT_CANARY\"\n[mcp_servers.project_canary]\ncommand=\"false\"\n",
        )
        .unwrap();
        std::fs::write(project.join("AGENTS.md"), "AGENTS_CANARY").unwrap();
        let outside = root.join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(
            outside.join("config.toml"),
            "developer_instructions=\"USER_CANARY\"\n",
        )
        .unwrap();
        let mut spec = Spec::in_dir(project.clone());
        spec.state_dir = Some(root.join("managed"));
        let service = root.join("tools.json");
        std::fs::write(&service,json!({"mcpServers":{"vults-specialist":{"command":"false","args":["--specialist-mcp","/no-host"]}}}).to_string()).unwrap();
        spec.mcp_config = Some(service);
        let mut command = Command::new("codex");
        command
            .arg("app-server")
            .env("CODEX_HOME", &outside)
            .env("OPENAI_API_KEY", "DO_NOT_INHERIT");
        prepare(&spec, &mut command).unwrap();
        assert!(
            !command
                .as_std()
                .get_envs()
                .any(|(key, value)| key == "OPENAI_API_KEY" && value.is_some())
        );
        let mut child = command
            .current_dir(&project)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
        for request in [
            json!({"id":1,"method":"initialize","params":{"clientInfo":{"name":"vults-isolation-check","version":"1"},"capabilities":null}}),
            json!({"method":"initialized"}),
            json!({"id":2,"method":"config/read","params":{"cwd":project,"includeLayers":true}}),
        ] {
            stdin.write_all(format!("{request}\n").as_bytes()).await.unwrap();
        }
        let reply = tokio::time::timeout(std::time::Duration::from_secs(15), async {
            while let Some(line) = lines.next_line().await.unwrap() {
                let reply: Value = serde_json::from_str(&line).unwrap();
                if reply["id"] == 2 {
                    return reply;
                }
            }
            panic!("Codex exited without reporting its isolated configuration");
        })
        .await
        .unwrap();
        assert!(reply.get("error").is_none(), "{reply}");
        let config = &reply["result"]["config"];
        let encoded = config.to_string();
        assert!(!encoded.contains("PROJECT_CANARY"));
        assert!(!encoded.contains("USER_CANARY"));
        assert!(!encoded.contains("project_canary"));
        assert_eq!(config["project_doc_max_bytes"], 0);
        assert_eq!(config["cli_auth_credentials_store"], "ephemeral");
        assert_eq!(config["mcp_servers"]["vults-specialist"]["command"], "false");
        qualify(&reply["result"], true).unwrap();
        child.kill().await.unwrap();
        let _ = child.wait().await;
        assert!(!spec.state_dir.unwrap().join("auth.json").exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}
