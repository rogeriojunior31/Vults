//! Review decisions keep active requested changes; stale/dismissed approvals never imply readiness.
use super::{BTreeMap, BTreeSet, Credential, Error, Gate, Value, id, invalid, json, list, segment, text};

fn actor(value: &Value) -> Result<String, Error> {
    if let Some(login) = value["user"]["login"].as_str().filter(|s| segment(s)) {
        return Ok(login.into());
    }
    text(&value["team"]["name"], 255)
        .map(|name| format!("team:{name}"))
        .ok_or_else(invalid)
}
pub(super) async fn read(
    client: &reqwest::Client,
    c: &Credential,
    gate: &dyn Gate,
    epoch: u64,
    repo: &str,
    number: u64,
    pull: &Value,
) -> Result<Vec<Value>, Error> {
    let rows = list(
        client,
        c,
        gate,
        epoch,
        &format!("repos/{repo}/pulls/{number}/reviews"),
        None,
        false,
    )
    .await?
    .ok_or_else(invalid)?;
    parse(&rows, pull, super::sha(&pull["head"]["sha"])?)
}
type Rank = (u64, u32, u8, u64);
fn rank(row: &Value, decision: Option<&str>) -> Result<Rank, Error> {
    let at = text(&row["submitted_at"], 64).ok_or_else(invalid)?;
    let seconds = crate::timestamp::parse(at).ok_or_else(invalid)?;
    let nanos = if let Some(fraction) = at.get(19..).and_then(|s| s.strip_prefix('.')) {
        let digits = fraction.bytes().take_while(u8::is_ascii_digit).count();
        fraction[..digits]
            .parse::<u32>()
            .map_err(|_| invalid())?
            .checked_mul(10u32.pow(9 - digits as u32))
            .ok_or_else(invalid)?
    } else {
        0
    };
    let priority = match decision {
        None => 2,
        Some("changes") => 1,
        _ => 0,
    };
    Ok((seconds, nanos, priority, id(&row["id"])?))
}
pub(super) fn parse(rows: &[Value], pull: &Value, head: &str) -> Result<Vec<Value>, Error> {
    let mut requested = BTreeSet::new();
    for (field, team) in [
        ("requested_reviewers", false),
        ("requested_reviewers_teams", true),
    ] {
        let values = match &pull[field] {
            Value::Null => &[][..],
            Value::Array(values) if values.len() <= 100 => values.as_slice(),
            _ => return Err(invalid()),
        };
        for user in values {
            let login = if team {
                format!("team:{}", text(&user["name"], 255).ok_or_else(invalid)?)
            } else {
                user["login"]
                    .as_str()
                    .filter(|s| segment(s))
                    .ok_or_else(invalid)?
                    .into()
            };
            if !requested.insert(login) {
                return Err(invalid());
            }
        }
    }
    let mut decisive: BTreeMap<String, (Rank, Option<&str>)> = BTreeMap::new();
    let mut comments = BTreeSet::new();
    let mut ids = BTreeSet::new();
    for row in rows {
        let number = id(&row["id"])?;
        if !ids.insert(number) {
            return Err(invalid());
        }
        let login = actor(row)?;
        let stale = row["stale"].as_bool().ok_or_else(invalid)?;
        let dismissed = row["dismissed"].as_bool().ok_or_else(invalid)?;
        let state = row["state"].as_str().ok_or_else(invalid)?;
        let decision = match state {
            "APPROVED" => Some((!stale && !dismissed && row["commit_id"] == head).then_some("approved")),
            "REQUEST_CHANGES" => Some((!dismissed).then_some("changes")),
            "COMMENT" => {
                if !dismissed {
                    comments.insert(login.clone());
                }
                None
            }
            "REQUEST_REVIEW" | "PENDING" => {
                if !dismissed {
                    requested.insert(login.clone());
                }
                None
            }
            _ => return Err(invalid()),
        };
        if let Some(decision) = decision {
            let rank = rank(row, decision)?;
            if decisive.get(&login).is_none_or(|(old, _)| *old < rank) {
                decisive.insert(login, (rank, decision));
            }
        }
    }
    let mut result = BTreeMap::new();
    for login in requested {
        result.insert(login, "waiting");
    }
    for login in comments {
        result.insert(login, "commented");
    }
    for (login, (_, decision)) in decisive {
        if let Some(said) = decision {
            result.insert(login, said);
        }
    }
    Ok(result
        .into_iter()
        .map(|(login, said)| json!({"login":login,"said":said}))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pull() -> Value {
        json!({"requested_reviewers":[{"login":"bob"}],"requested_reviewers_teams":[{"name":"maintainers"}]})
    }
    fn review(id: u64, state: &str, head: &str) -> Value {
        json!({"id":id,"state":state,"commit_id":head,"stale":false,"dismissed":false,"submitted_at":format!("2026-11-10T12:00:{id:02}Z"),"user":{"login":"bob"}})
    }
    #[test]
    fn stale_dismissed_and_wrong_commit_approvals_never_become_current_approvals() {
        let head = "a".repeat(40);
        for mode in 0..3 {
            let mut row = review(1, "APPROVED", &head);
            match mode {
                0 => row["stale"] = json!(true),
                1 => row["dismissed"] = json!(true),
                _ => row["commit_id"] = json!("b".repeat(40)),
            };
            let values = parse(&[row], &pull(), &head).unwrap();
            assert!(values.iter().all(|r| r["said"] != "approved"));
        }
        let values = parse(&[review(1, "APPROVED", &head)], &pull(), &head).unwrap();
        assert_eq!(values[0]["said"], "approved");
    }
    #[test]
    fn latest_decisive_review_wins_but_comments_do_not_erase_a_decision() {
        let head = "a".repeat(40);
        let values = parse(
            &[
                review(3, "COMMENT", &head),
                review(1, "APPROVED", &head),
                review(2, "REQUEST_CHANGES", &head),
            ],
            &pull(),
            &head,
        )
        .unwrap();
        assert_eq!(values[0]["said"], "changes");
        assert_eq!(values[1]["login"], "team:maintainers");
        assert_eq!(values[1]["said"], "waiting");
        let mut dismissed = review(4, "REQUEST_CHANGES", &head);
        dismissed["dismissed"] = json!(true);
        let values = parse(&[review(1, "APPROVED", &head), dismissed], &pull(), &head).unwrap();
        assert!(
            values
                .iter()
                .all(|v| v["said"] != "approved" && v["said"] != "changes")
        );
    }
    #[test]
    fn unknown_states_and_duplicate_review_ids_fail_closed() {
        let head = "a".repeat(40);
        let row = review(1, "APPROVED", &head);
        assert!(parse(&[row.clone(), row], &pull(), &head).is_err());
        assert!(parse(&[review(1, "FUTURE_STATE", &head)], &pull(), &head).is_err());
    }
    #[test]
    fn submission_time_beats_creation_id_and_ambiguous_ties_do_not_grant_approval() {
        let head = "a".repeat(40);
        let approved = review(2, "APPROVED", &head);
        let mut late = review(1, "REQUEST_CHANGES", &head);
        late["submitted_at"] = json!("2026-11-10T12:00:03Z");
        assert_eq!(
            parse(&[late.clone(), approved.clone()], &pull(), &head).unwrap()[0]["said"],
            "changes"
        );
        late["submitted_at"] = approved["submitted_at"].clone();
        assert_eq!(
            parse(&[late, approved], &pull(), &head).unwrap()[0]["said"],
            "changes"
        );
    }
}
