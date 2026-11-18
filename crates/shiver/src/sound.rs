//! Playing a line's sound with an external player, without waiting for it.

use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

use shiver_core::Interjection;

use crate::output::{self, Output};

/// Players to try, in order.
const PLAYERS: [&str; 2] = ["pw-play", "paplay"];

/// Plays sounds at `volume` (0.0 to 1.0).
pub struct Sound {
    pub volume: f64,
}

impl Output for Sound {
    fn emit(&self, interjection: &Interjection) -> anyhow::Result<()> {
        if let Some(path) = &interjection.sound
            && let Err(warning) = play(path, self.volume, &mut spawn_detached)
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
    volume: f64,
    spawn: &mut dyn FnMut(&str, &[String]) -> io::Result<()>,
) -> Result<&'static str, String> {
    if !path.is_file() {
        return Err(format!("sound file {} not found", path.display()));
    }
    for player in PLAYERS {
        match spawn(player, &player_args(player, path, volume)) {
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

/// Arguments for `player`: the volume in its own scale, then the file.
fn player_args(player: &str, path: &Path, volume: f64) -> Vec<String> {
    let volume = if player == "paplay" {
        // paplay's linear volume runs from 0 to 65536.
        format!("--volume={}", (volume * 65536.0).round() as u32)
    } else {
        format!("--volume={volume:.3}")
    };
    vec![volume, path.to_string_lossy().into_owned()]
}

/// Spawns `player args` with no stdio and doesn't wait for it.
fn spawn_detached(player: &str, args: &[String]) -> io::Result<()> {
    Command::new(player)
        .args(args)
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
    ) -> impl FnMut(&str, &[String]) -> io::Result<()> + 'a {
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
        let result = play(
            file.path(),
            1.0,
            &mut fake(&["pw-play", "paplay"], &mut calls),
        );
        assert_eq!(result, Ok("pw-play"));
        assert_eq!(calls, ["pw-play"]);
    }

    #[test]
    fn falls_back_to_paplay() {
        let file = sound_file();
        let mut calls = Vec::new();
        let result = play(file.path(), 1.0, &mut fake(&["paplay"], &mut calls));
        assert_eq!(result, Ok("paplay"));
        assert_eq!(calls, ["pw-play", "paplay"]);
    }

    #[test]
    fn warns_when_no_player() {
        let file = sound_file();
        let mut calls = Vec::new();
        let result = play(file.path(), 1.0, &mut fake(&[], &mut calls));
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
            1.0,
            &mut fake(&["pw-play"], &mut calls),
        );
        assert_eq!(
            result,
            Err("sound file /nonexistent/ding.ogg not found".into())
        );
        assert!(calls.is_empty(), "no player started");
    }

    #[test]
    fn passes_volume() {
        let path = Path::new("/s/ding.ogg");
        assert_eq!(
            player_args("pw-play", path, 0.6),
            ["--volume=0.600", "/s/ding.ogg"]
        );
        assert_eq!(
            player_args("paplay", path, 0.5),
            ["--volume=32768", "/s/ding.ogg"]
        );
        assert_eq!(
            player_args("paplay", path, 1.0),
            ["--volume=65536", "/s/ding.ogg"]
        );

        let file = sound_file();
        let mut seen = Vec::new();
        let result = play(file.path(), 0.25, &mut |player: &str, args: &[String]| {
            seen.push((player.to_owned(), args.to_vec()));
            Ok(())
        });
        assert_eq!(result, Ok("pw-play"));
        assert_eq!(seen[0].1[0], "--volume=0.250");
    }
}
