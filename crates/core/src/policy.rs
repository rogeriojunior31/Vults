//! What kind of action a request is (ADR 0014, `docs/dev/plan-zeca.md` S3). One taxonomy for the
//! agents' tools and, later, Zeca's own actions. Today it only classifies: no policy answers
//! anything until the policy engine exists (W2), and a destructive action always asks.
//!
//! It errs on the side of danger: a miss must make an action look worse, never safer. W2 still
//! never allows by a pattern alone what the shell could read otherwise.

use serde::Serialize;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case", tag = "class")]
pub enum Class {
    /// Reads this machine: a file, a search, a listing.
    Read,
    /// Reads beyond it: a web page, a search engine. Its content is untrusted.
    ReadExternal,
    /// Costs money or quota beyond the turn itself.
    Spend,
    /// Changes this machine; `destructive` when it can't be undone (`rm`, a forced push).
    Write { destructive: bool },
    /// Acts in the world beyond this machine: a push, a message, a deploy.
    External,
}

/// The class of an agent's tool call. `target` must be the whole thing the call acts on (a card's
/// `ask.full` when the card shows only its start), never the cut text. Unknown tools, MCP tools
/// included whatever their name says, are writes.
pub fn classify(tool: &str, target: &str) -> Class {
    if tool.starts_with("mcp__") {
        return Class::Write { destructive: false };
    }
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

const DESTRUCTIVE: Class = Class::Write { destructive: true };
const WRITE: Class = Class::Write { destructive: false };

/// A shell command: each piece between `;`, `&`, `|`, `$(`, backticks and new lines is read on its
/// own (quotes dropped, so `bash -c 'rm -rf x'` shows its `rm`), and the worst piece wins.
fn shell(cmd: &str) -> Class {
    let mut worst = WRITE;
    for piece in cmd.split(['\n', ';', '&', '|', '`', '(', ')']) {
        let words: Vec<String> = piece
            .split_whitespace()
            .map(|w| {
                w.trim_matches(|c| c == '\'' || c == '"' || c == '$' || c == '{' || c == '}')
                    .to_string()
            })
            .filter(|w| !w.is_empty())
            .collect();
        worst = worst.max(command(&words));
    }
    worst.max(redirection(cmd))
}

/// `> file` overwrites it; a device is worse still. Appending (`>>`), `/dev/null` and `>&` are fine.
fn redirection(cmd: &str) -> Class {
    let bytes = cmd.as_bytes();
    for (i, _) in cmd.match_indices('>') {
        let doubled = bytes.get(i + 1) == Some(&b'>') || i > 0 && bytes[i - 1] == b'>';
        if doubled || bytes.get(i + 1) == Some(&b'&') {
            continue;
        }
        let to = cmd[i + 1..].split_whitespace().next().unwrap_or("");
        if to.is_empty() || to == "/dev/null" || to.starts_with("/dev/std") {
            continue;
        }
        return DESTRUCTIVE;
    }
    WRITE
}

fn base(w: &str) -> &str {
    w.rsplit('/').next().unwrap_or(w)
}

/// One simple command, its words already unquoted.
fn command(words: &[String]) -> Class {
    // Wrappers that run the rest: the command is what follows them.
    let mut i = 0;
    while let Some(w) = words.get(i) {
        let b = base(w);
        if w.contains('=') && !w.starts_with('-') && i == 0
            || matches!(
                b,
                "sudo"
                    | "doas"
                    | "nohup"
                    | "time"
                    | "command"
                    | "exec"
                    | "nice"
                    | "env"
                    | "xargs"
                    | "timeout"
            )
            || (i > 0
                && w.starts_with('-')
                && matches!(base(&words[i - 1]), "sudo" | "xargs" | "env" | "nice" | "timeout"))
        {
            i += 1;
        } else {
            break;
        }
    }
    let words = &words[i..];
    let Some(first) = words.first() else {
        return WRITE;
    };
    let rest = &words[1..];
    let has = |w: &str| rest.iter().any(|x| x == w);
    let flag = |c: char| {
        rest.iter()
            .any(|x| x.starts_with('-') && !x.starts_with("--") && x.contains(c))
    };
    match base(first) {
        // A shell running a string: what it runs is the command.
        "sh" | "bash" | "zsh" | "dash" | "fish" | "ksh" => match rest.iter().position(|w| w == "-c") {
            Some(c) => command(&rest[c + 1..]),
            None => WRITE,
        },
        "eval" => command(rest),
        "rm" | "shred" | "unlink" | "wipefs" | "truncate" | "dd" | "mv" | "srm" => DESTRUCTIVE,
        p if p.starts_with("mkfs") => DESTRUCTIVE,
        "find" if has("-delete") || has("-exec") && rest.iter().any(|w| base(w) == "rm") => DESTRUCTIVE,
        "chmod" | "chown" if flag('R') || has("--recursive") => DESTRUCTIVE,
        "git" => git(rest),
        "docker" | "podman" => match rest.first().map(String::as_str) {
            Some("rm" | "rmi" | "prune") => DESTRUCTIVE,
            Some("system" | "volume" | "image" | "container" | "network")
                if rest.iter().any(|w| matches!(w.as_str(), "prune" | "rm")) =>
            {
                DESTRUCTIVE
            }
            Some("push") => Class::External,
            _ => WRITE,
        },
        "kubectl" | "helm" => match rest.first().map(String::as_str) {
            Some("delete" | "uninstall" | "drain") => DESTRUCTIVE,
            Some("apply" | "create" | "replace" | "patch" | "scale" | "rollout" | "install" | "upgrade") => {
                Class::External
            }
            _ => WRITE,
        },
        "gh" => match rest.first().map(String::as_str) {
            Some("repo") if has("delete") => DESTRUCTIVE,
            Some("pr" | "issue" | "release" | "gist" | "workflow" | "api") => Class::External,
            _ => WRITE,
        },
        "npm" | "pnpm" | "yarn" | "cargo" if has("publish") => Class::External,
        "ssh" | "scp" | "rsync" | "sftp" | "ftp" | "nc" | "telnet" => Class::External,
        "curl"
            if rest.iter().any(|w| {
                matches!(
                    w.as_str(),
                    "-d" | "--data"
                        | "--data-raw"
                        | "--data-binary"
                        | "-F"
                        | "--form"
                        | "-T"
                        | "--upload-file"
                ) || w.starts_with("--data")
            }) || rest.windows(2).any(|p| p[0] == "-X" && p[1] != "GET") =>
        {
            Class::External
        }
        "wget" if rest.iter().any(|w| w.starts_with("--post")) => Class::External,
        _ => WRITE,
    }
}

fn git(args: &[String]) -> Class {
    // Global options before the subcommand: `-C dir`, `-c k=v`, `--git-dir=…`.
    let mut i = 0;
    while let Some(a) = args.get(i) {
        if a == "-C" || a == "-c" {
            i += 2;
        } else if a.starts_with('-') {
            i += 1;
        } else {
            break;
        }
    }
    let Some(sub) = args.get(i) else {
        return WRITE;
    };
    let rest = &args[i + 1..];
    let has = |w: &str| rest.iter().any(|x| x == w);
    let short = |c: char| {
        rest.iter()
            .any(|x| x.starts_with('-') && !x.starts_with("--") && x.contains(c))
    };
    match sub.as_str() {
        "push" => {
            let rewrites = short('f')
                || short('d')
                || rest.iter().any(|w| {
                    w.starts_with("--force") || matches!(w.as_str(), "--delete" | "--mirror" | "--prune")
                })
                || rest.iter().any(|w| w.starts_with('+') || w.starts_with(':'));
            if rewrites { DESTRUCTIVE } else { Class::External }
        }
        "reset" if has("--hard") => DESTRUCTIVE,
        "clean" if short('f') || has("--force") => DESTRUCTIVE,
        "checkout" if has("--") || has(".") || short('f') || has("--force") => DESTRUCTIVE,
        "restore" if !has("--staged") => DESTRUCTIVE,
        "branch" if short('D') || has("--delete") && has("--force") => DESTRUCTIVE,
        "stash" if has("drop") || has("clear") => DESTRUCTIVE,
        "filter-branch" | "filter-repo" => DESTRUCTIVE,
        "reflog" if has("expire") || has("delete") => DESTRUCTIVE,
        "gc" if rest.iter().any(|w| w.starts_with("--prune")) => DESTRUCTIVE,
        _ => WRITE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_have_their_class() {
        assert_eq!(classify("Read", "src/main.rs"), Class::Read);
        assert_eq!(classify("WebFetch", "https://a.dev"), Class::ReadExternal);
        assert_eq!(classify("Edit", "src/main.rs"), WRITE);
        assert_eq!(classify("SomethingNew", "x"), WRITE);
        // An MCP tool's name says nothing about what it does.
        assert_eq!(classify("mcp__fs__read_file", "a"), WRITE);
        assert_eq!(classify("mcp__x__Read", "a"), WRITE);
    }

    #[test]
    fn shell_commands_are_read_for_what_they_can_break() {
        for cmd in [
            "rm -rf build",
            "rm notes.txt",
            "rm --recursive --force x",
            "sudo rm -fr /var/x",
            "ls | xargs rm -rf",
            "bash -c 'rm -rf x'",
            "true;rm -rf x",
            "echo $(rm -rf x)",
            "cargo test\nrm -rf ~",
            "find . -name '*.o' -delete",
            "find . -exec rm {} +",
            "truncate -s0 log.txt",
            "echo hi > notes.txt",
            "cat x >/dev/sda",
            "mv a.rs b.rs",
            "git push --force origin main",
            "git push -f",
            "git -C repo push -f",
            "git push --force-with-lease",
            "git push origin :old-branch",
            "git push origin --delete old",
            "git push origin +main",
            "git reset --hard HEAD~3",
            "git clean -fdx",
            "git checkout -- .",
            "git restore .",
            "git branch -D feat",
            "git stash drop",
            "dd if=/dev/zero of=/dev/sda",
            "mkfs.ext4 /dev/sdb1",
            "chmod -R 777 /",
            "docker system prune -a",
            "kubectl delete pod x",
            "gh repo delete me/x --yes",
        ] {
            assert_eq!(classify("Bash", &format!("Bash · {cmd}")), DESTRUCTIVE, "{cmd}");
        }
        for cmd in [
            "git push origin feat",
            "git -c user.name=me push",
            "gh pr create --fill",
            "npm publish",
            "rsync -a a/ host:b/",
            "ssh host uptime",
            "curl -X POST https://api",
            "curl -d x=1 https://api",
            "docker push img",
            "kubectl apply -f k.yml",
        ] {
            assert_eq!(classify("Bash", cmd), Class::External, "{cmd}");
        }
        for cmd in [
            "cargo test",
            "ls -la 2>/dev/null",
            "cargo build >> build.log 2>&1",
            "git commit -m 'rm -rf talk'",
            "git reset HEAD a.rs",
            "git restore --staged a.rs",
            "curl https://example.com",
        ] {
            assert_eq!(classify("Bash", cmd), WRITE, "{cmd}");
        }
    }
}
