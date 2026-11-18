//! Desktop notifications over the freedesktop notification spec.

use std::path::{Path, PathBuf};

use anyhow::Context;
use notify_rust::Notification;
use shiver_core::{Chorus, Interjection, Voice, line_id};

use crate::output::Output;

/// Desktop notifications. `default_icon` turns the Signal Flame fallback on.
pub struct Notify {
    pub default_icon: bool,
}

impl Output for Notify {
    fn emit(&self, interjection: &Interjection) -> anyhow::Result<()> {
        // Only install the default icon when it's needed.
        let default = if self.default_icon && interjection.icon.is_none() {
            crate::icon::default_icon()
        } else {
            None
        };
        let f = fields(interjection, default.as_deref());
        let mut n = Notification::new();
        n.appname("shiver").summary(&f.summary).body(&f.body);
        if let Some(icon) = &f.icon {
            // The spec wants a file:// URI for the app icon; many servers
            // (e.g. COSMIC) show the `image-path` hint instead, so send both.
            n.icon(icon).image_path(icon);
        }
        n.show().context("couldn't send the notification")?;
        Ok(())
    }
}

/// What goes into the notification.
#[derive(Debug, PartialEq)]
struct Fields {
    summary: String,
    body: String,
    icon: Option<String>,
}

/// The notification's fields; `default_icon` is used when the voice has none.
fn fields(i: &Interjection, default_icon: Option<&Path>) -> Fields {
    Fields {
        summary: i.voice.display.clone(),
        body: i.text.clone(),
        icon: i.icon.as_deref().or(default_icon).map(file_uri),
    }
}

/// `/a b/c.png` → `file:///a%20b/c.png`.
fn file_uri(path: &Path) -> String {
    let mut uri = String::from("file://");
    for byte in path.to_string_lossy().bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~".contains(&byte) {
            uri.push(char::from(byte));
        } else {
            uri.push_str(&format!("%{byte:02X}"));
        }
    }
    uri
}

/// Builds a one-off interjection for `notify --voice V --text TEXT`.
///
/// A loaded voice supplies its display name, icon and sound; an unknown `voice`
/// is used as the display name. `sound` overrides the voice's sound.
pub fn one_off(chorus: &Chorus, voice: &str, text: &str, sound: Option<PathBuf>) -> Interjection {
    let voice = chorus.voice(voice).cloned().unwrap_or_else(|| Voice {
        key: voice.to_owned(),
        display: voice.to_owned(),
        color: None,
        sound: None,
        icon: None,
    });
    Interjection {
        id: line_id(&voice.key, text),
        text: text.to_owned(),
        sound: sound.or_else(|| voice.sound.clone()),
        icon: voice.icon.clone(),
        voice,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    /// A chorus with voice `drift` that has an icon and a sound.
    fn sample_chorus() -> (tempfile::TempDir, Chorus) {
        let dir = tempfile::tempdir().unwrap();
        let pack = dir.path().join("p");
        std::fs::create_dir(&pack).unwrap();
        std::fs::write(
            pack.join("pack.toml"),
            "[voices.drift]\ndisplay = \"DRIFT\"\nsound = \"drift.ogg\"\nicon = \"drift.png\"\n",
        )
        .unwrap();
        std::fs::write(pack.join("drift.ogg"), b"").unwrap();
        std::fs::write(pack.join("drift.png"), b"").unwrap();
        let (chorus, warnings) = Chorus::load(dir.path()).unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        (dir, chorus)
    }

    #[test]
    fn builds_fields() {
        let (dir, chorus) = sample_chorus();
        let i = one_off(&chorus, "drift", "Close a tab or two.", None);
        assert_eq!(
            fields(&i, None),
            Fields {
                summary: "DRIFT".into(),
                body: "Close a tab or two.".into(),
                icon: Some(file_uri(&dir.path().join("p/drift.png"))),
            }
        );
    }

    #[test]
    fn uses_default_icon_without_voice_icon() {
        let (_dir, chorus) = sample_chorus();
        let i = one_off(&chorus, "Build Bot", "Tests passed.", None);
        let f = fields(&i, Some(Path::new("/data/shiver/icons/shiver.png")));
        assert_eq!(
            f.icon.as_deref(),
            Some("file:///data/shiver/icons/shiver.png")
        );
    }

    #[test]
    fn default_icon_can_be_off() {
        // With the setting off, `emit` passes no default to `fields`.
        let (_dir, chorus) = sample_chorus();
        let i = one_off(&chorus, "Build Bot", "Tests passed.", None);
        assert_eq!(fields(&i, None).icon, None);
        let off = Notify {
            default_icon: false,
        };
        assert!(!off.default_icon);
    }

    #[test]
    fn icons_are_file_uris() {
        assert_eq!(file_uri(Path::new("/a/b.png")), "file:///a/b.png");
        assert_eq!(
            file_uri(Path::new("/my icons/ünï.png")),
            "file:///my%20icons/%C3%BCn%C3%AF.png"
        );
    }

    #[test]
    fn voice_icon_beats_default() {
        let (dir, chorus) = sample_chorus();
        let i = one_off(&chorus, "drift", "Hi.", None);
        let f = fields(&i, Some(Path::new("/data/shiver/icons/shiver.png")));
        assert_eq!(f.icon, Some(file_uri(&dir.path().join("p/drift.png"))));
    }

    #[test]
    fn one_off_uses_known_voice() {
        let (dir, chorus) = sample_chorus();
        let i = one_off(&chorus, "drift", "Hello.", None);
        assert_eq!(i.voice.display, "DRIFT");
        assert_eq!(i.sound, Some(dir.path().join("p/drift.ogg")));
        assert_eq!(i.icon, Some(dir.path().join("p/drift.png")));
    }

    #[test]
    fn one_off_unknown_voice_uses_name() {
        let (_dir, chorus) = sample_chorus();
        let i = one_off(&chorus, "Build Bot", "Tests passed.", None);
        assert_eq!(i.voice.display, "Build Bot");
        assert_eq!(i.sound, None);
        assert_eq!(i.icon, None);
        assert_eq!(fields(&i, None).icon, None);
    }

    #[test]
    fn one_off_sound_overrides_voice() {
        let (_dir, chorus) = sample_chorus();
        let i = one_off(&chorus, "drift", "Ding.", Some("/tmp/ding.ogg".into()));
        assert_eq!(i.sound.as_deref(), Some(Path::new("/tmp/ding.ogg")));
    }
}
