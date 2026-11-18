//! Core of shiver: load packs of voices and lines, and pick a line to show.
//!
//! This crate does no desktop I/O. It only reads the packs folder it is given
//! and checks that asset files exist. Showing a line, playing a sound and
//! finding the user's config folder are left to the caller.

mod chorus;
mod pack;
mod select;

#[cfg(test)]
mod test_util;

pub use chorus::{Chorus, LoadError, PackSummary, Warning};
pub use pack::{Line, Voice};
pub use select::{Filter, Interjection};
