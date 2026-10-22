//! The Nest owns local profiles; saving instructions never starts an agent or changes access.
use crate::settings;
use serde::{Deserialize, Serialize};
use vults_store::{
    Store,
    profiles::{Profile, SkillSnapshot},
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
            })
            .collect();
        profiles.extend(
            Store::open(&crate::history::dir())
                .and_then(|s| s.agent_profiles())
                .map_err(error)?
                .into_iter()
                .map(|p| {
                    let id = format!("custom-{}", p.id);
                    let mut view = View::from(p);
                    view.access = crate::settings::specialist_chat_access(&app_for_access, &id);
                    view
                }),
        );
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
) -> Result<View, String> {
    page(&window)?;
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
        let mut view = store
            .agent_profile_save(&profile)
            .map(View::from)
            .map_err(error)?;
        view.access = settings::specialist_chat_access(&app, &view.id);
        Ok(view)
    })
    .await
    .map_err(|_| "storage".to_owned())?
}
