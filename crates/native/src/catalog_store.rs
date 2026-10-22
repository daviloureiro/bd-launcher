//! The catalog on Firebase: read it, keep a copy, hand it over.
//!
//! The catalog is one JSON document in the Realtime Database. It is fetched with an `ETag`, so
//! an unchanged catalog costs one small answer, and a copy is kept on disk for the case where
//! Firebase can't be reached: the launcher then shows the apps it knows about, with the versions
//! it last saw.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use bd_launcher_core::catalog::{CATALOG_URL, Catalog};

use crate::http::{Fetched, Http};

/// The catalog, and where it came from.
#[derive(Clone)]
pub struct CatalogStore {
    http: Option<Http>,
    path: PathBuf,
    etag: Arc<Mutex<Option<String>>>,
}

impl CatalogStore {
    pub fn new(http: Option<Http>, path: PathBuf) -> Self {
        let etag = std::fs::read_to_string(path.with_extension("etag")).ok().map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
        Self { http, path, etag: Arc::new(Mutex::new(etag)) }
    }

    /// Where the catalog is read from: the environment variable, or the compiled-in address.
    /// A `http` address is only accepted on the developer's own machine, and only in a build
    /// with the `local-mirror` feature.
    pub fn url() -> String {
        let from_env = std::env::var("BD_LAUNCHER_CATALOG_URL").ok().map(|u| u.trim().to_string()).filter(|u| !u.is_empty());
        match from_env {
            Some(u) if u.starts_with("https://") => u,
            Some(u) if cfg!(feature = "local-mirror") && u.starts_with("http://127.0.0.1") => u,
            Some(u) if cfg!(feature = "local-mirror") && u.starts_with("http://localhost") => u,
            _ => CATALOG_URL.to_string(),
        }
    }

    /// The catalog, read from Firebase. `Ok(None)` when it hasn't changed since the last read.
    pub fn fetch(&self) -> Result<Option<Catalog>, String> {
        let Some(http) = self.http.as_ref() else { return Err("secure connections aren't available".into()) };
        let url = Self::url();
        let etag = self.etag.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone();
        match http.get_document(&url, etag.as_deref())? {
            Fetched::Body { text, etag } => {
                let catalog = Catalog::parse(&text).ok_or("the catalog isn't readable")?;
                self.save(&catalog, etag.as_deref());
                Ok(Some(catalog))
            }
            Fetched::NotModified => Ok(None),
            Fetched::NotFound => Err("the catalog isn't published yet".into()),
            Fetched::RateLimited { .. } => Err("Firebase is asking us to slow down".into()),
        }
    }

    /// The copy on disk, for when Firebase can't be reached.
    pub fn cached(&self) -> Option<Catalog> {
        let text = std::fs::read_to_string(&self.path).ok()?;
        Catalog::parse(&text)
    }

    /// Keeps the catalog and its `ETag` for the next time.
    pub fn save(&self, catalog: &Catalog, etag: Option<&str>) {
        if let Ok(json) = serde_json::to_string(catalog)
            && let Err(e) = crate::paths::write_atomic(&self.path, json.as_bytes())
        {
            log::warn!("couldn't save the catalog: {e}");
        }
        if let Some(tag) = etag {
            *self.etag.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(tag.to_string());
            let _ = std::fs::write(self.path.with_extension("etag"), tag);
        }
    }
}

/// The file the catalog is kept in.
pub fn catalog_path(cache: &Path) -> PathBuf {
    cache.join("catalog.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_saved_catalog_is_read_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = catalog_path(dir.path());
        let store = CatalogStore::new(None, path);
        let catalog = bd_launcher_core::catalog::sample();
        store.save(&catalog, Some("\"abc\""));
        assert_eq!(store.cached().unwrap().apps.len(), catalog.apps.len());
        assert_eq!(store.cached().unwrap().apps[0].id, catalog.apps[0].id);
    }
}
