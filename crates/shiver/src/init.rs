//! `shiver init`: install the default pack into the packs folder.

use std::path::{Path, PathBuf};

use anyhow::{Context, bail};

/// The default pack, compiled into the binary.
const DEFAULT_PACK: &str = include_str!("../packs/default/pack.toml");

/// Folder name of the default pack inside the packs folder.
const FOLDER: &str = "default";

/// Shown after installing the pack.
pub const HINT: &str = "try `shiver say` or `shiver notify`; edit the pack to make it yours";

/// Whether `init` created a new pack or replaced an existing one.
#[derive(Debug, PartialEq)]
pub enum Outcome {
    Created(PathBuf),
    Replaced(PathBuf),
}

/// Writes the default pack to `<packs_dir>/default/pack.toml`.
///
/// Refuses to overwrite an existing file unless `force` is set.
pub fn install(packs_dir: &Path, force: bool) -> anyhow::Result<Outcome> {
    let folder = packs_dir.join(FOLDER);
    let file = folder.join("pack.toml");
    let existed = file.exists();
    if existed && !force {
        bail!(
            "{} already exists; use --force to overwrite",
            file.display()
        );
    }
    std::fs::create_dir_all(&folder)
        .with_context(|| format!("can't create {}", folder.display()))?;
    std::fs::write(&file, DEFAULT_PACK)
        .with_context(|| format!("can't write {}", file.display()))?;
    Ok(if existed {
        Outcome::Replaced(file)
    } else {
        Outcome::Created(file)
    })
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use shiver_core::Chorus;

    use super::*;

    const VOICES: [&str; 5] = ["backbone", "clockwork", "lantern", "signal", "static"];
    const TAGS: [&str; 6] = ["break", "fail", "focus", "late", "morning", "win"];

    fn installed() -> (tempfile::TempDir, Chorus) {
        let dir = tempfile::tempdir().unwrap();
        install(dir.path(), false).unwrap();
        let (chorus, warnings) = Chorus::load(dir.path()).unwrap();
        // System sounds may be missing (e.g. on CI); nothing else may warn.
        let other: Vec<_> = warnings
            .iter()
            .filter(|w| !w.message.contains("sound file"))
            .collect();
        assert!(other.is_empty(), "{other:?}");
        (dir, chorus)
    }

    #[test]
    fn default_pack_loads_cleanly() {
        let (_dir, chorus) = installed();

        assert_eq!(chorus.packs().len(), 1);
        assert_eq!(chorus.packs()[0].name, "default");
        let voices: BTreeSet<_> = chorus.packs()[0]
            .voices
            .iter()
            .map(String::as_str)
            .collect();
        assert_eq!(voices, BTreeSet::from(VOICES));

        let mut per_voice: BTreeMap<&str, usize> = BTreeMap::new();
        for line in chorus.lines() {
            *per_voice.entry(line.voice.as_str()).or_default() += 1;
        }
        for voice in VOICES {
            let n = per_voice.get(voice).copied().unwrap_or(0);
            assert!(n >= 5, "voice {voice} has only {n} lines");
            assert!(
                chorus.voice(voice).unwrap().color.is_some(),
                "{voice} has a colour"
            );
        }
    }

    #[test]
    fn default_pack_uses_known_tags() {
        let (_dir, chorus) = installed();
        let mut uses: BTreeMap<&str, usize> = BTreeMap::new();
        for tag in chorus.lines().iter().flat_map(|l| &l.tags) {
            assert!(TAGS.contains(&tag.as_str()), "unknown tag `{tag}`");
            *uses.entry(tag.as_str()).or_default() += 1;
        }
        for tag in TAGS {
            let n = uses.get(tag).copied().unwrap_or(0);
            assert!(n >= 2, "tag {tag} used {n} times");
        }
    }

    #[test]
    fn default_pack_sounds_are_system_sounds() {
        let sounds: Vec<_> = DEFAULT_PACK
            .lines()
            .filter(|l| l.starts_with("sound = "))
            .collect();
        assert_eq!(sounds.len(), 5);
        assert!(
            sounds
                .iter()
                .all(|l| l.contains("\"/usr/share/sounds/freedesktop/stereo/"))
        );
    }

    #[test]
    fn install_reports_created_then_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("default/pack.toml");
        assert_eq!(
            install(dir.path(), false).unwrap(),
            Outcome::Created(file.clone())
        );
        assert!(install(dir.path(), false).is_err());
        assert_eq!(install(dir.path(), true).unwrap(), Outcome::Replaced(file));
    }
}
