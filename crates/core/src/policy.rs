//! What kind of action a request is (ADR 0014, `docs/dev/plan-zeca.md` S3). One taxonomy for the
//! agents' tools and, later, Zeca's own actions. Today it only classifies: no policy answers
//! anything until the policy engine exists (W2), and a destructive action always asks.

use serde::Serialize;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case", tag = "class")]
pub enum Class {
    /// Reads this machine: a file, a search, a listing.
    Read,
    /// Reads beyond it: a web page, a search engine. Its content is untrusted.
    ReadExternal,
    /// Changes this machine; `destructive` when it can't be undone (`rm -rf`, a forced push).
    Write { destructive: bool },
    /// Costs money or quota beyond the turn itself.
    Spend,
    /// Acts in the world beyond this machine: a push, a message, a deploy.
    External,
}

/// The class of an agent's tool call, from its tool name and target as the card shows them. Unknown
/// tools are writes: a guess never makes an action look safer than it is.
pub fn classify(tool: &str, target: &str) -> Class {
    let tool = tool.rsplit("__").next().unwrap_or(tool);
    match tool {
        "Read"
        | "Glob"
        | "Grep"
        | "LS"
        | "NotebookRead"
        | "TodoRead"
        | "read_file"
        | "list_directory"
        | "glob"
        | "search_file_content"
        | "read_many_files" => Class::Read,
        "WebFetch" | "WebSearch" | "web_fetch" | "google_web_search" => Class::ReadExternal,
        "Edit" | "MultiEdit" | "Write" | "NotebookEdit" | "TodoWrite" | "replace" | "write_file" => {
            Class::Write { destructive: false }
        }
        "Bash" | "shell" | "run_shell_command" | "exec_command" => shell(command_of(target)),
        _ => Class::Write { destructive: false },
    }
}

/// The card's target is `Tool · command` for a shell; the command alone otherwise.
fn command_of(target: &str) -> &str {
    target.split_once(" · ").map_or(target, |(_, c)| c)
}

/// A shell command: destructive, external, or a plain write.
fn shell(cmd: &str) -> Class {
    let words: Vec<&str> = cmd.split_whitespace().collect();
    let has = |w: &str| words.contains(&w);
    let after = |first: &str, second: &str| {
        words
            .windows(2)
            .any(|p| p[0].rsplit('/').next() == Some(first) && p[1] == second)
    };
    let destructive = (words.iter().any(|w| w.rsplit('/').next() == Some("rm"))
        && words
            .iter()
            .any(|w| w.starts_with('-') && !w.starts_with("--") && w.contains('r')))
        || has("--force") && (after("git", "push") || after("git", "clean"))
        || after("git", "push") && words.iter().any(|w| w.starts_with("+") || *w == "-f")
        || after("git", "reset") && has("--hard")
        || after("git", "clean") && words.iter().any(|w| w.starts_with('-') && w.contains('f'))
        || words
            .iter()
            .any(|w| matches!(w.rsplit('/').next(), Some("dd" | "mkfs" | "shred" | "wipefs")))
        || words.iter().any(|w| w.starts_with("mkfs."))
        || cmd.contains("> /dev/sd")
        || cmd.contains("> /dev/nvme")
        || after("chmod", "-R") && has("777");
    if destructive {
        return Class::Write { destructive: true };
    }
    let external = after("git", "push")
        || after("gh", "pr")
            && words
                .iter()
                .any(|w| matches!(*w, "create" | "merge" | "comment" | "close"))
        || after("gh", "release")
        || after("npm", "publish")
        || after("cargo", "publish")
        || words
            .iter()
            .any(|w| matches!(w.rsplit('/').next(), Some("scp" | "rsync")));
    if external {
        return Class::External;
    }
    Class::Write { destructive: false }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_have_their_class() {
        assert_eq!(classify("Read", "src/main.rs"), Class::Read);
        assert_eq!(classify("mcp__fs__read_file", "a"), Class::Read);
        assert_eq!(classify("WebFetch", "https://a.dev"), Class::ReadExternal);
        assert_eq!(
            classify("Edit", "src/main.rs"),
            Class::Write { destructive: false }
        );
        assert_eq!(classify("SomethingNew", "x"), Class::Write { destructive: false });
    }

    #[test]
    fn shell_commands_are_read_for_what_they_can_break() {
        let destructive = Class::Write { destructive: true };
        for cmd in [
            "rm -rf build",
            "rm -r ~/old",
            "sudo rm -fr /var/x",
            "git push --force origin main",
            "git push origin +main",
            "git reset --hard HEAD~3",
            "git clean -fdx",
            "dd if=/dev/zero of=/dev/sda",
            "mkfs.ext4 /dev/sdb1",
            "chmod -R 777 /",
        ] {
            assert_eq!(classify("Bash", &format!("Bash · {cmd}")), destructive, "{cmd}");
        }
        for cmd in [
            "git push origin feat",
            "gh pr create --fill",
            "npm publish",
            "rsync -a a/ host:b/",
        ] {
            assert_eq!(classify("Bash", cmd), Class::External, "{cmd}");
        }
        for cmd in [
            "cargo test",
            "rm notes.txt",
            "git commit -m 'rm -rf talk'",
            "ls -la",
            "git reset HEAD a.rs",
        ] {
            assert_ne!(classify("Bash", cmd), destructive, "{cmd}");
        }
    }
}
