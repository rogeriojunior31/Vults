//! Pin a talk release to its press; an active hands-free dictation survives focus changes.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    Island,
    Nest,
    Other,
    External,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Island,
    Nest,
    Dictation,
}

#[derive(Default)]
pub struct Router {
    held: Option<Target>,
}

impl Router {
    pub fn reset(&mut self) {
        self.held = None;
    }

    pub fn talk(&mut self, down: bool, surface: Surface, zeca: bool, active: bool) -> Option<Target> {
        if !down {
            return self.held.take();
        }
        if self.held.is_some() {
            return None;
        }
        let target = if active {
            Target::Dictation
        } else {
            match surface {
                Surface::Island if zeca => Target::Island,
                Surface::Nest if zeca => Target::Nest,
                Surface::Other => return None,
                _ => Target::Dictation,
            }
        };
        self.held = Some(target);
        Some(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn release_returns_to_the_press_owner_instead_of_the_new_focus() {
        for (surface, zeca, expected) in [
            (Surface::Nest, true, Target::Nest),
            (Surface::Island, true, Target::Island),
            (Surface::External, true, Target::Dictation),
            (Surface::External, false, Target::Dictation),
            (Surface::Nest, false, Target::Dictation),
            (Surface::Island, false, Target::Dictation),
        ] {
            let mut route = Router::default();
            assert_eq!(route.talk(true, surface, zeca, false), Some(expected));
            assert_eq!(route.talk(true, Surface::Other, zeca, false), None);
            assert_eq!(route.talk(false, Surface::Other, !zeca, false), Some(expected));
            assert_eq!(route.talk(false, surface, zeca, false), None);
        }
    }
    #[test]
    fn settings_never_starts_chat_and_locked_dictation_keeps_its_owner() {
        let mut route = Router::default();
        assert_eq!(route.talk(true, Surface::Other, true, false), None);
        assert_eq!(route.talk(false, Surface::Other, true, false), None);
        assert_eq!(
            route.talk(true, Surface::Nest, true, true),
            Some(Target::Dictation)
        );
    }
}
