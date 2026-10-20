use super::*;
fn directory() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "vults-question-install-{}-{}",
        std::process::id(),
        crate::traces::new_id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
#[test]
fn config_review_keeps_other_servers_and_trust_and_makes_a_dated_backup() {
    let dir = directory();
    let program = Path::new("/opt/vults");
    for agent in ["claude", "codex"] {
        let path = dir.join(agent);
        let original = if agent == "claude" {
            "{\"oauthAccount\":{\"id\":\"retained\"},\"mcpServers\":{\"other\":{\"command\":\"other\"}}}"
        } else {
            "# Keep my layout\nmodel = 'sample'\n[hooks]\ntrusted_hash = 'unchanged'\n[mcp_servers.other]\ncommand = 'other'\n"
        };
        std::fs::write(&path, original).unwrap();
        let preview = preview_at(agent, &path, program, true).unwrap();
        assert!(preview.diff.contains(ARG));
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            original,
            "Preview does not write"
        );
        let backup = apply_at(
            agent,
            Plan {
                path: path.clone(),
                program: program.into(),
                install: true,
                preview,
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(std::fs::read_to_string(&backup).unwrap(), original);
        assert!(backup.file_name().unwrap().to_str().unwrap().contains(".bak-"));
        let installed = std::fs::read_to_string(&path).unwrap();
        if agent == "codex" {
            assert!(installed.starts_with("# Keep my layout"));
            assert!(installed.contains("trusted_hash = 'unchanged'"));
            assert!(installed.contains("tool_timeout_sec = 150"));
            assert!(installed.contains("[mcp_servers.other]"));
        } else {
            let json: Value = serde_json::from_str(&installed).unwrap();
            assert_eq!(json["oauthAccount"]["id"], "retained");
            assert_eq!(json["mcpServers"]["other"]["command"], "other");
        }
        assert!(preview_at(agent, &path, program, true).unwrap().is_noop());
        let updated = Path::new("/opt/updated/vults");
        let update = preview_at(agent, &path, updated, true).unwrap();
        assert!(
            !update.is_noop(),
            "A managed entry can follow the app to another installed path"
        );
        let update_backup = apply_at(
            agent,
            Plan {
                path: path.clone(),
                program: updated.into(),
                install: true,
                preview: update,
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(std::fs::read_to_string(update_backup).unwrap(), installed);

        let removal = preview_at(agent, &path, updated, false).unwrap();
        apply_at(
            agent,
            Plan {
                path: path.clone(),
                program: updated.into(),
                install: false,
                preview: removal,
            },
        )
        .unwrap();
        let removed = std::fs::read_to_string(&path).unwrap();
        assert!(!removed.contains(NAME));
        assert!(removed.contains("other"));
    }
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn malformed_foreign_and_changed_configurations_are_never_replaced() {
    let dir = directory();
    let program = Path::new("/opt/vults");
    for (agent, text) in [
        ("claude", "{ broken"),
        ("claude", "{\"mcpServers\":[]}"),
        (
            "claude",
            "{\"mcpServers\":{\"vults-questions\":{\"command\":\"mine\",\"args\":[\"--questions-mcp\"]}}}",
        ),
        ("codex", "broken = ["),
        ("codex", "mcp_servers = []"),
        (
            "codex",
            "[mcp_servers.vults-questions]\ncommand = 'mine'\nargs = ['--questions-mcp']",
        ),
    ] {
        let path = dir.join(agent);
        std::fs::write(&path, text).unwrap();
        assert!(preview_at(agent, &path, program, true).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
    }
    for agent in ["claude", "codex"] {
        let path = dir.join(agent);
        std::fs::write(&path, if agent == "claude" { "{}" } else { "# original\n" }).unwrap();
        let preview = preview_at(agent, &path, program, true).unwrap();
        let changed = if agent == "claude" {
            "{\"changed\":true}"
        } else {
            "# changed\n"
        };
        std::fs::write(&path, changed).unwrap();
        assert!(
            apply_at(
                agent,
                Plan {
                    path: path.clone(),
                    program: program.into(),
                    install: true,
                    preview
                }
            )
            .is_err()
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), changed);
    }
    std::fs::remove_dir_all(dir).unwrap();
}
