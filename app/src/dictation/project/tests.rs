use super::*;

struct Tree(PathBuf);
impl Tree {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "vults-project-words-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
    fn write(&self, path: &str, text: &str) {
        let path = self.0.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    fn repo(&self) -> crate::roosts::Repo {
        crate::roosts::Repo {
            root: self.0.clone(),
            common: self.0.join(".git"),
            origin: None,
            branch: Some("feat/dictation".into()),
        }
    }
}
impl Drop for Tree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn extracts_manifest_names_modules_files_and_branch_without_source_bodies() {
    let tree = Tree::new();
    tree.write(
        "Cargo.toml",
        r#"
        [workspace]
        members = ["./crates/*", "app", "../outside", "/tmp", "linked"]
        [package]
        name = "Nexora"
        [lib]
        name = "nexora_core"
        [[bin]]
        name = "nexora-cli"
        [dependencies]
        serde = "1"
        "Bad name with spaces" = "1"
    "#,
    );
    tree.write("crates/engine/Cargo.toml", "[package]\nname = 'Asterion'\n");
    tree.write(
        "app/package.json",
        r#"{"name":"@nexora/desktop","dependencies":{"Cobalto":"1"}}"#,
    );
    tree.write("package.json", r#"{"name":"NEXORA","workspaces":{"packages":["packages/*"]},"devDependencies":{"Vetria":"1"},"scripts":{"private-body":"do not read this"}}"#);
    tree.write("packages/tools/package.json", r#"{"name":"@nexora/tools"}"#);
    tree.write("src/transcriber.rs", "PRIVATE_SOURCE_BODY");
    tree.write("src/record/mod.rs", "PRIVATE_MODULE_BODY");
    tree.write("README.md", "PRIVATE_README_BODY");
    tree.write(".env", "PRIVATE_SECRET");
    let outside = Tree::new();
    outside.write("Cargo.toml", "[package]\nname = 'ForbiddenLinkedName'\n");
    std::os::unix::fs::symlink(&outside.0, tree.0.join("linked")).unwrap();
    let words = extract(&tree.repo());
    for expected in [
        "feat/dictation",
        "Nexora",
        "nexora_core",
        "nexora-cli",
        "serde",
        "Asterion",
        "@nexora/desktop",
        "Cobalto",
        "Vetria",
        "@nexora/tools",
        "transcriber",
        "record",
        "README.md",
    ] {
        assert!(words.iter().any(|w| w == expected), "{expected}: {words:?}");
    }
    assert_eq!(
        words.iter().filter(|w| w.eq_ignore_ascii_case("nexora")).count(),
        1
    );
    assert!(
        !words.iter().any(|w| w.contains("PRIVATE")
            || w.contains("Forbidden")
            || w.contains(' ')
            || w.starts_with('.'))
    );
    assert!(inside(&tree.0, "../outside").is_none());
    assert!(inside(&tree.0, "/tmp").is_none());
}

#[test]
fn malformed_oversized_and_linked_manifests_fall_back_to_names_with_bounded_words() {
    let tree = Tree::new();
    tree.write("Cargo.toml", "malformed = [");
    tree.write("package.json", &"x".repeat(FILE_BYTES as usize + 1));
    tree.write("src/voice.rs", "ignored");
    let words = extract(&tree.repo());
    assert!(words.contains(&"voice".into()) && words.contains(&"feat/dictation".into()));
    std::fs::remove_file(tree.0.join("Cargo.toml")).unwrap();
    let outside = Tree::new();
    outside.write("Cargo.toml", "[package]\nname = 'ForbiddenSymlinkName'\n");
    std::os::unix::fs::symlink(outside.0.join("Cargo.toml"), tree.0.join("Cargo.toml")).unwrap();
    assert!(!extract(&tree.repo()).contains(&"ForbiddenSymlinkName".into()));
    for n in 0..300 {
        tree.write(&format!("src/module{n}.rs"), "ignored");
    }
    assert_eq!(extract(&tree.repo()).len(), WORDS);
}

fn window(class: &str, title: &str) -> vults_platform::focus::Window {
    vults_platform::focus::Window {
        class: class.into(),
        title: title.into(),
        pid: None,
        identity: "test".into(),
    }
}

#[test]
fn context_prefers_session_cwd_and_rejects_ambiguous_titles_and_unrelated_apps() {
    let cache = Cache::default();
    let entry = |root: &str, word: &str| Entry {
        root: root.into(),
        roost: "owner/nexora".into(),
        words: vec![word.into()],
    };
    cache.insert(entry("/work/nexora", "First"));
    let editor = window("code", "README - nexora - Visual Studio Code");
    assert_eq!(cache.words(&editor, None), ["First"]);
    assert!(cache.words(&window("firefox", "nexora"), None).is_empty());
    assert!(cache.words(&window("code", "nexora-extra"), None).is_empty());
    assert!(cache.words(&window("code", "other"), None).is_empty());
    cache.insert(entry("/other/nexora", "Second"));
    assert!(cache.words(&editor, None).is_empty());
    assert_eq!(
        cache.words(&window("kitty", "shell"), Some("/other/nexora/src")),
        ["Second"]
    );
    assert!(cache.words(&editor, Some("/unknown")).is_empty());
    assert_eq!(
        cache.words(&window("code", "/work/nexora - Code"), None),
        ["First"]
    );
    cache.insert(entry("/work/nexora", "Refreshed"));
    assert_eq!(
        cache.words(&window("kitty", "shell"), Some("/work/nexora")),
        ["Refreshed"]
    );
    for n in 0..PROJECTS {
        cache.insert(entry(&format!("/cache/{n}"), "Bounded"));
    }
    assert_eq!(cache.0.lock().unwrap().len(), PROJECTS);
    assert!(cache.words(&editor, Some("/work/nexora")).is_empty());
}
