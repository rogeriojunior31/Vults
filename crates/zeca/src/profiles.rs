//! Inert built-in profile descriptions; selecting one grants no execution or access.
use serde::Serialize;

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Profile {
    pub id: &'static str,
    pub revision: u32,
    pub title: &'static str,
    pub about: &'static str,
    pub species: &'static str,
    pub instructions: &'static str,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Skill {
    pub id: &'static str,
    pub revision: u32,
    pub title: &'static str,
    pub content: &'static str,
}

// Marks sentences rendered through the UI's translation catalogs.
const fn tk(text: &'static str) -> &'static str {
    text
}

pub const PRESETS: [Profile; 3] = [
    Profile {
        id: "developer",
        revision: 1,
        title: tk("Developer"),
        about: tk("Focused changes with clear checks and explanations."),
        species: "atratus",
        instructions: tk(
            "Implement the requested change within the selected project. Explain the result, checks and remaining limitations.",
        ),
    },
    Profile {
        id: "reviewer",
        revision: 1,
        title: tk("Reviewer"),
        about: tk("Actionable defects, regression risks and supporting evidence."),
        species: "gypaetus",
        instructions: tk(
            "Review the requested changes for actionable defects and regressions. Explain each finding with its location, impact and evidence.",
        ),
    },
    Profile {
        id: "researcher",
        revision: 1,
        title: tk("Researcher"),
        about: tk("Source-based investigation with explicit unknowns."),
        species: "aura",
        instructions: tk(
            "Investigate the requested question using relevant sources. Distinguish evidence, inferences and unresolved questions.",
        ),
    },
];

pub const SKILLS: [Skill; 3] = [
    Skill {
        id: "small-diff",
        revision: 1,
        title: tk("Keep changes focused"),
        content: tk(
            "Keep the change focused on the requested behavior. Avoid unrelated refactoring and explain any necessary scope expansion.",
        ),
    },
    Skill {
        id: "verify-change",
        revision: 1,
        title: tk("Verify the change"),
        content: tk(
            "Choose checks that exercise the changed behavior. Report what passed and distinguish automated checks from manual validation.",
        ),
    },
    Skill {
        id: "evidence-first-review",
        revision: 1,
        title: tk("Evidence-first review"),
        content: tk(
            "Prioritize actionable defects. Support each finding with a code location and a concrete failure path; report uncertainty explicitly.",
        ),
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_identity_and_revision_are_explicit_and_unique() {
        let mut ids = std::collections::HashSet::new();
        for preset in PRESETS {
            assert!(ids.insert(preset.id));
            assert!(preset.id.bytes().all(|b| b.is_ascii_lowercase()));
            assert_eq!(preset.revision, 1);
            assert!(!preset.title.is_empty() && !preset.about.is_empty());
            assert!(!preset.species.is_empty() && !preset.instructions.is_empty());
        }
    }
}
