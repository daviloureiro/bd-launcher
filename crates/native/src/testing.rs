//! Fixtures for the tests: a small catalog with the apps they work with.

use bd_launcher_core::catalog::{App, Catalog};

/// The catalog the tests work with.
pub fn catalog() -> &'static Catalog {
    static CATALOG: std::sync::OnceLock<Catalog> = std::sync::OnceLock::new();
    CATALOG.get_or_init(bd_launcher_core::catalog::fixtures)
}

/// One app of it.
pub fn app(id: &str) -> &'static App {
    catalog().by_id(id).unwrap_or_else(|| panic!("no app {id} in the fixtures"))
}
