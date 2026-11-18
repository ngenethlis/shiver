//! The history of recent picks, used for cooldowns.
//!
//! Stored at `<state dir>/shiver/history`, one pick per line:
//! `<unix seconds> <id as 16 hex digits> <voice key>`.

use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use shiver_core::{Interjection, Seen};

use crate::output;

/// One recorded pick.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub time: u64,
    pub seen: Seen,
}

/// `~/.local/state/shiver/history`, or `None` if there's no state folder.
pub fn path() -> Option<PathBuf> {
    dirs::state_dir().map(|d| d.join("shiver").join("history"))
}

/// Parses the file, returning the entries and how many lines were malformed.
fn parse(text: &str) -> (Vec<Entry>, usize) {
    let mut entries = Vec::new();
    let mut malformed = 0;
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let mut parts = line.split_whitespace();
        let entry = (|| {
            let time = parts.next()?.parse().ok()?;
            let id = parts.next().filter(|h| h.len() == 16)?;
            let id = u64::from_str_radix(id, 16).ok()?;
            let voice = parts.next()?.to_owned();
            parts.next().is_none().then_some(Entry {
                time,
                seen: Seen { id, voice },
            })
        })();
        match entry {
            Some(e) => entries.push(e),
            None => malformed += 1,
        }
    }
    (entries, malformed)
}

fn render(entries: &[Entry]) -> String {
    entries
        .iter()
        .map(|e| format!("{} {:016x} {}\n", e.time, e.seen.id, e.seen.voice))
        .collect()
}

/// Reads the history. Problems are warnings; the result is then partial or empty.
pub fn load(path: &Path) -> Vec<Entry> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Vec::new(),
        Err(e) => {
            output::warn(format!("can't read history {}: {e}", path.display()));
            return Vec::new();
        }
    };
    let (entries, malformed) = parse(&text);
    if malformed > 0 {
        output::warn(format!(
            "ignored {malformed} malformed line(s) in history {}",
            path.display()
        ));
    }
    entries
}

/// Writes the last `keep` entries, via a temporary file and a rename.
fn save(path: &Path, entries: &[Entry], keep: usize) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let start = entries.len().saturating_sub(keep);
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, render(&entries[start..]))?;
    std::fs::rename(&tmp, path)
}

/// Appends a pick and saves the last `keep`. Failure is a warning, never an error.
pub fn record(path: &Path, mut entries: Vec<Entry>, picked: &Interjection, keep: usize) {
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    entries.push(Entry {
        time,
        seen: Seen {
            id: picked.id,
            voice: picked.voice.key.clone(),
        },
    });
    if let Err(e) = save(path, &entries, keep) {
        output::warn(format!("can't save history {}: {e}", path.display()));
    }
}

/// The `Seen` part of each entry, oldest first.
pub fn seen(entries: &[Entry]) -> Vec<Seen> {
    entries.iter().map(|e| e.seen.clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(n: u64) -> Entry {
        Entry {
            time: 1_700_000_000 + n,
            seen: Seen {
                id: n * 0x1_0000_0001,
                voice: format!("v{n}"),
            },
        }
    }

    #[test]
    fn round_trips() {
        let entries: Vec<_> = (0..3).map(entry).collect();
        let text = render(&entries);
        assert_eq!(text.lines().next(), Some("1700000000 0000000000000000 v0"));
        assert_eq!(parse(&text), (entries, 0));
    }

    #[test]
    fn keeps_last_50() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state/shiver/history");
        let entries: Vec<_> = (0..60).map(entry).collect();
        save(&path, &entries, 50).unwrap();
        let loaded = load(&path);
        assert_eq!(loaded.len(), 50);
        assert_eq!(loaded[0], entry(10));
        assert_eq!(loaded[49], entry(59));
        assert!(!path.with_extension("tmp").exists());
    }

    #[test]
    fn skips_malformed_lines() {
        let good = render(&[entry(1), entry(2)]);
        let text = format!(
            "{good}garbage\n12 nothex!!!!!!!!!!! v\n12 00ff v\n12 {:016x} v extra\n\n",
            7
        );
        let (entries, malformed) = parse(&text);
        assert_eq!(entries, [entry(1), entry(2)]);
        assert_eq!(malformed, 4);
    }

    #[test]
    fn missing_file_is_empty() {
        assert!(load(Path::new("/nonexistent/shiver/history")).is_empty());
    }

    #[test]
    fn record_appends() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history");
        let picked = Interjection {
            id: 0xabc,
            voice: shiver_core::Voice {
                key: "drift".into(),
                display: "DRIFT".into(),
                color: None,
                sound: None,
                icon: None,
            },
            text: "x".into(),
            sound: None,
            icon: None,
        };
        record(&path, vec![entry(1)], &picked, 50);
        let loaded = load(&path);
        assert_eq!(loaded.len(), 2);
        assert_eq!(
            seen(&loaded)[1],
            Seen {
                id: 0xabc,
                voice: "drift".into()
            }
        );
    }
}
