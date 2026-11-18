//! Loading every pack in a folder into one chorus.

use std::collections::BTreeMap;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use crate::pack::{self, Line, Voice};

/// All loaded voices and lines, merged across packs.
#[derive(Debug, Default)]
pub struct Chorus {
    pub(crate) voices: BTreeMap<String, Voice>,
    pub(crate) lines: Vec<Line>,
    packs: Vec<PackSummary>,
}

/// What a loaded pack contributed, for listing.
#[derive(Debug, Clone, PartialEq)]
pub struct PackSummary {
    /// The pack's name.
    pub name: String,
    /// The pack's folder.
    pub dir: PathBuf,
    /// Keys of the voices this pack defined, after merging.
    pub voices: Vec<String>,
    /// Number of lines loaded from this pack.
    pub lines: usize,
}

/// Something in a pack was wrong and was skipped; the rest still loaded.
#[derive(Debug, Clone, PartialEq)]
pub struct Warning {
    /// The file the warning is about (a `pack.toml`, or a pack folder).
    pub file: PathBuf,
    /// What was wrong and what was skipped.
    pub message: String,
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.file.display(), self.message)
    }
}

/// The packs folder itself couldn't be read.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("can't read packs folder {}", path.display())]
    ReadDir {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

const PACK_FILE: &str = "pack.toml";

/// A pack that parsed, with where it came from.
struct Loaded {
    dir: PathBuf,
    file: PathBuf,
    pack: pack::Pack,
}

impl Chorus {
    /// Loads every pack in `dir`: each subfolder containing a `pack.toml`.
    ///
    /// Packs load in folder-name order. Anything broken is skipped and reported
    /// in the returned warnings. A missing `dir` gives an empty chorus.
    pub fn load(dir: &Path) -> Result<(Chorus, Vec<Warning>), LoadError> {
        Chorus::load_skipping(dir, &[])
    }

    /// Like [`Chorus::load`], but skips the pack folders named in `skip`.
    pub fn load_skipping(dir: &Path, skip: &[String]) -> Result<(Chorus, Vec<Warning>), LoadError> {
        let mut warnings = Vec::new();
        let loaded = read_packs(dir, skip, &mut warnings)?;

        let mut chorus = Chorus::default();
        let mut summaries: Vec<PackSummary> = loaded
            .iter()
            .map(|l| PackSummary {
                name: l.pack.name.clone(),
                dir: l.dir.clone(),
                voices: Vec::new(),
                lines: 0,
            })
            .collect();

        // Voices first, from every pack, so lines can use voices from any pack.
        let mut owner: BTreeMap<String, String> = BTreeMap::new();
        for (l, summary) in loaded.iter().zip(&mut summaries) {
            for voice in &l.pack.voices {
                if let Some(first) = owner.get(&voice.key) {
                    warnings.push(warn(
                        &l.file,
                        format!(
                            "voice `{}` is already defined by pack `{first}`; this definition is ignored",
                            voice.key
                        ),
                    ));
                    continue;
                }
                let voice = check_voice(voice.clone(), &l.file, &mut warnings);
                owner.insert(voice.key.clone(), l.pack.name.clone());
                summary.voices.push(voice.key.clone());
                chorus.voices.insert(voice.key.clone(), voice);
            }
        }

        for (l, summary) in loaded.iter().zip(&mut summaries) {
            for (i, line) in l.pack.lines.iter().enumerate() {
                if let Some(line) = check_line(line.clone(), i + 1, &chorus, &l.file, &mut warnings)
                {
                    chorus.lines.push(line);
                    summary.lines += 1;
                }
            }
        }

        chorus.packs = summaries;
        Ok((chorus, warnings))
    }

    /// The loaded packs, in folder order.
    pub fn packs(&self) -> &[PackSummary] {
        &self.packs
    }

    /// Looks up a voice by key.
    pub fn voice(&self, key: &str) -> Option<&Voice> {
        self.voices.get(key)
    }

    /// All loaded lines, in folder order and then file order.
    pub fn lines(&self) -> &[Line] {
        &self.lines
    }
}

/// Reads and parses each pack folder in `dir`, in folder-name order.
fn read_packs(
    dir: &Path,
    skip: &[String],
    warnings: &mut Vec<Warning>,
) -> Result<Vec<Loaded>, LoadError> {
    let read_err = |source| LoadError::ReadDir {
        path: dir.to_path_buf(),
        source,
    };
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(read_err(e)),
    };

    let mut folders = Vec::new();
    for entry in entries {
        let path = entry.map_err(read_err)?.path();
        let skipped = path
            .file_name()
            .is_some_and(|n| skip.iter().any(|s| n == s.as_str()));
        if path.is_dir() && !skipped {
            folders.push(path);
        }
    }
    folders.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    let mut loaded = Vec::new();
    for folder in folders {
        let file = folder.join(PACK_FILE);
        if !file.is_file() {
            warnings.push(warn(&folder, format!("no {PACK_FILE}; folder skipped")));
            continue;
        }
        let source = match std::fs::read_to_string(&file) {
            Ok(s) => s,
            Err(e) => {
                warnings.push(warn(&file, format!("can't read file: {e}; pack skipped")));
                continue;
            }
        };
        match pack::parse(&source, &folder) {
            Ok(pack) => loaded.push(Loaded {
                dir: folder,
                file,
                pack,
            }),
            Err(e) => warnings.push(warn(&file, format!("invalid pack, skipped: {e}"))),
        }
    }
    Ok(loaded)
}

/// Drops a voice's invalid colour and missing asset files, with warnings.
fn check_voice(mut voice: Voice, file: &Path, warnings: &mut Vec<Warning>) -> Voice {
    if let Some(color) = &voice.color
        && !is_hex_color(color)
    {
        warnings.push(warn(
            file,
            format!(
                "voice `{}`: color `{color}` isn't #rrggbb; color ignored",
                voice.key
            ),
        ));
        voice.color = None;
    }
    let what = format!("voice `{}`", voice.key);
    voice.sound = existing(voice.sound, &what, "sound", file, warnings);
    voice.icon = existing(voice.icon, &what, "icon", file, warnings);
    voice
}

/// Validates a line; returns `None` (with a warning) if it must be skipped.
fn check_line(
    mut line: Line,
    number: usize,
    chorus: &Chorus,
    file: &Path,
    warnings: &mut Vec<Warning>,
) -> Option<Line> {
    let what = format!("line {number}");
    let problem = if !chorus.voices.contains_key(&line.voice) {
        Some(format!("unknown voice `{}`", line.voice))
    } else if line.text.is_empty() {
        Some("text is empty".to_owned())
    } else if !(line.weight.is_finite() && line.weight > 0.0) {
        Some(format!(
            "weight {} must be a number greater than 0",
            line.weight
        ))
    } else {
        None
    };
    if let Some(problem) = problem {
        warnings.push(warn(file, format!("{what}: {problem}; line skipped")));
        return None;
    }
    line.sound = existing(line.sound, &what, "sound", file, warnings);
    Some(line)
}

/// Keeps `path` if the file exists; otherwise warns and returns `None`.
fn existing(
    path: Option<PathBuf>,
    what: &str,
    kind: &str,
    file: &Path,
    warnings: &mut Vec<Warning>,
) -> Option<PathBuf> {
    let path = path?;
    if path.is_file() {
        Some(path)
    } else {
        warnings.push(warn(
            file,
            format!(
                "{what}: {kind} file {} not found; {kind} ignored",
                path.display()
            ),
        ));
        None
    }
}

fn is_hex_color(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

fn warn(file: &Path, message: String) -> Warning {
    Warning {
        file: file.to_path_buf(),
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Packs;

    const VOICE: &str = "[voices.drift]\ndisplay = \"DRIFT\"\n";

    /// Asserts there's exactly one warning, about `file`, containing `text`.
    fn assert_one_warning(warnings: &[Warning], file: &Path, text: &str) {
        assert_eq!(warnings.len(), 1, "warnings: {warnings:?}");
        assert_eq!(warnings[0].file, file);
        assert!(
            warnings[0].message.contains(text),
            "warning `{}` should mention `{text}`",
            warnings[0]
        );
    }

    #[test]
    fn loads_packs_in_folder_order() {
        let packs = Packs::new();
        packs.add(
            "b-second",
            &format!("{VOICE}[[lines]]\nvoice = \"drift\"\ntext = \"B\"\n"),
        );
        packs.add(
            "a-first",
            "[voices.ember]\n[[lines]]\nvoice = \"ember\"\ntext = \"A\"\n",
        );

        let (chorus, warnings) = Chorus::load(packs.dir()).unwrap();

        assert!(warnings.is_empty(), "{warnings:?}");
        let names: Vec<_> = chorus.packs().iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["a-first", "b-second"]);
        let texts: Vec<_> = chorus.lines().iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, ["A", "B"]);
    }

    #[test]
    fn skips_disabled_folders() {
        let packs = Packs::new();
        packs.add("keep", VOICE);
        packs.add("work", VOICE);
        let (chorus, warnings) = Chorus::load_skipping(packs.dir(), &["work".to_owned()]).unwrap();
        let names: Vec<_> = chorus.packs().iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["keep"]);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn missing_dir_is_empty() {
        let packs = Packs::new();
        let (chorus, warnings) = Chorus::load(&packs.dir().join("nope")).unwrap();
        assert!(chorus.packs().is_empty());
        assert!(chorus.lines.is_empty());
        assert!(warnings.is_empty());
    }

    #[test]
    fn unreadable_dir_is_an_error() {
        let packs = Packs::new();
        let file = packs.dir().join("not-a-folder");
        std::fs::write(&file, "").unwrap();
        assert!(matches!(
            Chorus::load(&file),
            Err(LoadError::ReadDir { .. })
        ));
    }

    #[test]
    fn warns_on_folder_without_pack_toml() {
        let packs = Packs::new();
        let folder = packs.dir().join("empty");
        std::fs::create_dir(&folder).unwrap();
        std::fs::write(packs.dir().join("notes.txt"), "loose file").unwrap();

        let (chorus, warnings) = Chorus::load(packs.dir()).unwrap();

        assert!(chorus.packs().is_empty());
        assert_one_warning(&warnings, &folder, "no pack.toml");
    }

    #[test]
    fn skips_malformed_pack() {
        let packs = Packs::new();
        let bad = packs.add("bad", "[[lines]]\ntext = \"no voice key\"\n");
        packs.add(
            "good",
            &format!("{VOICE}[[lines]]\nvoice = \"drift\"\ntext = \"Fine.\"\n"),
        );

        let (chorus, warnings) = Chorus::load(packs.dir()).unwrap();

        assert_eq!(chorus.packs().len(), 1);
        assert_eq!(chorus.packs()[0].name, "good");
        assert_one_warning(&warnings, &bad, "invalid pack");
    }

    #[test]
    fn skips_invalid_toml() {
        let packs = Packs::new();
        let bad = packs.add("bad", "this is = = not toml");

        let (chorus, warnings) = Chorus::load(packs.dir()).unwrap();

        assert!(chorus.packs().is_empty());
        assert_one_warning(&warnings, &bad, "invalid pack");
    }

    #[test]
    fn skips_line_with_unknown_voice() {
        let packs = Packs::new();
        let file = packs.add(
            "p",
            &format!(
                "{VOICE}[[lines]]\nvoice = \"ghost\"\ntext = \"Boo.\"\n\
                 [[lines]]\nvoice = \"drift\"\ntext = \"Kept.\"\n"
            ),
        );

        let (chorus, warnings) = Chorus::load(packs.dir()).unwrap();

        assert_eq!(chorus.lines.len(), 1);
        assert_eq!(chorus.lines[0].text, "Kept.");
        assert_one_warning(&warnings, &file, "line 1: unknown voice `ghost`");
    }

    #[test]
    fn skips_empty_text() {
        let packs = Packs::new();
        let file = packs.add(
            "p",
            &format!("{VOICE}[[lines]]\nvoice = \"drift\"\ntext = \"  \"\n"),
        );

        let (chorus, warnings) = Chorus::load(packs.dir()).unwrap();

        assert!(chorus.lines.is_empty());
        assert_eq!(chorus.packs()[0].lines, 0);
        assert_one_warning(&warnings, &file, "text is empty");
    }

    #[test]
    fn skips_invalid_weight() {
        for weight in ["0.0", "-1.0", "nan", "inf"] {
            let packs = Packs::new();
            let file = packs.add(
                "p",
                &format!(
                    "{VOICE}[[lines]]\nvoice = \"drift\"\ntext = \"Hm.\"\nweight = {weight}\n"
                ),
            );

            let (chorus, warnings) = Chorus::load(packs.dir()).unwrap();

            assert!(
                chorus.lines.is_empty(),
                "weight {weight} should be rejected"
            );
            assert_one_warning(&warnings, &file, "weight");
        }
    }

    #[test]
    fn drops_invalid_color() {
        let packs = Packs::new();
        let file = packs.add("p", "[voices.drift]\ncolor = \"purple\"\n");

        let (chorus, warnings) = Chorus::load(packs.dir()).unwrap();

        let voice = chorus.voice("drift").expect("voice still loads");
        assert_eq!(voice.color, None);
        assert_one_warning(&warnings, &file, "color `purple`");
    }

    #[test]
    fn keeps_valid_color() {
        let packs = Packs::new();
        packs.add("p", "[voices.drift]\ncolor = \"#A1b2C3\"\n");
        let (chorus, warnings) = Chorus::load(packs.dir()).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(
            chorus.voice("drift").unwrap().color.as_deref(),
            Some("#A1b2C3")
        );
    }

    #[test]
    fn drops_missing_sound_and_icon() {
        let packs = Packs::new();
        let file = packs.add(
            "p",
            "[voices.drift]\nsound = \"missing.ogg\"\n\
             [voices.ember]\nicon = \"missing.png\"\n\
             [[lines]]\nvoice = \"ember\"\ntext = \"Still here.\"\nsound = \"gone.ogg\"\n",
        );

        let (chorus, warnings) = Chorus::load(packs.dir()).unwrap();

        assert_eq!(chorus.voice("drift").unwrap().sound, None);
        assert_eq!(chorus.voice("ember").unwrap().icon, None);
        assert_eq!(chorus.lines.len(), 1, "line still loads");
        assert_eq!(chorus.lines[0].sound, None);
        assert_eq!(warnings.len(), 3, "{warnings:?}");
        assert!(warnings.iter().all(|w| w.file == file));
        assert!(warnings[0].message.contains("missing.ogg"));
        assert!(warnings[1].message.contains("missing.png"));
        assert!(warnings[2].message.contains("gone.ogg"));
    }

    #[test]
    fn keeps_existing_sound() {
        let packs = Packs::new();
        let dir = packs.dir().join("p");
        packs.add("p", "[voices.drift]\nsound = \"ping.ogg\"\n");
        std::fs::write(dir.join("ping.ogg"), b"").unwrap();

        let (chorus, warnings) = Chorus::load(packs.dir()).unwrap();

        assert!(warnings.is_empty());
        assert_eq!(
            chorus.voice("drift").unwrap().sound,
            Some(dir.join("ping.ogg"))
        );
    }

    #[test]
    fn line_uses_voice_from_other_pack() {
        let packs = Packs::new();
        packs.add("a-voices", VOICE);
        packs.add(
            "b-lines",
            "[[lines]]\nvoice = \"drift\"\ntext = \"Borrowed voice.\"\n",
        );

        let (chorus, warnings) = Chorus::load(packs.dir()).unwrap();

        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(chorus.lines.len(), 1);
        assert_eq!(chorus.packs()[1].lines, 1);
    }

    #[test]
    fn first_voice_definition_wins() {
        let packs = Packs::new();
        packs.add("a", "[voices.drift]\ndisplay = \"FIRST\"\n");
        let second = packs.add("b", "[voices.drift]\ndisplay = \"SECOND\"\n");

        let (chorus, warnings) = Chorus::load(packs.dir()).unwrap();

        assert_eq!(chorus.voice("drift").unwrap().display, "FIRST");
        assert_eq!(chorus.packs()[0].voices, ["drift"]);
        assert!(chorus.packs()[1].voices.is_empty());
        assert_one_warning(&warnings, &second, "already defined by pack `a`");
    }

    #[test]
    fn summarises_packs() {
        let packs = Packs::new();
        packs.add(
            "one",
            "[pack]\nname = \"Desk Voices\"\n\
             [voices.drift]\n[voices.ember]\n\
             [[lines]]\nvoice = \"drift\"\ntext = \"a\"\n\
             [[lines]]\nvoice = \"ember\"\ntext = \"b\"\n",
        );
        packs.add("two", "[[lines]]\nvoice = \"drift\"\ntext = \"c\"\n");

        let (chorus, _) = Chorus::load(packs.dir()).unwrap();

        assert_eq!(
            chorus.packs(),
            [
                PackSummary {
                    name: "Desk Voices".into(),
                    dir: packs.dir().join("one"),
                    voices: vec!["drift".into(), "ember".into()],
                    lines: 2,
                },
                PackSummary {
                    name: "two".into(),
                    dir: packs.dir().join("two"),
                    voices: vec![],
                    lines: 1,
                },
            ]
        );
    }

    #[test]
    fn warning_display_names_file() {
        let w = warn(Path::new("/p/pack.toml"), "oops".into());
        assert_eq!(w.to_string(), "/p/pack.toml: oops");
    }
}
