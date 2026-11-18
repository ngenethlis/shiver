//! Parsing a single `pack.toml`.
//!
//! Parsing applies defaults and resolves relative paths, but doesn't validate
//! values or touch the filesystem; the library does that when loading.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// A voice that lines are attributed to.
#[derive(Debug, Clone, PartialEq)]
pub struct Voice {
    /// Key used by lines to refer to this voice, e.g. `resolve`.
    pub key: String,
    /// Name shown to the user, e.g. `RESOLVE`. Defaults to the key.
    pub display: String,
    /// Colour as `#rrggbb`, if set and valid.
    pub color: Option<String>,
    /// Default sound for this voice's lines.
    pub sound: Option<PathBuf>,
    /// Icon for this voice.
    pub icon: Option<PathBuf>,
}

/// One line a voice can say.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    /// Key of the voice saying this line.
    pub voice: String,
    /// The text, trimmed.
    pub text: String,
    /// Tags used for filtering.
    pub tags: Vec<String>,
    /// Relative chance of being picked. Always finite and greater than 0 once loaded.
    pub weight: f64,
    /// Sound for this line, overriding the voice's sound.
    pub sound: Option<PathBuf>,
}

/// A parsed pack, before validation.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Pack {
    pub name: String,
    pub license: Option<String>,
    /// Voices in key order.
    pub voices: Vec<Voice>,
    /// Lines in file order.
    pub lines: Vec<Line>,
}

#[derive(Deserialize)]
struct RawPack {
    #[serde(default)]
    pack: RawMeta,
    #[serde(default)]
    voices: BTreeMap<String, RawVoice>,
    #[serde(default)]
    lines: Vec<RawLine>,
}

#[derive(Deserialize, Default)]
struct RawMeta {
    name: Option<String>,
    license: Option<String>,
}

#[derive(Deserialize)]
struct RawVoice {
    display: Option<String>,
    color: Option<String>,
    sound: Option<PathBuf>,
    icon: Option<PathBuf>,
}

#[derive(Deserialize)]
struct RawLine {
    voice: String,
    text: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default = "default_weight")]
    weight: f64,
    sound: Option<PathBuf>,
}

fn default_weight() -> f64 {
    1.0
}

/// Parses the contents of a `pack.toml` found in `dir`.
///
/// The pack name defaults to the folder name, and relative paths are resolved
/// against `dir`.
pub(crate) fn parse(source: &str, dir: &Path) -> Result<Pack, toml::de::Error> {
    let raw: RawPack = toml::from_str(source)?;
    let resolve = |p: PathBuf| dir.join(p);

    let name = raw.pack.name.unwrap_or_else(|| {
        dir.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    });

    let voices = raw
        .voices
        .into_iter()
        .map(|(key, v)| Voice {
            display: v.display.unwrap_or_else(|| key.clone()),
            key,
            color: v.color,
            sound: v.sound.map(resolve),
            icon: v.icon.map(resolve),
        })
        .collect();

    let lines = raw
        .lines
        .into_iter()
        .map(|l| Line {
            voice: l.voice,
            text: l.text.trim().to_owned(),
            tags: l.tags,
            weight: l.weight,
            sound: l.sound.map(resolve),
        })
        .collect();

    Ok(Pack {
        name,
        license: raw.pack.license,
        voices,
        lines,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_pack() {
        let src = r##"
            [pack]
            name = "desk"
            license = "CC0-1.0"

            [voices.resolve]
            display = "RESOLVE"
            color = "#8a5cd6"
            sound = "sounds/resolve.ogg"
            icon = "icons/resolve.png"

            [[lines]]
            voice = "resolve"
            text = "  The kettle can wait. Finish the paragraph.  "
            tags = ["late", "idle"]
            weight = 2.5
            sound = "sounds/other.ogg"
        "##;
        let pack = parse(src, Path::new("/packs/desk")).unwrap();

        assert_eq!(pack.name, "desk");
        assert_eq!(pack.license.as_deref(), Some("CC0-1.0"));
        assert_eq!(
            pack.voices,
            vec![Voice {
                key: "resolve".into(),
                display: "RESOLVE".into(),
                color: Some("#8a5cd6".into()),
                sound: Some("/packs/desk/sounds/resolve.ogg".into()),
                icon: Some("/packs/desk/icons/resolve.png".into()),
            }]
        );
        assert_eq!(
            pack.lines,
            vec![Line {
                voice: "resolve".into(),
                text: "The kettle can wait. Finish the paragraph.".into(),
                tags: vec!["late".into(), "idle".into()],
                weight: 2.5,
                sound: Some("/packs/desk/sounds/other.ogg".into()),
            }]
        );
    }

    #[test]
    fn applies_defaults() {
        let src = r#"
            [voices.drift]

            [[lines]]
            voice = "drift"
            text = "Your tea went cold an hour ago."
        "#;
        let pack = parse(src, Path::new("/packs/my-folder")).unwrap();

        assert_eq!(pack.name, "my-folder");
        assert_eq!(pack.license, None);
        assert_eq!(pack.voices[0].display, "drift");
        assert_eq!(pack.voices[0].color, None);
        assert_eq!(pack.lines[0].tags, Vec::<String>::new());
        assert_eq!(pack.lines[0].weight, 1.0);
        assert_eq!(pack.lines[0].sound, None);
    }

    #[test]
    fn ignores_unknown_keys() {
        let src = r#"
            future_setting = true

            [pack]
            mood = "bright"

            [voices.drift]
            font = "serif"

            [[lines]]
            voice = "drift"
            text = "Window open. Breathe."
            cooldown = 30
        "#;
        let pack = parse(src, Path::new("/packs/p")).unwrap();
        assert_eq!(pack.lines.len(), 1);
    }

    #[test]
    fn keeps_absolute_paths() {
        let src = r#"
            [voices.drift]
            sound = "/usr/share/sounds/ping.ogg"
        "#;
        let pack = parse(src, Path::new("/packs/p")).unwrap();
        assert_eq!(
            pack.voices[0].sound.as_deref(),
            Some(Path::new("/usr/share/sounds/ping.ogg"))
        );
    }
}
