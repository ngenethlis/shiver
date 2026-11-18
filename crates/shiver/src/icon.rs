//! The default notification icon: the Signal Flame logo, compiled in and
//! written to the data folder on first use.

use std::io;
use std::path::{Path, PathBuf};

use crate::output;

/// `assets/logo/shiver-icon-128.png`.
const ICON: &[u8] = include_bytes!("../../../assets/logo/shiver-icon-128.png");

/// Makes sure `<data_dir>/shiver/icons/shiver.png` holds the icon and returns
/// its path. The file is only written when missing or different.
pub fn ensure(data_dir: &Path) -> io::Result<PathBuf> {
    let dir = data_dir.join("shiver").join("icons");
    let path = dir.join("shiver.png");
    if std::fs::read(&path).is_ok_and(|bytes| bytes == ICON) {
        return Ok(path);
    }
    std::fs::create_dir_all(&dir)?;
    std::fs::write(&path, ICON)?;
    Ok(path)
}

/// The installed default icon, or `None` (with a warning) if it can't be
/// installed.
pub fn default_icon() -> Option<PathBuf> {
    let result = dirs::data_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no data folder"))
        .and_then(|dir| ensure(&dir));
    match result {
        Ok(path) => Some(path),
        Err(e) => {
            output::warn(format!("can't install the shiver icon: {e}"));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use super::*;

    #[test]
    fn embedded_icon_is_png() {
        assert_eq!(&ICON[..8], b"\x89PNG\r\n\x1a\n");
        // IHDR: width and height as big-endian u32 at bytes 16..24.
        let width = u32::from_be_bytes(ICON[16..20].try_into().unwrap());
        let height = u32::from_be_bytes(ICON[20..24].try_into().unwrap());
        assert_eq!((width, height), (128, 128));
    }

    #[test]
    fn writes_icon() {
        let dir = tempfile::tempdir().unwrap();
        let path = ensure(dir.path()).unwrap();
        assert_eq!(path, dir.path().join("shiver/icons/shiver.png"));
        assert_eq!(std::fs::read(&path).unwrap(), ICON);
    }

    #[test]
    fn keeps_matching_icon() {
        let dir = tempfile::tempdir().unwrap();
        let path = ensure(dir.path()).unwrap();
        let old = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(old)
            .unwrap();
        ensure(dir.path()).unwrap();
        assert_eq!(std::fs::metadata(&path).unwrap().modified().unwrap(), old);
    }

    #[test]
    fn replaces_different_icon() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("shiver/icons/shiver.png");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"old icon").unwrap();
        ensure(dir.path()).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), ICON);
    }
}
