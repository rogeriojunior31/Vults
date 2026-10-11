//! Sessions Vults starts for a task (`docs/dev/plan-zeca.md` W4): on the user's click, a git
//! worktree of the task's repository on its own branch, and Claude Code in it through the driver
//! (S8, S11), sandboxed (bubblewrap, a card before each command), with the user's own settings, so
//! our hooks run and the session shows in the flock like any other: its permissions are island
//! cards, answered by a human, a rule or a policy, never here. Only these sessions can be stopped
//! from Vults (D8): ask the turn to stop, wait, then end the process. A watched session never is.
//!
//! Codex waits: its app-server asks for approvals on its own channel, which no card shows yet.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Emitter};
use vults_agents::driver::{
    Answer, Ask, BoxFuture, Event, Input, Interrupter, Role, Sandbox, Settings, Sink, Spec,
};
use vults_store::Store;
use vults_store::tasks::{Change, Task};

/// Told to every window when a run starts or ends (the Tasks tab draws it).
const CHANGED: &str = "tasks-changed";
/// How long a stopped turn has to end before its process is ended.
const STOP_WAIT: Duration = Duration::from_secs(10);

/// A run Vults holds: what stops its turn, and the turn itself.
struct Run {
    interrupter: Arc<dyn Interrupter>,
    turn: tauri::async_runtime::JoinHandle<()>,
}

/// The runs going on now, by task.
fn runs() -> &'static Mutex<HashMap<i64, Run>> {
    static RUNS: std::sync::OnceLock<Mutex<HashMap<i64, Run>>> = std::sync::OnceLock::new();
    RUNS.get_or_init(Mutex::default)
}

/// Whether the task has a session Vults started, running now.
pub fn running(id: i64) -> bool {
    runs().lock().is_ok_and(|r| r.contains_key(&id))
}

fn open() -> Result<Store, String> {
    Store::open(&crate::history::dir()).map_err(|e| e.to_string())
}

/// The windows the user acts in.
fn from_user(window: &tauri::WebviewWindow) -> Result<(), String> {
    if [crate::ISLAND, crate::NEST].contains(&window.label()) {
        Ok(())
    } else {
        Err("only the chat and the Nest can do this".into())
    }
}

/// The task's branch: its own, so two tasks never write the same tree.
fn branch(id: i64) -> String {
    format!("vults/task-{id}")
}

/// Where the task's worktree lives: under Vults' data folder, named after its roost so the flock
/// reads it as that project's (`site-task-12`).
pub fn worktree_dir(task: &Task) -> PathBuf {
    let slug: String = task
        .roost_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .take(40)
        .collect();
    let slug = if slug.trim_matches('-').is_empty() {
        "task".to_string()
    } else {
        slug
    };
    crate::paths::data_dir()
        .join("worktrees")
        .join(format!("{slug}-task-{}", task.id))
}

/// Runs git in `repo`, no prompt, no pager, no network of its own: an error is git's own words.
fn git(repo: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_PAGER", "cat")
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("can't run git: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// The task's worktree, made on first start from the repository's current commit, on the task's
/// branch; reused when it is already there.
fn worktree(task: &Task) -> Result<PathBuf, String> {
    let folder = Path::new(&task.folder);
    let repo = crate::roosts::repo_of(folder).ok_or("This task is not in a git repository.")?;
    let dir = worktree_dir(task);
    if dir.join(".git").exists() {
        return Ok(dir);
    }
    if let Some(parent) = dir.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let name = branch(task.id);
    let exists = git(
        &repo.root,
        &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{name}")],
    )
    .is_ok();
    let dir_arg = dir.to_string_lossy().into_owned();
    if exists {
        git(&repo.root, &["worktree", "add", &dir_arg, &name])?;
    } else {
        git(&repo.root, &["worktree", "add", "-b", &name, &dir_arg, "HEAD"])?;
    }
    Ok(dir)
}

/// What the worker reads before anything: the task, where it works, and what it must not do.
fn brief(task: &Task) -> String {
    let mut text = format!(
        "You work on one task, alone, in a git worktree of {roost} on the branch {branch}. Stay in this folder.\n\nGoal: {goal}\n",
        roost = if task.roost_name.is_empty() {
            "the project"
        } else {
            &task.roost_name
        },
        branch = branch(task.id),
        goal = task.goal,
    );
    if !task.criteria.is_empty() {
        text.push_str("\nIt is done when:\n");
        for c in &task.criteria {
            text.push_str(&format!("- {}\n", c.text));
        }
    }
    text.push_str(
        "\nWhen the criteria are met, commit your work on this branch and say what you did and how each criterion is met. Never push, and never change another branch. If you are stuck, say what you need.",
    );
    text
}

/// Where a run's driver reports. Cards come through the hooks; the driver's own channel asks only
/// when no card could (the app paused, a card gone), and then the answer is no.
struct RunSink {
    task: i64,
    cost: Mutex<f64>,
}

impl Sink for RunSink {
    fn event(&self, event: Event) -> BoxFuture<'_, ()> {
        Box::pin(async move {
            match event {
                Event::Usage(u) => {
                    if let (Some(c), Ok(mut cost)) = (u.cost_usd, self.cost.lock()) {
                        *cost += c;
                    }
                }
                Event::TurnEnd(reason) => {
                    tracing::info!(task = self.task, ?reason, "task run's turn ended");
                }
                _ => {}
            }
        })
    }

    fn ask(&self, ask: Ask) -> BoxFuture<'_, Answer> {
        Box::pin(async move {
            tracing::info!(task = self.task, tool = %ask.tool, "a task run asked outside a card: no");
            Answer::Deny
        })
    }
}

/// **Start**: the task's worktree, and Claude Code in it on its brief. The task becomes Claude
/// Code's and *Doing*.
#[tauri::command]
pub async fn task_start(app: AppHandle, window: tauri::WebviewWindow, id: i64) -> Result<(), String> {
    from_user(&window)?;
    if running(id) {
        return Err("This task is already running.".into());
    }
    let (task, dir) = tauri::async_runtime::spawn_blocking(move || {
        let task = open()?
            .task(id)
            .map_err(|e| e.to_string())?
            .ok_or("No such task.")?;
        if matches!(task.status.as_str(), "done" | "dropped") {
            return Err("This task is finished.".to_string());
        }
        let dir = worktree(&task)?;
        Ok::<_, String>((task, dir))
    })
    .await
    .map_err(|e| e.to_string())??;
    let spec = Spec {
        // The user's own settings: our hooks run, so the flock shows it and its cards are the
        // island's. Never the project's: a worktree path is one Claude Code never trusted.
        settings: Settings::User,
        sandbox: Sandbox::Native,
        role: Role::Worker,
        system_append: Some(brief(&task)),
        mcp_config: crate::mcp::config_file(),
        env: vec![("VULTS_TASK".into(), id.to_string())],
        ..Spec::in_dir(dir)
    };
    let mut driver = vults_chat::drivers::for_cli(vults_chat::Provider::Claude, spec)
        .ok_or("Claude Code can't run here.")?;
    let interrupter = driver.interrupter();
    let done_app = app.clone();
    let turn = tauri::async_runtime::spawn(async move {
        let sink = RunSink {
            task: id,
            cost: Mutex::new(0.0),
        };
        let input = Input {
            text: "Start on the task.".into(),
            ..Input::default()
        };
        let result = match driver.start().await {
            Ok(()) => driver.send(&input, &sink).await,
            Err(e) => Err(e),
        };
        if let Err(e) = &result {
            tracing::warn!(task = id, error = %e, "task run failed");
        }
        let cost = sink.cost.lock().map(|c| *c).unwrap_or(0.0);
        let _ = tauri::async_runtime::spawn_blocking(move || {
            let mut store = open()?;
            if cost > 0.0 {
                store.task_cost(id, cost).map_err(|e| e.to_string())?;
            }
            if result.is_err() {
                store
                    .task_change(id, &Change::Status("blocked".into()), crate::traces::now_ms())
                    .map_err(|e| e.to_string())?;
            }
            Ok::<_, String>(())
        })
        .await;
        drop(driver);
        if let Ok(mut r) = runs().lock() {
            r.remove(&id);
        }
        let _ = done_app.emit(CHANGED, ());
    });
    if let Ok(mut r) = runs().lock() {
        r.insert(id, Run { interrupter, turn });
    }
    tauri::async_runtime::spawn_blocking(move || {
        let mut store = open()?;
        let now = crate::traces::now_ms();
        store
            .task_change(id, &Change::Owner("claude".into()), now)
            .map_err(|e| e.to_string())?;
        store
            .task_change(id, &Change::Status("doing".into()), now)
            .map_err(|e| e.to_string())?;
        Ok::<_, String>(())
    })
    .await
    .map_err(|e| e.to_string())??;
    let _ = app.emit(CHANGED, ());
    Ok(())
}

/// **Stop**: asks the run's turn to stop, waits for it, and ends its process when it doesn't. Only
/// a run Vults started; the task goes back to *To do*.
#[tauri::command]
pub async fn task_stop(app: AppHandle, window: tauri::WebviewWindow, id: i64) -> Result<(), String> {
    from_user(&window)?;
    let run = runs()
        .lock()
        .ok()
        .and_then(|mut r| r.remove(&id))
        .ok_or("This task isn't running.")?;
    let _ = run.interrupter.interrupt().await;
    let turn = run.turn;
    // Ending the task (a dropped turn drops the driver, which ends its process) when the turn
    // doesn't end on its own in time.
    if tokio::time::timeout(STOP_WAIT, async {
        while !turn.inner().is_finished() {
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    })
    .await
    .is_err()
    {
        turn.abort();
    }
    tauri::async_runtime::spawn_blocking(move || {
        open()?
            .task_change(id, &Change::Status("open".into()), crate::traces::now_ms())
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;
    let _ = app.emit(CHANGED, ());
    Ok(())
}

/// **Remove worktree**: the task's worktree goes (its branch, and the commits on it, stay). Git
/// refuses while it holds uncommitted changes; nothing is forced.
#[tauri::command]
pub async fn task_remove_worktree(
    app: AppHandle,
    window: tauri::WebviewWindow,
    id: i64,
) -> Result<(), String> {
    from_user(&window)?;
    if running(id) {
        return Err("Stop the task first.".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let task = open()?
            .task(id)
            .map_err(|e| e.to_string())?
            .ok_or("No such task.")?;
        let dir = worktree_dir(&task);
        if !dir.exists() {
            return Ok(());
        }
        let repo =
            crate::roosts::repo_of(Path::new(&task.folder)).ok_or("This task is not in a git repository.")?;
        git(&repo.root, &["worktree", "remove", &dir.to_string_lossy()]).map(|_| ())
    })
    .await
    .map_err(|e| e.to_string())??;
    let _ = app.emit(CHANGED, ());
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn task(id: i64, roost: &str) -> Task {
        Task {
            id,
            created_ms: 0,
            updated_ms: 0,
            goal: "Fix the login test".into(),
            criteria: vec![vults_store::tasks::Criterion {
                text: "passes 20 runs".into(),
                done: false,
            }],
            roost: String::new(),
            roost_name: roost.into(),
            folder: String::new(),
            owner: "you".into(),
            status: "open".into(),
            cost_usd: None,
            issue: None,
        }
    }

    #[test]
    fn each_task_has_its_own_branch_and_worktree_named_after_its_roost() {
        assert_eq!(branch(12), "vults/task-12");
        let a = worktree_dir(&task(12, "site"));
        let b = worktree_dir(&task(13, "site"));
        assert_ne!(a, b, "two tasks never share a tree");
        assert!(a.ends_with("worktrees/site-task-12"));
        assert!(
            worktree_dir(&task(4, "../../etc")).ends_with("worktrees/------etc-task-4"),
            "no way out of the folder"
        );
        assert!(worktree_dir(&task(5, "")).ends_with("worktrees/task-task-5"));
    }

    #[test]
    fn the_brief_holds_the_goal_the_criteria_and_the_limits() {
        let b = brief(&task(12, "site"));
        assert!(b.contains("Goal: Fix the login test"));
        assert!(b.contains("- passes 20 runs"));
        assert!(b.contains("vults/task-12"));
        assert!(b.contains("Never push"));
    }

    #[cfg(unix)]
    #[test]
    fn two_tasks_get_two_worktrees_of_one_repository() {
        let root = std::env::temp_dir().join(format!("vults-runs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let repo = root.join("site");
        std::fs::create_dir_all(&repo).unwrap();
        let g = |args: &[&str]| git(&repo, args).unwrap();
        g(&["init", "-q", "-b", "main"]);
        g(&[
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "first",
        ]);
        let add = |id: i64| {
            let dir = root.join(format!("site-task-{id}"));
            git(
                &repo,
                &[
                    "worktree",
                    "add",
                    "-b",
                    &branch(id),
                    &dir.to_string_lossy(),
                    "HEAD",
                ],
            )
            .unwrap();
            dir
        };
        let (a, b) = (add(1), add(2));
        assert!(a.join(".git").exists() && b.join(".git").exists());
        assert_eq!(git(&a, &["branch", "--show-current"]).unwrap(), "vults/task-1");
        assert_eq!(git(&b, &["branch", "--show-current"]).unwrap(), "vults/task-2");
        // Removing one leaves its branch, and never forces a tree with changes away.
        std::fs::write(b.join("wip.txt"), "x").unwrap();
        assert!(
            git(&repo, &["worktree", "remove", &b.to_string_lossy()]).is_err(),
            "uncommitted work stays"
        );
        git(&repo, &["worktree", "remove", &a.to_string_lossy()]).unwrap();
        assert!(
            git(
                &repo,
                &["rev-parse", "--verify", "--quiet", "refs/heads/vults/task-1"]
            )
            .is_ok()
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
