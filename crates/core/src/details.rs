//! The pictures of an app's details page.
//!
//! The copy and the picture list come from the catalog (`crate::catalog`); the pictures
//! themselves are fetched when a page is opened, so the repository and the binary stay small.
//! They are JPEGs from the storage bucket, capped, cached, and never run.

/// The largest picture the launcher accepts. The real ones are 60 to 150 KB.
pub const MAX_SCREENSHOT_BYTES: usize = 2 << 20;

/// True when `bytes` start like a JPEG file. A captive portal's login page, a proxy's error page
/// or a truncated answer is refused before it is saved or decoded.
pub fn looks_like_jpeg(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xFF, 0xD8, 0xFF])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_jpeg_files_are_accepted() {
        assert!(looks_like_jpeg(&[0xFF, 0xD8, 0xFF, 0xE0, 0, 0x10]));
        assert!(!looks_like_jpeg(b""));
        assert!(!looks_like_jpeg(&[0xFF, 0xD8]));
        assert!(!looks_like_jpeg(b"<!DOCTYPE html>"));
        // A PNG's signature.
        assert!(!looks_like_jpeg(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]));
    }
}
