//! An explicit GitHub grant belongs to one built-in profile, repository and signed-in account.
use crate::flock_tools::{GithubSource, Source};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use vults_agents::driver::BoxFuture;
use vults_chat::tools::{Tool, ToolApproval, Tools};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Grant {
    pub version: u64,
    pub enabled: bool,
    pub repository: String,
    pub host: String,
    pub account: String,
}
pub(crate) fn grant_active(settings: &crate::settings::Settings, profile: &str, grant: &Grant) -> bool {
    valid_profile(profile) && grant.enabled && settings.specialist_github.get(profile) == Some(grant)
}
fn check_input(profile: &str, current: &Grant, grant: &Grant, input: &Value) -> Result<(), String> {
    if !valid_profile(profile) || !grant.enabled || current != grant {
        return Err("GitHub access for this profile is disabled or has changed.".into());
    }
    if input["repository"].as_str() != Some(grant.repository.as_str()) {
        return Err("This GitHub call is outside the profile's selected repository.".into());
    }
    Ok(())
}
fn identity_matches(grant: &Grant, identity: &Value) -> bool {
    identity["source"]["account"].as_str() == Some(grant.account.as_str())
        && identity["source"]["host"].as_str() == Some(grant.host.as_str())
}
fn tool_allowed(name: &str) -> bool {
    name.starts_with("github_")
        && !matches!(
            name,
            "github_snapshot" | "github_search_repositories" | "github_account"
        )
}
fn scoped_schema(mut schema: Value, grant: &Grant) -> Value {
    schema["properties"]["repository"]["const"] = serde_json::json!(grant.repository);
    schema
}
pub(crate) fn current(app: &AppHandle, profile: &str) -> Grant {
    app.state::<crate::settings::SettingsState>()
        .0
        .lock()
        .ok()
        .and_then(|s| s.specialist_github.get(profile).cloned())
        .unwrap_or_default()
}
fn valid_profile(profile: &str) -> bool {
    matches!(profile, "developer" | "reviewer")
}
fn valid_repository(repository: &str) -> bool {
    let parts: Vec<_> = repository.split('/').collect();
    parts.len() == 2
        && parts.iter().all(|part| {
            !part.is_empty()
                && *part != "."
                && *part != ".."
                && part.len() <= 100
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        })
}
async fn account(app: &AppHandle) -> Result<Value, String> {
    let request = crate::gate::connector_request(app, "github")
        .ok_or("Enable the GitHub connector and check its request budget first.")?;
    let result = vults_connectors::github::account().await;
    request.finish(result.is_ok());
    result.map_err(|e| e.to_string())
}
#[tauri::command]
pub(crate) async fn set_specialist_github(
    app: AppHandle,
    window: tauri::Webview,
    profile: String,
    version: u64,
    enabled: bool,
    repository: String,
) -> Result<Grant, String> {
    if window.label() != crate::NEST || !valid_profile(&profile) {
        return Err("GitHub profile access is available for Developer and Reviewer in the Nest only.".into());
    }
    if current(&app, &profile).version != version {
        return Err("This GitHub grant changed. Reload the profile and try again.".into());
    }
    let mut next = current(&app, &profile);
    if enabled {
        if !valid_repository(&repository) {
            return Err("Enter a repository as owner/name.".into());
        }
        let identity = account(&app).await?;
        next.repository = repository;
        next.account = identity["source"]["account"]
            .as_str()
            .ok_or("GitHub did not identify its account.")?
            .into();
        next.host = identity["source"]["host"]
            .as_str()
            .ok_or("GitHub did not identify its host.")?
            .into();
    }
    next.enabled = enabled;
    next.version = version
        .checked_add(1)
        .ok_or("This GitHub grant cannot be updated.")?;
    let state = app.state::<crate::settings::SettingsState>();
    let mut settings = state.0.lock().map_err(|_| "Settings are busy.")?;
    if settings.specialist_github.get(&profile).map_or(0, |g| g.version) != version {
        return Err("This GitHub grant changed. Reload the profile and try again.".into());
    }
    let mut saved = settings.clone();
    saved.specialist_github.insert(profile.clone(), next.clone());
    crate::settings::save(&saved)
        .map_err(|_| "Could not save GitHub access. Your previous choice stays active.")?;
    *settings = saved;
    crate::audit::keep(vults_core::audit::Audit::new(
        vults_core::audit::Actor::Human,
        if enabled {
            vults_core::audit::Act::Allow
        } else {
            vults_core::audit::Act::Deny
        },
        &profile,
        &next.repository,
        "specialist_github",
        &serde_json::json!(next).to_string(),
    ));
    Ok(next)
}

impl GithubSource {
    async fn authorize(&self, input: &Value) -> Result<(), String> {
        let Some((profile, grant)) = &self.grant else {
            return Ok(());
        };
        check_input(profile, &current(&self.app, profile), grant, input)?;
        if !self.github_work_item_allowed() {
            return Err("GitHub access for this profile is disabled or has changed.".into());
        }
        let archived = vults_store::Store::open(&crate::history::dir())
            .and_then(|s| s.agent_profile_state(profile))
            .map_err(|_| "Could not check the profile's GitHub access.")?
            .archived;
        if archived {
            return Err("This agent is archived. Restore it before using GitHub.".into());
        }
        let identity = account(&self.app).await?;
        if !identity_matches(grant, &identity) || current(&self.app, profile) != *grant {
            return Err(
                "The GitHub account or profile grant changed. Review the profile's GitHub access again."
                    .into(),
            );
        }
        Ok(())
    }
}
struct ScopedTool {
    tool: Arc<dyn Tool>,
    source: GithubSource,
}
impl std::fmt::Debug for ScopedTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScopedGithubTool")
            .field("name", &self.tool.name())
            .finish()
    }
}
impl Tool for ScopedTool {
    fn name(&self) -> &str {
        self.tool.name()
    }
    fn description(&self) -> &str {
        self.tool.description()
    }
    fn schema(&self) -> Value {
        self.source.grant.as_ref().map_or_else(
            || self.tool.schema(),
            |(_, grant)| scoped_schema(self.tool.schema(), grant),
        )
    }
    fn reads_outside(&self) -> bool {
        true
    }
    fn call<'a>(&'a self, input: &'a Value) -> BoxFuture<'a, Result<String, String>> {
        Box::pin(async move {
            self.source.authorize(input).await?;
            let result = self.tool.call(input).await?;
            self.source.authorize(input).await?;
            Ok(result)
        })
    }
    fn approval<'a>(&'a self, input: &'a Value) -> Option<BoxFuture<'a, Result<ToolApproval, String>>> {
        self.tool.approval(input).map(|prepare| {
            Box::pin(async move {
                self.source.authorize(input).await?;
                let prepared = prepare.await?;
                self.source.authorize(input).await?;
                Ok(prepared)
            }) as BoxFuture<'a, _>
        })
    }
    fn call_approved<'a>(
        &'a self,
        input: &'a Value,
        context: &'a Value,
    ) -> BoxFuture<'a, Result<String, String>> {
        Box::pin(async move {
            self.source.authorize(input).await?;
            let result = self.tool.call_approved(input, context).await?;
            self.source.authorize(input).await?;
            Ok(result)
        })
    }
}
pub(crate) fn tools(app: &AppHandle, profile: &str) -> Tools {
    let grant = current(app, profile);
    if !valid_profile(profile) || !grant.enabled {
        return Vec::new();
    }
    let source = GithubSource::specialist(app.clone(), profile.into(), grant);
    crate::flock_tools::all(Arc::new(source.clone()))
        .into_iter()
        .filter(|tool| tool_allowed(tool.name()))
        .map(|tool| {
            Arc::new(ScopedTool {
                tool,
                source: source.clone(),
            }) as Arc<dyn Tool>
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn grants_are_independent_and_cannot_be_revived_or_used_for_another_repository_or_account() {
        let mut settings = crate::settings::Settings {
            zeca: false,
            ..Default::default()
        };
        let grant = Grant {
            version: 1,
            enabled: true,
            repository: "org/repo".into(),
            host: "github.com".into(),
            account: "maintainer".into(),
        };
        settings
            .specialist_github
            .insert("developer".into(), grant.clone());
        assert!(grant_active(&settings, "developer", &grant));
        assert!(!grant_active(&settings, "reviewer", &grant));
        assert!(!grant_active(&settings, "researcher", &grant));
        assert!(check_input("developer", &grant, &grant, &json!({"repository":"org/repo"})).is_ok());
        assert!(check_input("developer", &grant, &grant, &json!({"repository":"org/other"})).is_err());
        let mut next = grant.clone();
        next.enabled = false;
        assert!(check_input("developer", &next, &grant, &json!({"repository":"org/repo"})).is_err());
        next.enabled = true;
        next.version += 1;
        assert!(check_input("developer", &next, &grant, &json!({"repository":"org/repo"})).is_err());
        assert!(identity_matches(
            &grant,
            &json!({"source":{"account":"maintainer","host":"github.com"}})
        ));
        assert!(!identity_matches(
            &grant,
            &json!({"source":{"account":"another","host":"github.com"}})
        ));
        assert!(!identity_matches(
            &grant,
            &json!({"source":{"account":"maintainer","host":"other.example"}})
        ));
        assert_eq!(
            scoped_schema(json!({"type":"object"}), &grant)["properties"]["repository"]["const"],
            "org/repo"
        );
        for name in [
            "github_repository",
            "github_issue_close",
            "github_pull_request_merge",
        ] {
            assert!(tool_allowed(name));
        }
        for name in [
            "github_snapshot",
            "github_account",
            "github_search_repositories",
            "recall",
            "memory_propose",
            "flock_list",
        ] {
            assert!(!tool_allowed(name));
        }
        assert!(valid_repository("owner/repository"));
        for invalid in [
            "../repo",
            "owner/../repo",
            "https://github.com/owner/repo",
            "owner/repo?x",
            "owner/",
        ] {
            assert!(!valid_repository(invalid));
        }
    }
}
