//! Playing a line's sound with an external player, without waiting for it.

use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

use shiver_core::Interjection;

use crate::output::{self, Output};

/// Players to try, in order.
const PLAYERS: [&str; 2] = ["pw-play", "paplay"];

pub struct Sound;

impl Output for Sound {
    fn emit(&self, interjection: &Interjection) -> anyhow::Result<()> {
        if let Some(path) = &interjection.sound
            && let Err(warning) = play(path, &mut spawn_detached)
        {
            output::warn(warning);
        }
        Ok(())
    }
}

/// Starts the first available player on `path`. Returns a warning message if
/// nothing could be played; sound problems never fail the command.
fn play(
    path: &Path,
    spawn: &mut dyn FnMut(&str, &Path) -> io::Result<()>,
) -> Result<&'static str, String> {
    if !path.is_file() {
        return Err(format!("sound file {} not found", path.display()));
    }
    for player in PLAYERS {
        match spawn(player, path) {
            Ok(()) => return Ok(player),
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            Err(e) => return Err(format!("can't start {player}: {e}")),
        }
    }
    Err(format!(
        "no sound player found (tried {})",
        PLAYERS.join(", ")
    ))
}

/// Spawns `player path` with no stdio and doesn't wait for it.
fn spawn_detached(player: &str, path: &Path) -> io::Result<()> {
    Command::new(player)
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fake spawner where only `installed` players exist; records calls.
    fn fake<'a>(
        installed: &'static [&'static str],
        calls: &'a mut Vec<String>,
    ) -> impl FnMut(&str, &Path) -> io::Result<()> + 'a {
        move |player, _| {
            calls.push(player.to_owned());
            if installed.contains(&player) {
                Ok(())
            } else {
                Err(io::ErrorKind::NotFound.into())
            }
        }
    }

    fn sound_file() -> tempfile::NamedTempFile {
        tempfile::NamedTempFile::new().unwrap()
    }

    #[test]
    fn uses_pw_play_first() {
        let file = sound_file();
        let mut calls = Vec::new();
        let result = play(file.path(), &mut fake(&["pw-play", "paplay"], &mut calls));
        assert_eq!(result, Ok("pw-play"));
        assert_eq!(calls, ["pw-play"]);
    }

    #[test]
    fn falls_back_to_paplay() {
        let file = sound_file();
        let mut calls = Vec::new();
        let result = play(file.path(), &mut fake(&["paplay"], &mut calls));
        assert_eq!(result, Ok("paplay"));
        assert_eq!(calls, ["pw-play", "paplay"]);
    }

    #[test]
    fn warns_when_no_player() {
        let file = sound_file();
        let mut calls = Vec::new();
        let result = play(file.path(), &mut fake(&[], &mut calls));
        assert_eq!(
            result,
            Err("no sound player found (tried pw-play, paplay)".into())
        );
    }

    #[test]
    fn warns_when_file_missing() {
        let mut calls = Vec::new();
        let result = play(
            Path::new("/nonexistent/ding.ogg"),
            &mut fake(&["pw-play"], &mut calls),
        );
        assert_eq!(
            result,
            Err("sound file /nonexistent/ding.ogg not found".into())
        );
        assert!(calls.is_empty(), "no player started");
    }
}
