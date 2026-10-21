//! The Nest owns local profiles; saving instructions never starts an agent or changes access.
use serde::{Deserialize, Serialize};
use vults_store::{Store, profiles::Profile};

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
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "camelCase", deny_unknown_fields)]
pub enum Edit {
    Copy {
        preset: String,
        title: String,
        instructions: String,
        species: String,
        look: String,
    },
    Update {
        id: i64,
        revision: u32,
        title: String,
        instructions: String,
        species: String,
        look: String,
    },
}

fn page(window: &tauri::Webview) -> Result<(), String> {
    if window.label() != crate::NEST {
        return Err("Agent profiles are available in the Nest only.".into());
    }
    Ok(())
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
pub async fn agent_profiles(window: tauri::Webview) -> Result<Vec<View>, String> {
    page(&window)?;
    tauri::async_runtime::spawn_blocking(|| {
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
            })
            .collect();
        profiles.extend(
            Store::open(&crate::history::dir())
                .and_then(|s| s.agent_profiles())
                .map_err(error)?
                .into_iter()
                .map(View::from),
        );
        Ok(profiles)
    })
    .await
    .map_err(|_| "storage".to_owned())?
}

#[tauri::command]
pub async fn agent_profile_save(window: tauri::Webview, edit: Edit) -> Result<View, String> {
    page(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut store = Store::open(&crate::history::dir()).map_err(error)?;
        let profile = match edit {
            Edit::Copy {
                preset,
                title,
                instructions,
                species,
                look,
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
                }
            }
            Edit::Update {
                id,
                revision,
                title,
                instructions,
                species,
                look,
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
                p
            }
        };
        store.agent_profile_save(&profile).map(View::from).map_err(error)
    })
    .await
    .map_err(|_| "storage".to_owned())?
}
