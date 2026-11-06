//! Persistent native Unicode copying, with Windows clipboard-history/cloud exclusion.
use arboard::SetExtWindows;
pub struct Clipboard(arboard::Clipboard);
impl std::fmt::Debug for Clipboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Clipboard(Windows)")
    }
}
impl Clipboard {
    pub fn new() -> Result<Self, String> {
        arboard::Clipboard::new()
            .map(Self)
            .map_err(|_| "The Windows clipboard is unavailable.".into())
    }
    pub fn copy(&mut self, text: &str) -> Result<(), String> {
        if text.contains('\0') {
            return Err("Dictation text contains a NUL character.".into());
        }
        self.0
            .set()
            .exclude_from_monitoring()
            .text(text)
            .map_err(|_| "The Windows clipboard refused the text.".to_string())?;
        self.confirm(text)
    }
    pub fn text(&mut self) -> Option<String> {
        self.0.get_text().ok()
    }
    pub fn confirm(&mut self, text: &str) -> Result<(), String> {
        if self.text().as_deref() == Some(text) {
            Ok(())
        } else {
            Err("The Windows clipboard did not confirm the text.".into())
        }
    }
}
