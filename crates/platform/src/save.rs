//! "Save as" through the desktop's own dialog (the xdg-desktop-portal `FileChooser`): the app learns
//! only the one path the user picked, and writes nothing anywhere else.

use std::path::PathBuf;

use ashpd::desktop::file_chooser::{FileFilter, SelectedFiles};

/// Asks where to save `name` (a PNG image); none when the user cancels or the desktop has no portal.
pub async fn png(title: &str, name: &str) -> Option<PathBuf> {
    let request = SelectedFiles::save_file()
        .title(title)
        .accept_label("Save")
        .modal(true)
        .current_name(name)
        .filter(FileFilter::new("PNG image").mimetype("image/png"))
        .send()
        .await
        .ok()?;
    let files = request.response().ok()?;
    file_path(files.uris().first()?.as_str())
}

/// A `file://` URI as a path, `%XX` decoded; none for any other scheme or a host that is not ours.
pub fn file_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let path = rest.strip_prefix("localhost").unwrap_or(rest);
    if !path.starts_with('/') {
        return None;
    }
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = std::str::from_utf8(bytes.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    use std::os::unix::ffi::OsStringExt;
    Some(PathBuf::from(std::ffi::OsString::from_vec(out)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_uri_is_a_decoded_path_and_nothing_else_is() {
        assert_eq!(
            file_path("file:///home/me/Pictures/My%20week%20%C3%A9.png"),
            Some(PathBuf::from("/home/me/Pictures/My week \u{e9}.png"))
        );
        assert_eq!(
            file_path("file://localhost/tmp/a.png"),
            Some(PathBuf::from("/tmp/a.png"))
        );
        for bad in [
            "https://example.com/a.png",
            "file://other-host/a.png",
            "file:///a%zz.png",
            "file:///a%2",
            "/tmp/a.png",
        ] {
            assert_eq!(file_path(bad), None, "{bad}");
        }
    }
}
