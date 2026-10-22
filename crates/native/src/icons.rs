//! An app's icon: fetched from the storage bucket, kept in the cache folder, and never run.
//!
//! The catalog says where the icon is; the bytes come with the first request for it and stay on
//! disk, so the icon is there the next time and offline.

use std::path::Path;

use bd_launcher_core::catalog::App;

use crate::http::Http;
use crate::paths::{Paths, write_atomic};

/// The largest icon we accept.
pub const MAX_ICON_BYTES: usize = 4 << 20;

/// The icon saved at `saved`, if there is one and it looks like a picture. A damaged or swapped
/// file is ignored, and the icon is fetched again over it.
fn saved_copy(saved: &Path) -> Option<Vec<u8>> {
    if std::fs::metadata(saved).ok()?.len() > MAX_ICON_BYTES as u64 {
        return None;
    }
    let bytes = std::fs::read(saved).ok()?;
    looks_like_picture(&bytes).then_some(bytes)
}

/// True when `bytes` start like a PNG or a JPEG file: a captive portal's login page or a proxy's
/// error page arrives as a 200, and must not be kept as an icon.
pub fn looks_like_picture(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) || bytes.starts_with(&[0xFF, 0xD8, 0xFF])
}

/// The app's icon, as bytes. The saved copy is used when there is one; otherwise it is fetched
/// once and kept. An app without an icon in the catalog has none.
pub fn fetch(http: Option<&Http>, paths: &Paths, app: &App, storage: &str) -> Result<Vec<u8>, String> {
    let Some(url) = app.icon_url(storage) else { return Err("this app has no icon".into()) };
    let saved = paths.icon(&app.id);
    if let Some(bytes) = saved_copy(&saved) {
        return Ok(bytes);
    }
    let Some(http) = http else { return Err("HTTPS isn't available".into()) };
    let bytes = match http.get_bytes(&url, MAX_ICON_BYTES) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => return Err("this app has no icon".into()),
        Err(why) => return Err(why),
    };
    if !looks_like_picture(&bytes) {
        return Err("the answer isn't a picture".into());
    }
    if let Err(e) = write_atomic(&saved, &bytes) {
        log::warn!("couldn't keep the icon of {}: {e}", app.id);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png() -> Vec<u8> {
        let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        bytes.extend((0..2_000u32).map(|i| (i % 251) as u8));
        bytes
    }

    #[test]
    fn a_damaged_saved_copy_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let saved = dir.path().join("icon.png");
        assert_eq!(saved_copy(&saved), None, "nothing saved");
        std::fs::write(&saved, b"<html>oops</html>").unwrap();
        assert_eq!(saved_copy(&saved), None, "not a picture");
        std::fs::write(&saved, vec![0x89; MAX_ICON_BYTES + 1]).unwrap();
        assert_eq!(saved_copy(&saved), None, "too big");
        std::fs::write(&saved, png()).unwrap();
        assert_eq!(saved_copy(&saved), Some(png()));
    }
}
