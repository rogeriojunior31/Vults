//! Complete read-only boards; every live read still uses the parent credential-bound gate.
use super::{Connection, Credential, EPOCH, Gate, MAX_PAGES, get, invalid, request};
use crate::{
    Checks, Diff, Error, Event, Facts, Group, Level, Release, Review, Reviewer, Row, Run, RunState, Said,
    Snapshot, Story, StoryKind,
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::atomic::Ordering,
};
use vults_core::SafeUrl;

#[path = "checks.rs"]
mod checks;
#[path = "reviews.rs"]
mod reviews;

pub(super) fn text(v: &Value, limit: usize) -> Option<&str> {
    v.as_str()
        .filter(|s| !s.is_empty() && s.len() <= limit && !s.chars().any(char::is_control))
}
pub(super) fn segment(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 255
        && !matches!(s, "." | "..")
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.'))
}
fn repo(v: &Value) -> Result<&str, Error> {
    text(v, 511)
        .filter(|s| s.split('/').count() == 2 && s.split('/').all(segment))
        .ok_or_else(invalid)
}
pub(super) fn sha(v: &Value) -> Result<&str, Error> {
    v.as_str()
        .filter(|s| matches!(s.len(), 40 | 64) && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or_else(invalid)
}
pub(super) fn id(v: &Value) -> Result<u64, Error> {
    v.as_u64()
        .filter(|n| *n > 0 && *n <= i64::MAX as u64)
        .ok_or_else(invalid)
}
pub(super) fn encoded(s: &str) -> String {
    s.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b'~') {
                char::from(b).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

/// Lists requiring complete coverage. Windowed project/release reads are deliberately separate.
pub(super) async fn list(
    client: &reqwest::Client,
    c: &Credential,
    gate: &dyn Gate,
    epoch: u64,
    path: &str,
    field: Option<&str>,
    optional: bool,
) -> Result<Option<Vec<Value>>, Error> {
    let mut result = Vec::new();
    let mut expected = None;
    for page in 1..=MAX_PAGES {
        let join = if path.contains('?') { '&' } else { '?' };
        let Some((v, header)) = request(
            client,
            c,
            gate,
            epoch,
            &format!("{path}{join}limit=100&page={page}"),
            optional,
        )
        .await?
        else {
            return Ok(None);
        };
        let (rows, total) = if let Some(field) = field {
            (
                v[field].as_array().ok_or_else(invalid)?,
                v["total_count"]
                    .as_u64()
                    .and_then(|n| usize::try_from(n).ok())
                    .ok_or_else(invalid)?,
            )
        } else {
            (v.as_array().ok_or_else(invalid)?, header.ok_or_else(invalid)?)
        };
        if total > 500 || rows.len() > 100 || expected.is_some_and(|n| n != total) {
            return Err(invalid());
        }
        expected = Some(total);
        result.extend(rows.iter().cloned());
        if result.len() > total || (rows.is_empty() && result.len() != total) {
            return Err(invalid());
        }
        if result.len() == total {
            return Ok(Some(result));
        }
    }
    Err(invalid())
}

pub(super) async fn read(
    client: &reqwest::Client,
    c: &Credential,
    gate: &dyn Gate,
    epoch: u64,
) -> Result<Snapshot, Error> {
    let (user, _) = get(client, c, gate, epoch, "user").await?;
    let login = user["login"]
        .as_str()
        .filter(|s| segment(s))
        .ok_or_else(invalid)?;
    let user_id = id(&user["id"])?;
    let namespace = format!("{login}@{user_id}");
    let mut snapshot = Snapshot::from([(
        "meta".into(),
        json!({"login":login,"id":user_id,"base":c.connection.base_url,"generation":c.connection.generation,"schema":2}),
    )]);
    for (group, filter) in [("yours", "created=true"), ("review", "review_requested=true")] {
        let rows = list(
            client,
            c,
            gate,
            epoch,
            &format!("repos/issues/search?state=open&type=pulls&{filter}"),
            None,
            false,
        )
        .await?
        .ok_or_else(invalid)?;
        for row in rows {
            insert(&mut snapshot, &row, group, &namespace, &c.connection)?;
        }
    }
    let targets: Vec<_> = snapshot
        .iter()
        .filter_map(|(key, v)| Some((key.clone(), v["repo"].as_str()?.to_owned(), v["number"].as_u64()?)))
        .collect();
    let mut cache = BTreeMap::new();
    for (key, repo, number) in targets {
        let target = (repo.clone(), number);
        if !cache.contains_key(&target) {
            cache.insert(target.clone(), pull(client, c, gate, epoch, &repo, number).await?);
        }
        let enriched = cache.get(&target).ok_or_else(invalid)?;
        let row = snapshot
            .get_mut(&key)
            .and_then(Value::as_object_mut)
            .ok_or_else(invalid)?;
        for (k, v) in enriched.as_object().ok_or_else(invalid)? {
            row.insert(k.clone(), v.clone());
        }
    }
    projects(&mut snapshot, client, c, gate, epoch, &namespace, user_id).await?;
    if EPOCH.load(Ordering::SeqCst) != epoch {
        return Err(Error::Budget);
    }
    Ok(snapshot)
}

pub(super) fn insert(
    snapshot: &mut Snapshot,
    v: &Value,
    group: &str,
    login: &str,
    c: &Connection,
) -> Result<(), Error> {
    let repo = repo(&v["repository"]["full_name"])?;
    let number = id(&v["number"])?;
    if number > u32::MAX as u64 || v["state"] != "open" || !v["pull_request"].is_object() {
        return Err(invalid());
    }
    let title = text(&v["title"], 4096).ok_or_else(invalid)?;
    let url = format!("{}{repo}/pulls/{number}", c.base_url);
    if SafeUrl::for_service(&url, &c.base_url).is_none() {
        return Err(invalid());
    }
    let key = format!("gitea:{}:{login}:{group}:{repo}#{number}:item", c.generation);
    let rank = snapshot.len();
    if snapshot.insert(key,json!({"group":group,"repo":repo,"number":number,"title":title,"url":url,"author":v["user"]["login"].as_str().filter(|s|segment(s)),"rank":rank})).is_some(){return Err(invalid());}
    Ok(())
}

async fn pull(
    client: &reqwest::Client,
    c: &Credential,
    gate: &dyn Gate,
    epoch: u64,
    repo: &str,
    number: u64,
) -> Result<Value, Error> {
    let (v, _) = get(client, c, gate, epoch, &format!("repos/{repo}/pulls/{number}")).await?;
    if v["number"] != number
        || v["state"] != "open"
        || v["merged"] != false
        || v["base"]["repo"]["full_name"] != repo
    {
        return Err(invalid());
    }
    let head = sha(&v["head"]["sha"])?;
    let (reviewers, (ci, runs)) = tokio::try_join!(
        reviews::read(client, c, gate, epoch, repo, number, &v),
        checks::read(
            client,
            c,
            gate,
            epoch,
            repo,
            head,
            v["base"]["repo"]["has_actions"].as_bool()
        )
    )?;
    let size = match (&v["additions"], &v["deletions"], &v["changed_files"]) {
        (Value::Null, Value::Null, Value::Null) => Value::Null,
        (a, d, f) => {
            json!({"added":u32::try_from(a.as_u64().ok_or_else(invalid)?).map_err(|_|invalid())?,"removed":u32::try_from(d.as_u64().ok_or_else(invalid)?).map_err(|_|invalid())?,"files":u32::try_from(f.as_u64().ok_or_else(invalid)?).map_err(|_|invalid())?})
        }
    };
    let review = if reviewers.iter().any(|r| r["said"] == "changes") {
        Some("changes")
    } else if reviewers.iter().any(|r| r["said"] == "approved") {
        Some("approved")
    } else {
        None
    };
    Ok(
        json!({"oid":head,"title":text(&v["title"],4096).ok_or_else(invalid)?,"branch":text(&v["head"]["ref"],1024),"author":v["user"]["login"].as_str().filter(|s|segment(s)),"size":size,"reviewers":reviewers,"review":review,"ci":ci,"runs":runs,"updated":v["updated_at"]}),
    )
}

async fn projects(
    snapshot: &mut Snapshot,
    client: &reqwest::Client,
    c: &Credential,
    gate: &dyn Gate,
    epoch: u64,
    login: &str,
    user_id: u64,
) -> Result<(), Error> {
    let (v, _) = get(
        client,
        c,
        gate,
        epoch,
        &format!("repos/search?uid={user_id}&sort=updated&order=desc&archived=false&limit=10&page=1"),
    )
    .await?;
    if v["ok"] != true {
        return Err(invalid());
    }
    let repos = v["data"]
        .as_array()
        .filter(|r| r.len() <= 10)
        .ok_or_else(invalid)?;
    let mut seen = BTreeSet::new();
    for (rank, p) in repos.iter().enumerate() {
        let name = repo(&p["full_name"])?;
        if !seen.insert(name) {
            return Err(invalid());
        }
        if p["archived"].as_bool().ok_or_else(invalid)? {
            continue;
        }
        if !p["empty"].as_bool().ok_or_else(invalid)? {
            let branch = text(&p["default_branch"], 1024).ok_or_else(invalid)?;
            let (b, _) = get(
                client,
                c,
                gate,
                epoch,
                &format!("repos/{name}/branches/{}", encoded(branch)),
            )
            .await?;
            if b["name"] != branch {
                return Err(invalid());
            }
            let head = sha(&b["commit"]["id"])?;
            let (ci, runs) =
                checks::read(client, c, gate, epoch, name, head, p["has_actions"].as_bool()).await?;
            if ci.is_some() || !runs.is_empty() {
                let title = b["commit"]["message"]
                    .as_str()
                    .and_then(|s| s.lines().next())
                    .filter(|s| !s.is_empty() && s.len() <= 4096)
                    .unwrap_or(head);
                snapshot.insert(format!("gitea:{}:{login}:branch:{name}:item",c.connection.generation),json!({"group":"branch","repo":name,"branch":branch,"oid":head,"title":title,"ci":ci,"runs":runs,"updated":b["commit"]["timestamp"],"url":format!("{}{name}/commit/{head}",c.connection.base_url),"rank":rank}));
            }
        }
        if p["has_releases"] == false {
            continue;
        }
        let (releases, _) = get(
            client,
            c,
            gate,
            epoch,
            &format!("repos/{name}/releases?limit=1&page=1"),
        )
        .await?;
        let rows = releases.as_array().filter(|r| r.len() <= 1).ok_or_else(invalid)?;
        if let Some(r) = rows.first() {
            let tag = text(&r["tag_name"], 255).ok_or_else(invalid)?;
            let draft = r["draft"].as_bool().ok_or_else(invalid)?;
            let prerelease = r["prerelease"].as_bool().ok_or_else(invalid)?;
            let mut url = format!("{}{name}/releases/tag/{}", c.connection.base_url, encoded(tag));
            if SafeUrl::for_service(&url, &c.connection.base_url).is_none() {
                url = format!("{}{name}/releases", c.connection.base_url);
            }
            snapshot.insert(format!("gitea:{}:{login}:release:{name}:item",c.connection.generation),json!({"group":"release","repo":name,"title":text(&r["name"],4096).unwrap_or(tag),"tag":tag,"draft":draft,"prerelease":prerelease,"url":url,"updated":r["published_at"].as_str().or(r["created_at"].as_str()),"rank":rank}));
        }
    }
    Ok(())
}

pub(super) fn board(snapshot: &Snapshot) -> Vec<Row> {
    let mut rows: Vec<_> = snapshot
        .iter()
        .filter_map(|(key, v)| {
            let kind = v["group"].as_str()?;
            let group = match kind {
                "yours" => Group::Yours,
                "review" => Group::ToReview,
                "branch" => Group::Branches,
                "release" => Group::Releases,
                _ => return None,
            };
            let repo = v["repo"].as_str()?;
            let short = repo.rsplit('/').next()?;
            let name = if matches!(kind, "yours" | "review") {
                format!("{short}#{}", v["number"].as_u64()?)
            } else {
                short.to_owned()
            };
            let size = &v["size"];
            Some((
                group,
                v["rank"].as_u64().unwrap_or(u64::MAX),
                Row {
                    item: key.clone(),
                    group,
                    name,
                    title: v["title"].as_str()?.into(),
                    checks: match v["ci"].as_str() {
                        Some("passing") => Some(Checks::Passing),
                        Some("failing") => Some(Checks::Failing),
                        Some("running") => Some(Checks::Running),
                        _ => None,
                    },
                    review: match v["review"].as_str() {
                        Some("approved") => Some(Review::Approved),
                        Some("changes") => Some(Review::Changes),
                        _ => None,
                    },
                    url: v["url"].as_str().map(str::to_owned),
                    head: None,
                    facts: Facts {
                        repo: Some(repo.into()),
                        branch: v["branch"].as_str().map(str::to_owned),
                        author: v["author"].as_str().map(str::to_owned),
                        updated: v["updated"].as_str().and_then(crate::timestamp::parse),
                        diff: (|| {
                            Some(Diff {
                                added: size["added"].as_u64()?.try_into().ok()?,
                                removed: size["removed"].as_u64()?.try_into().ok()?,
                                files: size["files"].as_u64()?.try_into().ok()?,
                            })
                        })(),
                        reviewers: v["reviewers"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(|r| {
                                Some(Reviewer {
                                    login: r["login"].as_str()?.into(),
                                    said: match r["said"].as_str()? {
                                        "approved" => Said::Approved,
                                        "changes" => Said::Changes,
                                        "commented" => Said::Commented,
                                        "waiting" => Said::Waiting,
                                        _ => return None,
                                    },
                                })
                            })
                            .collect(),
                        runs: v["runs"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(|r| {
                                Some(Run {
                                    name: r["name"].as_str()?.into(),
                                    state: match r["state"].as_str()? {
                                        "passed" => RunState::Passed,
                                        "failed" => RunState::Failed,
                                        "running" => RunState::Running,
                                        "waiting" => RunState::Waiting,
                                        "skipped" => RunState::Skipped,
                                        _ => return None,
                                    },
                                })
                            })
                            .collect(),
                        release: (kind == "release").then_some(if v["draft"] == true {
                            Release::Draft
                        } else if v["prerelease"] == true {
                            Release::Prerelease
                        } else {
                            Release::Latest
                        }),
                        ..Facts::default()
                    },
                },
            ))
        })
        .collect();
    rows.sort_by_key(|(group, rank, _)| (*group, *rank));
    rows.into_iter().map(|(_, _, r)| r).collect()
}

pub(super) fn diff(before: &Snapshot, after: &Snapshot) -> Vec<Event> {
    let mut events = Vec::new();
    for (key, v) in after {
        let Some(group) = v["group"].as_str() else {
            continue;
        };
        if group == "release" {
            continue;
        }
        let previous = before.get(key);
        let repo = v["repo"].as_str().unwrap_or_default();
        let name = if group == "branch" {
            repo.to_owned()
        } else {
            format!("{repo}#{}", v["number"])
        };
        let mut push = |kind, news: &str, topic: Option<&str>, level, label: &str| {
            events.push(Event {
                connector: "gitea".into(),
                key: format!("{key}:{news}"),
                topic: topic.map(|t| format!("{key}:{t}")),
                level,
                title: format!("{label} · {name}"),
                detail: v["title"].as_str().unwrap_or_default().into(),
                url: v["url"].as_str().map(str::to_owned),
                story: Some(Story {
                    kind,
                    name: name.clone(),
                    branch: (group == "branch").then(|| v["branch"].as_str().unwrap_or_default().into()),
                }),
            })
        };
        if group == "review" && previous.is_none() {
            push(
                StoryKind::ReviewRequested,
                "requested",
                None,
                Level::Info,
                "Review requested",
            );
        }
        if !matches!(group, "yours" | "branch") {
            continue;
        }
        if previous.is_none_or(|old| old["oid"] != v["oid"] || old["ci"] != v["ci"]) {
            let sha = v["oid"].as_str().unwrap_or_default();
            match v["ci"].as_str() {
                Some("failing") => push(
                    StoryKind::ChecksFailed,
                    &format!("ci-failed:{sha}"),
                    Some("ci"),
                    Level::Error,
                    "Checks failed",
                ),
                Some("passing") => push(
                    StoryKind::ChecksPassed,
                    &format!("ci-passed:{sha}"),
                    Some("ci"),
                    Level::Ok,
                    "Checks passed",
                ),
                _ => {}
            }
        }
        if v["review"] == "changes" && previous.is_none_or(|p| p["review"] != "changes") {
            push(
                StoryKind::ChangesRequested,
                "changes",
                Some("review"),
                Level::Warn,
                "Changes requested",
            );
        }
        if v["review"] == "approved" && previous.is_none_or(|p| p["review"] != "approved") {
            push(
                StoryKind::Approved,
                "approved",
                Some("review"),
                Level::Ok,
                "Approved",
            );
        }
    }
    events
}

#[cfg(test)]
#[path = "dashboard_tests.rs"]
mod tests;
