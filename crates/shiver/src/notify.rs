//! Desktop notifications over the freedesktop notification spec.

use std::path::PathBuf;

use anyhow::Context;
use notify_rust::Notification;
use shiver_core::{Chorus, Interjection, Voice};

use crate::output::Output;

pub struct Notify;

impl Output for Notify {
    fn emit(&self, interjection: &Interjection) -> anyhow::Result<()> {
        let f = fields(interjection);
        let mut n = Notification::new();
        n.appname("shiver").summary(&f.summary).body(&f.body);
        if let Some(icon) = &f.icon {
            n.icon(icon);
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

fn fields(i: &Interjection) -> Fields {
    Fields {
        summary: i.voice.display.clone(),
        body: i.text.clone(),
        icon: i.icon.as_ref().map(|p| p.to_string_lossy().into_owned()),
    }
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
            fields(&i),
            Fields {
                summary: "DRIFT".into(),
                body: "Close a tab or two.".into(),
                icon: Some(dir.path().join("p/drift.png").to_string_lossy().into()),
            }
        );
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
        assert_eq!(fields(&i).icon, None);
    }

    #[test]
    fn one_off_sound_overrides_voice() {
        let (_dir, chorus) = sample_chorus();
        let i = one_off(&chorus, "drift", "Ding.", Some("/tmp/ding.ogg".into()));
        assert_eq!(i.sound.as_deref(), Some(Path::new("/tmp/ding.ogg")));
    }
}
