//! Tasks (`docs/dev/plan-zeca.md` W3): what the user wants done, with how they will know it is,
//! where (a roost), who works on it, how far it got and what it cost. The user makes them (`/task`
//! in the chat, the Nest's Tasks tab); W4 starts sessions for them and adds their cost.

use rusqlite::{OptionalExtension, params};

use crate::{Error, Store};

/// Migration 14. Shipped: never edit it.
pub(crate) const TABLE: &str = "
    CREATE TABLE tasks (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        created_ms INTEGER NOT NULL,
        updated_ms INTEGER NOT NULL,
        goal TEXT NOT NULL,
        roost TEXT NOT NULL DEFAULT '',
        roost_name TEXT NOT NULL DEFAULT '',
        folder TEXT NOT NULL DEFAULT '',
        owner TEXT NOT NULL DEFAULT 'you',
        status TEXT NOT NULL DEFAULT 'open',
        cost_usd REAL,
        issue TEXT
    );
    CREATE INDEX tasks_status ON tasks (status, updated_ms);
    CREATE TABLE task_criteria (
        task INTEGER NOT NULL REFERENCES tasks (id) ON DELETE CASCADE,
        idx INTEGER NOT NULL,
        text TEXT NOT NULL,
        done INTEGER NOT NULL DEFAULT 0,
        PRIMARY KEY (task, idx)
    );";

/// Where a task stands.
pub const STATUSES: &[&str] = &["open", "doing", "blocked", "done", "dropped"];
/// The statuses still to do: listed first.
const OPEN: &[&str] = &["open", "doing", "blocked"];
/// Longest goal and criterion kept.
pub const MAX_TEXT: usize = 500;
/// Most criteria on one task.
pub const MAX_CRITERIA: usize = 20;

/// A task as made.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct New {
    pub goal: String,
    pub criteria: Vec<String>,
    /// The roost's key (`github:owner/repo`, or a folder) and name; empty for none.
    pub roost: String,
    pub roost_name: String,
    /// The folder it was made from.
    pub folder: String,
    /// `you`, `zeca`, or an agent's name.
    pub owner: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Criterion {
    pub text: String,
    pub done: bool,
}

/// A task as read back.
#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    pub id: i64,
    pub created_ms: i64,
    pub updated_ms: i64,
    pub goal: String,
    pub criteria: Vec<Criterion>,
    pub roost: String,
    pub roost_name: String,
    pub folder: String,
    pub owner: String,
    pub status: String,
    pub cost_usd: Option<f64>,
    /// A GitHub issue's address, once linked.
    pub issue: Option<String>,
}

/// One change to a task.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    Status(String),
    Owner(String),
    /// A criterion met, or not, by its place.
    Criterion(usize, bool),
    /// The issue linked, or none.
    Issue(Option<String>),
}

fn cut(text: &str) -> String {
    text.trim().chars().take(MAX_TEXT).collect()
}

impl Store {
    /// Keeps a new task; its criteria are the non-empty lines given, at most [`MAX_CRITERIA`].
    pub fn task_add(&mut self, task: &New, ms: i64) -> Result<i64, Error> {
        let goal = cut(&task.goal);
        if goal.is_empty() {
            return Err(Error::Invalid("a task needs a goal"));
        }
        let owner = if task.owner.is_empty() {
            "you"
        } else {
            task.owner.as_str()
        };
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO tasks (created_ms, updated_ms, goal, roost, roost_name, folder, owner)
             VALUES (?1, ?1, ?2, ?3, ?4, ?5, ?6)",
            params![ms, goal, task.roost, task.roost_name, task.folder, owner],
        )?;
        let id = tx.last_insert_rowid();
        let criteria = task
            .criteria
            .iter()
            .map(|c| cut(c))
            .filter(|c| !c.is_empty())
            .take(MAX_CRITERIA);
        for (idx, text) in criteria.enumerate() {
            tx.execute(
                "INSERT INTO task_criteria (task, idx, text) VALUES (?1, ?2, ?3)",
                params![id, idx as i64, text],
            )?;
        }
        tx.commit()?;
        Ok(id)
    }

    /// The tasks still to do, newest change first, then (with `closed`) the done and dropped ones;
    /// at most `limit` of each.
    pub fn tasks(&self, closed: bool, limit: u32) -> Result<Vec<Task>, Error> {
        let mut out = Vec::new();
        let wanted: &[bool] = if closed { &[true, false] } else { &[true] };
        for &open in wanted {
            let list = OPEN
                .iter()
                .map(|s| format!("'{s}'"))
                .collect::<Vec<_>>()
                .join(",");
            let sql = format!(
                "SELECT id FROM tasks WHERE status {} IN ({list}) ORDER BY updated_ms DESC, id DESC LIMIT ?1",
                if open { "" } else { "NOT" }
            );
            let ids: Vec<i64> = {
                let mut stmt = self.conn.prepare(&sql)?;
                stmt.query_map([limit], |r| r.get(0))?.collect::<Result<_, _>>()?
            };
            for id in ids {
                if let Some(t) = self.task(id)? {
                    out.push(t);
                }
            }
        }
        Ok(out)
    }

    pub fn task(&self, id: i64) -> Result<Option<Task>, Error> {
        let Some(mut task) = self
            .conn
            .query_row(
                "SELECT id, created_ms, updated_ms, goal, roost, roost_name, folder, owner, status, cost_usd,
                        issue
                 FROM tasks WHERE id = ?1",
                [id],
                |r| {
                    Ok(Task {
                        id: r.get(0)?,
                        created_ms: r.get(1)?,
                        updated_ms: r.get(2)?,
                        goal: r.get(3)?,
                        criteria: Vec::new(),
                        roost: r.get(4)?,
                        roost_name: r.get(5)?,
                        folder: r.get(6)?,
                        owner: r.get(7)?,
                        status: r.get(8)?,
                        cost_usd: r.get(9)?,
                        issue: r.get(10)?,
                    })
                },
            )
            .optional()?
        else {
            return Ok(None);
        };
        let mut stmt = self
            .conn
            .prepare("SELECT text, done FROM task_criteria WHERE task = ?1 ORDER BY idx")?;
        task.criteria = stmt
            .query_map([id], |r| {
                Ok(Criterion {
                    text: r.get(0)?,
                    done: r.get::<_, i64>(1)? != 0,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(Some(task))
    }

    /// Applies one change; false when there is no such task (or criterion).
    pub fn task_change(&mut self, id: i64, change: &Change, ms: i64) -> Result<bool, Error> {
        let changed = match change {
            Change::Status(s) => {
                if !STATUSES.contains(&s.as_str()) {
                    return Err(Error::Invalid("no such status"));
                }
                self.conn
                    .execute("UPDATE tasks SET status = ?2 WHERE id = ?1", params![id, s])?
            }
            Change::Owner(o) => {
                let o = cut(o);
                if o.is_empty() {
                    return Err(Error::Invalid("a task needs an owner"));
                }
                self.conn
                    .execute("UPDATE tasks SET owner = ?2 WHERE id = ?1", params![id, o])?
            }
            Change::Criterion(idx, done) => self.conn.execute(
                "UPDATE task_criteria SET done = ?3 WHERE task = ?1 AND idx = ?2",
                params![id, *idx as i64, i64::from(*done)],
            )?,
            Change::Issue(url) => self
                .conn
                .execute("UPDATE tasks SET issue = ?2 WHERE id = ?1", params![id, url])?,
        };
        if changed > 0 {
            self.conn
                .execute("UPDATE tasks SET updated_ms = ?2 WHERE id = ?1", params![id, ms])?;
        }
        Ok(changed > 0)
    }

    /// Adds what a session started for it cost (W4).
    pub fn task_cost(&mut self, id: i64, usd: f64) -> Result<(), Error> {
        self.conn.execute(
            "UPDATE tasks SET cost_usd = COALESCE(cost_usd, 0) + ?2 WHERE id = ?1",
            params![id, usd],
        )?;
        Ok(())
    }

    /// Removes a task and its criteria.
    pub fn task_delete(&mut self, id: i64) -> Result<bool, Error> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM task_criteria WHERE task = ?1", [id])?;
        let gone = tx.execute("DELETE FROM tasks WHERE id = ?1", [id])?;
        tx.commit()?;
        Ok(gone > 0)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::temp;

    fn new(goal: &str, criteria: &[&str]) -> New {
        New {
            goal: goal.into(),
            criteria: criteria.iter().map(|c| (*c).to_string()).collect(),
            roost: "github:me/site".into(),
            roost_name: "site".into(),
            folder: "/home/me/site".into(),
            owner: String::new(),
        }
    }

    #[test]
    fn a_task_keeps_its_criteria_in_order_and_changes() {
        let mut s = Store::open(&temp("tasks")).unwrap();
        let id = s
            .task_add(
                &new("  Fix the login test ", &["passes 20 runs", "", "no sleep added"]),
                10,
            )
            .unwrap();
        let t = s.task(id).unwrap().unwrap();
        assert_eq!(t.goal, "Fix the login test");
        assert_eq!((t.owner.as_str(), t.status.as_str()), ("you", "open"));
        assert_eq!(
            t.criteria.iter().map(|c| c.text.as_str()).collect::<Vec<_>>(),
            ["passes 20 runs", "no sleep added"]
        );
        assert!(s.task_change(id, &Change::Criterion(1, true), 20).unwrap());
        assert!(
            !s.task_change(id, &Change::Criterion(5, true), 20).unwrap(),
            "no such criterion"
        );
        assert!(s.task_change(id, &Change::Status("doing".into()), 30).unwrap());
        assert!(s.task_change(id, &Change::Owner("claude".into()), 30).unwrap());
        assert!(
            s.task_change(
                id,
                &Change::Issue(Some("https://github.com/me/site/issues/4".into())),
                40
            )
            .unwrap()
        );
        assert!(s.task_change(id, &Change::Status("nope".into()), 50).is_err());
        s.task_cost(id, 0.25).unwrap();
        s.task_cost(id, 0.5).unwrap();
        let t = s.task(id).unwrap().unwrap();
        assert!(t.criteria[1].done && !t.criteria[0].done);
        assert_eq!(
            (t.status.as_str(), t.owner.as_str(), t.updated_ms),
            ("doing", "claude", 40)
        );
        assert_eq!(t.cost_usd, Some(0.75));
        assert!(s.task_add(&new("   ", &[]), 1).is_err(), "a task needs a goal");
    }

    #[test]
    fn open_tasks_come_first_newest_change_first() {
        let mut s = Store::open(&temp("tasks-order")).unwrap();
        let a = s.task_add(&new("a", &[]), 1).unwrap();
        let b = s.task_add(&new("b", &[]), 2).unwrap();
        let c = s.task_add(&new("c", &[]), 3).unwrap();
        s.task_change(b, &Change::Status("done".into()), 4).unwrap();
        s.task_change(a, &Change::Status("blocked".into()), 5).unwrap();
        let ids = |s: &Store, closed| {
            s.tasks(closed, 50)
                .unwrap()
                .iter()
                .map(|t| t.id)
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(&s, false), [a, c]);
        assert_eq!(ids(&s, true), [a, c, b]);
        assert!(s.task_delete(c).unwrap());
        assert_eq!(ids(&s, true), [a, b]);
    }
}
