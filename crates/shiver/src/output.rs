//! The interface shared by every way of showing a line.

use std::fmt::Display;

use shiver_core::Interjection;

/// Shows an interjection somewhere: the terminal, a notification, a sound.
pub trait Output {
    fn emit(&self, interjection: &Interjection) -> anyhow::Result<()>;
}

/// Prints a warning to stderr.
pub fn warn(message: impl Display) {
    eprintln!("shiver: warning: {message}");
}
