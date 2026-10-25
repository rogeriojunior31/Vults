//! The Nest owns local profiles; saving instructions never starts an agent or changes access.
use crate::settings;
use serde::{Deserialize, Serialize};
use vults_store::{
    Store,
    profiles::{ExecutionDefaults, Profile, ProfileState, SkillSnapshot, StateChange},
};

#[derive(Serialize)]
pub struct View {
    id: String,
    revision: u32,
    title: String,
    about: String,
    species: String,
    look: String,
    instructions: String,
    built_in: bool,
    skills: Vec<SkillSnapshot>,
    skill_catalog: Vec<vults_zeca::profiles::Skill>,
    access: crate::settings::ChatAccess,
    state: ProfileState,
    github: crate::specialist_github::Grant,
    providers: Vec<vults_chat::providers::Provider>,
}

impl From<Profile> for View {
    fn from(p: Profile) -> Self {
        Self {
            id: format!("custom-{}", p.id),
            revision: p.revision,
            title: p.title,
            about: p.about,
            species: p.species,
            look: p.look,
            instructions: p.instructions,
            built_in: false,
            skills: p.skills,
            skill_catalog: vults_zeca::profiles::SKILLS.to_vec(),
            access: crate::settings::ChatAccess::Ask,
            state: ProfileState::default(),
            github: Default::default(),
            providers: vults_chat::providers::PROVIDERS.to_vec(),
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "camelCase", deny_unknown_fields)]
pub enum Edit {
    Create {
        title: String,
        instructions: String,
        species: String,
        look: String,
        skill_ids: Vec<String>,
    },
    Copy {
        preset: String,
        title: String,
        instructions: String,
        species: String,
        look: String,
        skill_ids: Vec<String>,
    },
    Update {
        id: i64,
        revision: u32,
        title: String,
        instructions: String,
        species: String,
        look: String,
        skill_ids: Vec<String>,
    },
}

fn page(window: &tauri::Webview) -> Result<(), String> {
    if window.label() != crate::NEST {
        return Err("Agent profiles are available in the Nest only.".into());
    }
    Ok(())
}

fn skills(ids: Vec<String>) -> Result<Vec<SkillSnapshot>, String> {
    if ids.len() > 8 {
        return Err("invalid".into());
    }
    let mut result = Vec::with_capacity(ids.len());
    for id in ids {
        let skill = vults_zeca::profiles::SKILLS
            .iter()
            .find(|skill| skill.id == id)
            .ok_or("invalid")?;
        if result.iter().any(|saved: &SkillSnapshot| saved.id == skill.id) {
            return Err("invalid".into());
        }
        result.push(SkillSnapshot {
            id: skill.id.into(),
            revision: skill.revision,
            title: skill.title.into(),
            content: skill.content.into(),
        });
    }
    Ok(result)
}
fn error(e: vults_store::Error) -> String {
    match e {
        vults_store::Error::Invalid("Profile changed. Reload before saving.") => "conflict",
        vults_store::Error::Invalid("Profile limit reached.") => "limit",
        vults_store::Error::Invalid("Profile is archived.") => "archived",
        vults_store::Error::Invalid("Invalid profile appearance.") => "appearance",
        vults_store::Error::Invalid(_) => "invalid",
        _ => "storage",
    }
    .into()
}

#[tauri::command]
pub async fn agent_profiles(app: tauri::AppHandle, window: tauri::Webview) -> Result<Vec<View>, String> {
    page(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        let app_for_access = app.clone();
        let store = Store::open(&crate::history::dir()).map_err(error)?;
        let mut profiles: Vec<View> = vults_zeca::profiles::PRESETS
            .iter()
            .map(|p| View {
                id: p.id.into(),
                revision: p.revision,
                title: p.title.into(),
                about: p.about.into(),
                species: p.species.into(),
                look: "none".into(),
                instructions: p.instructions.into(),
                built_in: true,
                skills: Vec::new(),
                skill_catalog: vults_zeca::profiles::SKILLS.to_vec(),
                access: crate::settings::specialist_chat_access(&app_for_access, p.id),
                state: ProfileState::default(),
                github: crate::specialist_github::current(&app_for_access, p.id),
                providers: vults_chat::providers::PROVIDERS.to_vec(),
            })
            .collect();
        profiles.extend(store.agent_profiles().map_err(error)?.into_iter().map(|p| {
            let id = format!("custom-{}", p.id);
            let mut view = View::from(p);
            view.access = crate::settings::specialist_chat_access(&app_for_access, &id);
            view
        }));
        for profile in &mut profiles {
            profile.state = store.agent_profile_state(&profile.id).map_err(error)?;
            profile.github = crate::specialist_github::current(&app_for_access, &profile.id);
        }
        Ok(profiles)
    })
    .await
    .map_err(|_| "storage".to_owned())?
}

#[tauri::command]
pub async fn set_specialist_chat_access(
    app: tauri::AppHandle,
    window: tauri::Webview,
    profile: String,
    access: crate::settings::ChatAccess,
) -> Result<(), String> {
    page(&window)?;
    let preset = vults_zeca::profiles::PRESETS.iter().any(|p| p.id == profile);
    let valid = if let Some(id) = profile
        .strip_prefix("custom-")
        .and_then(|id| id.parse::<i64>().ok())
        .filter(|id| *id > 0 && format!("custom-{id}") == profile)
    {
        let dir = crate::history::dir();
        tauri::async_runtime::spawn_blocking(move || {
            Store::open(&dir)
                .and_then(|store| store.agent_profiles())
                .map(|profiles| profiles.iter().any(|p| p.id == id))
                .unwrap_or(false)
        })
        .await
        .map_err(|_| "storage".to_owned())?
    } else {
        preset
    };
    if !valid {
        return Err("invalid".into());
    }
    settings::save_specialist_access(&app, profile, access)
}

#[tauri::command]
pub async fn agent_profile_save(
    app: tauri::AppHandle,
    window: tauri::Webview,
    edit: Edit,
    preferences: Option<PreferenceEdit>,
) -> Result<View, String> {
    page(&window)?;
    if let Some(p) = &preferences {
        validate_defaults(&p.defaults)?;
    }
    tauri::async_runtime::spawn_blocking(move || {
        let mut store = Store::open(&crate::history::dir()).map_err(error)?;
        let profile = match edit {
            Edit::Create {
                title,
                instructions,
                species,
                look,
                skill_ids,
            } => Profile {
                id: 0,
                revision: 0,
                title,
                instructions,
                preset: String::new(),
                preset_revision: 0,
                species,
                look,
                about: "Custom agent profile".into(),
                skills: skills(skill_ids)?,
            },
            Edit::Copy {
                preset,
                title,
                instructions,
                species,
                look,
                skill_ids,
            } => {
                let p = vults_zeca::profiles::PRESETS
                    .iter()
                    .find(|p| p.id == preset)
                    .ok_or("invalid")?;
                Profile {
                    id: 0,
                    revision: 0,
                    title,
                    instructions,
                    preset: p.id.into(),
                    preset_revision: p.revision,
                    species,
                    look,
                    about: p.about.into(),
                    skills: skills(skill_ids)?,
                }
            }
            Edit::Update {
                id,
                revision,
                title,
                instructions,
                species,
                look,
                skill_ids,
            } => {
                let mut p = store
                    .agent_profiles()
                    .map_err(error)?
                    .into_iter()
                    .find(|p| p.id == id)
                    .ok_or("invalid")?;
                p.revision = revision;
                p.title = title;
                p.instructions = instructions;
                p.species = species;
                p.look = look;
                p.skills = skills(skill_ids)?;
                p
            }
        };
        if crate::chat::specialist_brief(&vults_store::profiles::Snapshot::from(profile.clone())).len()
            > 32768
        {
            return Err("brief".into());
        }
        let saved = match &preferences {
            Some(p) => store.agent_profile_save_with_defaults(&profile, p.version, &p.defaults),
            None => store.agent_profile_save(&profile),
        }
        .map_err(error)?;
        let mut view = View::from(saved);
        view.state = store.agent_profile_state(&view.id).map_err(error)?;
        view.access = settings::specialist_chat_access(&app, &view.id);
        Ok(view)
    })
    .await
    .map_err(|_| "storage".to_owned())?
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreferenceEdit {
    pub version: u32,
    pub defaults: ExecutionDefaults,
}
pub(crate) fn validate_defaults(defaults: &ExecutionDefaults) -> Result<(), String> {
    defaults.validate().map_err(error)?;
    let model = match defaults {
        ExecutionDefaults::Claude { model, effort } | ExecutionDefaults::Codex { model, effort } => {
            if effort
                .as_deref()
                .is_some_and(|e| vults_chat::Effort::parse(e).is_none())
            {
                return Err("invalid".into());
            }
            model
        }
        ExecutionDefaults::Api { provider, model } => {
            if vults_chat::providers::find(provider).is_none() {
                return Err("invalid".into());
            }
            model
        }
    };
    if model.as_deref().is_some_and(|m| !vults_chat::valid_model(m)) {
        return Err("invalid".into());
    }
    Ok(())
}
#[derive(Serialize, Clone)]
struct StateEvent {
    profile: String,
    state: ProfileState,
}
#[tauri::command]
pub async fn agent_profile_state_set(
    app: tauri::AppHandle,
    window: tauri::Webview,
    profile: String,
    version: u32,
    change: StateChange,
) -> Result<ProfileState, String> {
    page(&window)?;
    if let StateChange::Defaults { defaults } = &change {
        validate_defaults(defaults)?;
    }
    let key = profile.clone();
    let state = tauri::async_runtime::spawn_blocking(move || {
        Store::open(&crate::history::dir())
            .and_then(|mut s| s.agent_profile_state_set(&key, version, &change))
            .map_err(error)
    })
    .await
    .map_err(|_| "storage".to_owned())??;
    use tauri::Emitter;
    let _ = app.emit_to(
        crate::NEST,
        "agent-profile-state",
        StateEvent {
            profile,
            state: state.clone(),
        },
    );
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_use_the_chat_provider_registry_without_credentials_or_network() {
        for provider in vults_chat::providers::PROVIDERS {
            assert!(
                validate_defaults(&ExecutionDefaults::Api {
                    provider: provider.id.into(),
                    model: Some("test-model".into()),
                })
                .is_ok()
            );
        }
        for defaults in [
            ExecutionDefaults::Api {
                provider: "unknown".into(),
                model: None,
            },
            ExecutionDefaults::Claude {
                model: Some("--flag".into()),
                effort: None,
            },
            ExecutionDefaults::Codex {
                model: None,
                effort: Some("unknown".into()),
            },
        ] {
            assert!(validate_defaults(&defaults).is_err());
        }
        assert!(
            serde_json::from_str::<PreferenceEdit>(
                r#"{"version":0,"defaults":{"executor":"claude","model":null,"effort":null},"access":"full"}"#
            )
            .is_err()
        );
    }
}
