//! The app catalog: the apps BD Launcher knows about.
//!
//! The catalog is data, not code. It is fetched from Firebase (Realtime Database), cached on
//! disk and read leniently: every field is optional, a field of the wrong type reads as missing,
//! and one bad entry never spoils the others. Nothing is built into the binary, so publishing an
//! app is a matter of writing to the catalog, not of shipping a new launcher.
//!
//! The catalog carries everything the launcher needs about an app in one place: how it is called,
//! what colour it is, where its icon is, and the files of its latest release (with their
//! checksums). One request therefore tells the launcher the whole list and every version in it.
//!
//! Text here is English, the source the UI translates from: each `tagline` has a catalog row
//! keyed by the app id (`tr_ctx(app.id, app.tagline)` in `ui-egui`).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::release::{self, Asset, Release};
use crate::version;

/// The Firebase project the catalog and the bucket live in.
pub const PROJECT: &str = "bdesk-bdtecnologia";

/// The Firebase Realtime Database document the catalog is read from. A deployment can point
/// somewhere else with the `BD_LAUNCHER_CATALOG_URL` environment variable.
pub const CATALOG_URL: &str = "https://bdesk-bdtecnologia-default-rtdb.firebaseio.com/catalog.json";

/// The Realtime Database host the catalog is read from: this project's own database.
/// Anything else — notably a database left behind on a previous project — is refused,
/// so only this project's rules decide what the launcher sees.
pub const CATALOG_HOST: &str = "bdesk-bdtecnologia-default-rtdb.firebaseio.com";

/// The Firebase Storage bucket the icons, the pictures and the downloads live in. A deployment
/// can point somewhere else with the `BD_LAUNCHER_STORAGE` environment variable.
pub const STORAGE_BUCKET: &str = "bdesk-bdtecnologia.firebasestorage.app";

/// The public address of a file in the storage bucket.
pub fn storage_url(bucket: &str, path: &str) -> String {
    format!("https://storage.googleapis.com/{bucket}/{path}")
}

/// The launcher's own id in the catalog: the entry its own update is read from.
pub const LAUNCHER_ID: &str = "bd-launcher";

/// BD Launcher itself, described like an app so it can update itself with the same machinery
/// (asset names `bd-launcher-<ver>-…`, bundle id `com.bdtecnologia.bd-launcher`).
pub fn launcher() -> &'static App {
    static LAUNCHER: std::sync::OnceLock<App> = std::sync::OnceLock::new();
    LAUNCHER.get_or_init(|| App {
        id: LAUNCHER_ID.to_string(),
        name: "BD Launcher".to_string(),
        tagline: "Installs and updates the BD Apps".to_string(),
        color: BRAND_BLUE,
        ink_light: BRAND_INK_LIGHT,
        ink_dark: BRAND_INK_DARK,
        icon: String::new(),
        website: WEBSITE_URL.to_string(),
        github: "daviloureiro/bd-launcher".to_string(),
        package_id: format!("{PACKAGE_PREFIX}.{LAUNCHER_ID}"),
        categories: "Utility;".to_string(),
        wm_class: String::new(),
        former_ids: Vec::new(),
        former_names: Vec::new(),
        details: None,
        release: None,
    })
}

/// The package id prefix every app gets when the catalog doesn't name one.
pub const PACKAGE_PREFIX: &str = "com.bdtecnologia";

/// BD's website.
pub const WEBSITE_URL: &str = "https://bdtecnolog.ia.br";
/// BD on GitHub: where the apps' source lives.
pub const GITHUB_URL: &str = "https://github.com/daviloureiro";

/// BD's social accounts: (service, URL). Shown as small icons in the footer and in Settings,
/// next to the website and the GitHub link.
///
/// The list the catalog names comes first; this one is the fallback. The names are the ones
/// [`icons::social`](crate::icons) knows: `X`, `Instagram`, `LinkedIn`, `YouTube`, `TikTok`,
/// `WhatsApp`.
pub const SOCIAL: [(&str, &str); 3] =
    [("WhatsApp", WHATSAPP_URL), ("Instagram", "https://instagram.com/bdtecnolog.ia"), ("LinkedIn", "https://linkedin.com")];

/// BD's WhatsApp: the number a message goes to, with the text already written.
pub const WHATSAPP_URL: &str = "https://wa.me/5511963080423";
/// The same number, as it is written for a person to read.
pub const PHONE: &str = "(11) 96308-0423";

/// The brand blue, when the catalog doesn't say: the colour of the BD mark.
pub const BRAND_BLUE: [u8; 3] = [0x18, 0x76, 0xff];
/// The brand blue, for text on a light background.
pub const BRAND_INK_LIGHT: [u8; 3] = [0x0b, 0x51, 0xb8];
/// The brand blue, for text on a dark background.
pub const BRAND_INK_DARK: [u8; 3] = [0x7a, 0xb5, 0xff];

/// One app in the catalog.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct App {
    /// Machine name: the download's stem, the folder name, the package id's last part.
    pub id: String,
    /// Display name.
    pub name: String,
    /// What it's for, in a few words (English; the UI translates it by app id).
    pub tagline: String,
    /// Brand colour (sRGB), for surfaces.
    pub color: [u8; 3],
    /// The brand colour for text on a light background.
    pub ink_light: [u8; 3],
    /// The brand colour for text on a dark background.
    pub ink_dark: [u8; 3],
    /// The icon: a path in the storage bucket, or an absolute `https` address.
    pub icon: String,
    /// The app's page on BD's website.
    pub website: String,
    /// `owner/repo` on GitHub, when the app's source lives there.
    pub github: String,
    /// The package id: the macOS bundle id, the Flatpak id and the Linux desktop-file id.
    pub package_id: String,
    /// The menu categories a Linux entry lists.
    pub categories: String,
    /// The window class the app's own windows report, when it isn't the package id.
    pub wm_class: String,
    /// Older names its release files may carry.
    pub former_ids: Vec<String>,
    /// Older display names its installers registered.
    pub former_names: Vec<String>,
    /// The app's page: the story, the features and the pictures.
    pub details: Option<AppDetails>,
    /// The latest release, as the catalog describes it.
    pub release: Option<CatalogRelease>,
}

impl App {
    /// The app's package id: the one the catalog names, or the brand's default.
    pub fn bundle_id(&self) -> String {
        if self.package_id.is_empty() { format!("{PACKAGE_PREFIX}.{}", self.id) } else { self.package_id.clone() }
    }

    /// The app's page on BD's website: the one the catalog names, or the default.
    pub fn website(&self) -> String {
        if self.website.is_empty() { format!("{WEBSITE_URL}/apps/{}", self.id) } else { self.website.clone() }
    }

    /// The app's repository on GitHub, when it has one.
    pub fn repo_url(&self) -> Option<String> {
        (!self.github.is_empty()).then(|| format!("https://github.com/{}", self.github))
    }

    /// The app's releases on GitHub, when it has a repository.
    pub fn releases_url(&self) -> Option<String> {
        self.repo_url().map(|u| format!("{u}/releases"))
    }

    /// Asset stems this app's releases may use, current first.
    pub fn stems(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.id.as_str()).chain(self.former_ids.iter().map(String::as_str))
    }

    /// Display names this app's installers may have registered, current first.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.name.as_str()).chain(self.former_names.iter().map(String::as_str))
    }

    /// The icon's address, resolved against the storage base.
    pub fn icon_url(&self, storage: &str) -> Option<String> {
        absolute(&self.icon, storage)
    }

    /// Where the picture `key` of this app is published, or `None` if the page has no such
    /// picture. The only way to turn a request into an address: callers never build one from
    /// their own text.
    pub fn screenshot_url(&self, key: &str, storage: &str) -> Option<String> {
        let shot = self.details.as_ref()?.shots.iter().find(|s| s.key == key)?;
        absolute(&format!("shots/{}/{}.jpg", self.id, shot.key), storage)
    }

    /// The latest release, ready to install from.
    pub fn release(&self, storage: &str) -> Option<Release> {
        self.release.as_ref().and_then(|r| r.to_release(storage))
    }
}

/// The whole catalog: the apps, the launcher's own release and the news.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Catalog {
    /// The apps, in the order they are shown.
    pub apps: Vec<App>,
    /// The launcher's own latest release, so the launcher can update itself.
    pub launcher: Option<CatalogRelease>,
    /// News shown inline, until dismissed.
    pub announcements: Vec<Notice>,
    /// Alerts shown as popups, until acknowledged.
    pub alerts: Vec<Notice>,
    /// BD's social accounts, as the catalog names them. The built-in list is the fallback.
    pub social: Vec<SocialLink>,
    /// Where the icons, the pictures and the downloads live.
    pub storage: String,
}

impl Catalog {
    /// The catalog of an installation that has not heard from Firebase yet: no apps.
    pub fn builtin() -> Self {
        Self::default()
    }

    /// The app with this id.
    pub fn by_id(&self, id: &str) -> Option<&App> {
        self.apps.iter().find(|a| a.id == id)
    }

    /// Reads the catalog. `None` only when the answer isn't a JSON object at all.
    pub fn parse(json: &str) -> Option<Self> {
        let value: Value = serde_json::from_str(json).ok()?;
        let root = value.as_object()?;
        let mut catalog = Catalog { storage: text(root.get("storage")).unwrap_or_default(), ..Catalog::default() };
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for item in entries(root.get("apps")) {
            // An id twice would make two cards fight over one install: the first wins.
            if let Some(app) = App::from_value(&item)
                && seen.insert(app.id.clone())
            {
                catalog.apps.push(app);
            }
        }
        if let Some(release) = root.get("launcher").and_then(CatalogRelease::from_value) {
            catalog.launcher = Some(release);
        }
        for item in array(root.get("announcements")) {
            if let Some(notice) = Notice::from_value(&item, "announcement") {
                catalog.announcements.push(notice);
            }
        }
        for item in array(root.get("alerts")) {
            if let Some(notice) = Notice::from_value(&item, "alert") {
                catalog.alerts.push(notice);
            }
        }
        for item in entries(root.get("social")) {
            if let Some(link) = SocialLink::from_value(&item) {
                catalog.social.push(link);
            }
        }
        catalog.apps.truncate(MAX_APPS);
        catalog.announcements.truncate(MAX_NOTICES);
        catalog.alerts.truncate(MAX_NOTICES);
        catalog.social.truncate(MAX_SOCIAL);
        for release in catalog.launcher.as_mut().into_iter().chain(catalog.apps.iter_mut().filter_map(|a| a.release.as_mut())) {
            release.assets.truncate(MAX_ASSETS);
        }
        for app in &mut catalog.apps {
            if let Some(details) = app.details.as_mut() {
                details.shots.truncate(MAX_SHOTS);
            }
        }
        Some(catalog)
    }

    /// The launcher's own release, ready to install from.
    pub fn launcher_release(&self) -> Option<Release> {
        self.launcher.as_ref().and_then(|r| r.to_release(&self.storage))
    }

    /// The catalog with another storage bucket, for a deployment that keeps its files
    /// somewhere else.
    pub fn with_storage(mut self, storage: &str) -> Self {
        let storage = storage.trim().trim_end_matches('/');
        if !storage.is_empty() {
            self.storage = storage.to_string();
        }
        self
    }
}

/// A small catalog with two made-up apps, for the tests and for `cargo xtask local serve`.
pub fn sample() -> Catalog {
    let json = serde_json::json!({
        "storage": format!("https://storage.googleapis.com/{STORAGE_BUCKET}"),
        "apps": [
            {
                "id": "exemplo",
                "name": "Exemplo",
                "tagline": "Um app de exemplo",
                "color": "#0071ff",
                "icon": "icons/exemplo.png",
                "categories": "Utility;",
                "details": {
                    "headline": "Um app de exemplo.",
                    "about": "O primeiro parágrafo.\n\nO segundo.",
                    "features": [{"title": "Destaque", "text": "O que o app faz bem."}],
                    "shots": [{"key": "demo", "caption": "A tela principal"}, {"key": "ajustes", "caption": "Os ajustes"}],
                    "platforms": "macOS, Windows, Linux, web"
                },
                "release": {
                    "version": "1.2.0",
                    "published_at": 1_791_500_000_i64,
                    "notes": "Notas da versão.",
                    "base": "apps/exemplo/1.2.0/",
                    "assets": [
                        { "name": "exemplo-1.2.0-windows-x64.msi", "size": 123, "sha256": "8a5edab282632443219e051e4ade2d1d5bbc671c781051bf1437897cbdfea0f1" },
                        { "name": "exemplo-1.2.0-linux-x86_64.AppImage", "size": 456, "sha256": "8a5edab282632443219e051e4ade2d1d5bbc671c781051bf1437897cbdfea0f1" }
                    ]
                }
            },
            {
                "id": "amostra",
                "name": "Amostra",
                "tagline": "Outro app de exemplo",
                "color": "#00aee6",
                "icon": "icons/amostra.png"
            }
        ]
    });
    Catalog::parse(&serde_json::to_string(&json).unwrap_or_default()).unwrap_or_default()
}

/// A catalog with the Craft apps the tests work with: every app the old suite had, with a page
/// of its own. It is the shape a real catalog has, so the tests exercise the same paths.
pub fn fixtures() -> Catalog {
    const APPS: [(&str, &str, &str, &str, &str, &str); 13] = [
        ("photocraft", "PhotoCraft", "Image editing", "#2f7bf5", "", ""),
        ("vectorcraft", "VectorCraft", "Vector illustration", "#e8573f", "", ""),
        ("filmcraft", "FilmCraft", "Video editing, color and sound", "#8b5cf6", "", ""),
        ("lightcraft", "LightCraft", "Photo library and raw development", "#f2a516", "", ""),
        ("pdfcraft", "PdfCraft", "Reading, organizing and protecting PDFs", "#12a58a", "printcraft", "PrintCraft"),
        ("effectcraft", "EffectCraft", "Motion graphics and visual effects", "#e0368f", "", ""),
        ("designcraft", "DesignCraft", "Page layout and publishing", "#7bb51c", "", ""),
        ("wordcraft", "WordCraft", "Writing and document design", "#3f5fe0", "", ""),
        ("deckcraft", "DeckCraft", "Presentations and slide shows", "#f07a1e", "", ""),
        ("gridcraft", "GridCraft", "Spreadsheets and data", "#1f9d55", "", ""),
        ("cadcraft", "CADCraft", "Computer-aided design and drafting", "#3ea6e8", "", ""),
        ("soundcraft", "SoundCraft", "Recording, editing and mixing audio", "#1fa5c9", "", ""),
        ("figcraft", "FigCraft", "Interface design and prototyping", "#02da96", "", ""),
    ];
    let mut apps = Vec::with_capacity(APPS.len());
    for (id, name, tagline, color, former_id, former_name) in APPS {
        let app = App {
            id: id.to_string(),
            name: name.to_string(),
            tagline: tagline.to_string(),
            color: color_of(color),
            ink_light: BRAND_INK_LIGHT,
            ink_dark: BRAND_INK_DARK,
            icon: format!("icons/{id}.png"),
            website: String::new(),
            github: format!("bd-tecnologia/{id}"),
            package_id: format!("{PACKAGE_PREFIX}.{id}"),
            categories: "Utility;".to_string(),
            wm_class: String::new(),
            former_ids: if former_id.is_empty() { Vec::new() } else { vec![former_id.to_string()] },
            former_names: if former_name.is_empty() { Vec::new() } else { vec![former_name.to_string()] },
            details: None,
            release: None,
        };
        let mut app = app;
        app.details = Some(page(&app));
        apps.push(app);
    }
    Catalog { storage: format!("https://storage.googleapis.com/{STORAGE_BUCKET}"), apps, ..Catalog::default() }
}

/// A page for the fixtures: the copy the tests were written against.
fn page(app: &App) -> AppDetails {
    let copy = crate::fixture_pages::PAGES.iter().find(|p| p.id == app.id);
    AppDetails {
        id: app.id.clone(),
        headline: copy.map(|p| p.headline.to_string()).unwrap_or_default(),
        about: copy.map(|p| p.about.to_string()).unwrap_or_default(),
        features: copy
            .map(|p| p.features.iter().map(|(title, text)| Feature { title: (*title).to_string(), text: (*text).to_string() }).collect())
            .unwrap_or_default(),
        shots: copy
            .map(|p| p.shots.iter().map(|(key, caption)| Shot { key: (*key).to_string(), caption: (*caption).to_string() }).collect())
            .unwrap_or_default(),
        platforms: "macOS, Windows, Linux, FreeBSD, web".to_string(),
    }
}

/// A colour from a hex string.
fn color_of(hex: &str) -> [u8; 3] {
    let value = color(Some(&serde_json::Value::String(hex.to_string())));
    value.unwrap_or(BRAND_BLUE)
}

/// A release as the catalog describes it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogRelease {
    /// The version, without a leading `v`.
    pub version: String,
    /// The tag, when the release has one.
    pub tag: String,
    /// Unix seconds.
    pub published_at: Option<i64>,
    /// Release notes (Markdown).
    pub notes: String,
    pub prerelease: bool,
    /// Where the files are: a path in the storage bucket, or an absolute `https` address.
    pub base: String,
    /// The files.
    pub assets: Vec<CatalogAsset>,
}

impl CatalogRelease {
    /// The release, ready to install from. `None` when the version doesn't read as one.
    pub fn to_release(&self, storage: &str) -> Option<Release> {
        let parsed = version::parse(&self.version)?;
        let base = absolute(&self.base, storage)?;
        let assets = self
            .assets
            .iter()
            .filter_map(|a| {
                if !release::valid_file_name(&a.name) {
                    return None;
                }
                let url = format!("{base}{}", a.name);
                if !release::safe_download(&url) {
                    return None;
                }
                Some(Asset { name: a.name.clone(), size: a.size, url, sha256: a.sha256.clone().filter(|h| release::is_sha256(h)) })
            })
            .collect();
        Some(Release {
            version: Some(parsed),
            name: if self.tag.is_empty() { self.version.clone() } else { self.tag.clone() },
            tag: if self.tag.is_empty() { format!("v{}", self.version) } else { self.tag.clone() },
            published_at: self.published_at,
            html_url: String::new(),
            notes: self.notes.clone(),
            prerelease: self.prerelease,
            assets,
        })
    }
}

/// One file of a release.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogAsset {
    /// The file's name, which is also how the launcher recognises the platform and the format.
    pub name: String,
    /// Bytes; 0 when unknown.
    pub size: u64,
    /// Lowercase hex SHA-256.
    pub sha256: Option<String>,
}

/// An announcement (shown inline, dismissible) or an alert (a popup the user acknowledges).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notice {
    /// Stable, so a dismissal sticks across launches and refreshes.
    pub id: String,
    pub title: String,
    pub body: String,
    /// Only `https://` / `http://` links survive parsing.
    pub link: Option<String>,
    /// Unix seconds.
    pub created_at: Option<i64>,
}

/// One of BD's social accounts, as the catalog names it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocialLink {
    /// The service, as [`crate::icons::social`] knows it (`YouTube`, `Instagram`, ...).
    pub name: String,
    /// The account's address.
    pub url: String,
}

impl SocialLink {
    fn from_value(value: &Value) -> Option<Self> {
        let name = text(value.get("name"))?;
        let url = text(value.get("url"))?;
        // Only the services the interface has an icon for, and only real links.
        if name.is_empty() || name.len() > 32 || !url.starts_with("https://") || url.len() > 512 {
            return None;
        }
        Some(Self { name, url })
    }
}

/// The editorial half of an app's page.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppDetails {
    /// The app this page belongs to.
    pub id: String,
    /// One line under the name.
    pub headline: String,
    /// One or two paragraphs, separated by a blank line.
    pub about: String,
    pub features: Vec<Feature>,
    /// The first is the lead picture. All are 8:5.
    pub shots: Vec<Shot>,
    /// Where it runs, for display (`"macOS, Windows, Linux, web"`).
    pub platforms: String,
}

impl AppDetails {
    /// The paragraphs of the story, split on the blank lines.
    pub fn paragraphs(&self) -> Vec<&str> {
        self.about.split("\n\n").map(str::trim).filter(|p| !p.is_empty()).collect()
    }
}

/// One feature: a short name and a sentence.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Feature {
    pub title: String,
    pub text: String,
}

/// A screenshot: `key` names the file to fetch (`<app>/<key>.jpg` in the storage bucket).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shot {
    pub key: String,
    pub caption: String,
}

/// How many apps the catalog may hold.
const MAX_APPS: usize = 200;
/// How many notices of one kind the catalog may hold.
const MAX_NOTICES: usize = 20;
/// The social accounts the catalog may name.
const MAX_SOCIAL: usize = 12;
/// The files of one release the catalog may name: each one is a download the launcher may take.
const MAX_ASSETS: usize = 500;
/// The pictures of one details page the catalog may name: each one is a request the launcher
/// makes when the page is opened.
const MAX_SHOTS: usize = 20;
/// The longest text a field may hold.
const MAX_TEXT: usize = 8000;

// ---- reading the catalog ----------------------------------------------------------------------

impl App {
    fn from_value(value: &Value) -> Option<Self> {
        let id = text(value.get("id"))?;
        if id.is_empty() || id.len() > 64 || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b)) {
            return None;
        }
        let raw_name = text(value.get("name")).unwrap_or_else(|| id.clone());
        // The name becomes a folder name for MSI installs: anything that could escape the
        // folder falls back to the id (which is already validated).
        let name = if is_safe_name(&raw_name) { raw_name } else { id.clone() };
        let details = value.get("details").and_then(|d| AppDetails::from_value(d, &id));
        // Every field that becomes a path, a command line or a URL is validated: an invalid
        // value reads as missing, and the safe default below stands in.
        let website = text(value.get("website")).filter(|u| release::safe_web_link(u)).unwrap_or_default();
        let github = text(value.get("github")).filter(|g| is_safe_repo(g)).unwrap_or_default();
        let package_id = text(value.get("package_id")).filter(|p| is_safe_package_id(p)).unwrap_or_default();
        let categories = text(value.get("categories")).filter(|c| is_safe_categories(c)).unwrap_or_default();
        let wm_class = text(value.get("wm_class")).filter(|w| is_safe_wm_class(w)).unwrap_or_default();
        Some(App {
            id,
            name,
            tagline: text(value.get("tagline")).unwrap_or_default(),
            color: color(value.get("color")).unwrap_or(BRAND_BLUE),
            ink_light: color(value.get("ink_light")).unwrap_or(BRAND_INK_LIGHT),
            ink_dark: color(value.get("ink_dark")).unwrap_or(BRAND_INK_DARK),
            icon: text(value.get("icon")).unwrap_or_default(),
            website,
            github,
            package_id,
            categories,
            wm_class,
            former_ids: strings(value.get("former_ids")),
            former_names: strings(value.get("former_names")),
            details,
            release: value.get("release").and_then(CatalogRelease::from_value),
        })
    }
}

impl AppDetails {
    fn from_value(value: &Value, id: &str) -> Option<Self> {
        let about = text(value.get("about"))?;
        Some(AppDetails {
            id: id.to_string(),
            headline: text(value.get("headline")).unwrap_or_default(),
            about,
            features: array(value.get("features")).iter().filter_map(Feature::from_value).collect(),
            shots: array(value.get("shots")).iter().filter_map(Shot::from_value).collect(),
            platforms: text(value.get("platforms")).unwrap_or_default(),
        })
    }
}

impl Feature {
    fn from_value(value: &Value) -> Option<Self> {
        let title = text(value.get("title"))?;
        Some(Feature { title, text: text(value.get("text")).unwrap_or_default() })
    }
}

impl Shot {
    fn from_value(value: &Value) -> Option<Self> {
        let key = text(value.get("key"))?;
        if !release::valid_file_name(&key) {
            return None;
        }
        Some(Shot { key, caption: text(value.get("caption")).unwrap_or_default() })
    }
}

impl CatalogRelease {
    fn from_value(value: &Value) -> Option<Self> {
        let version = text(value.get("version"))?;
        if version.is_empty() {
            return None;
        }
        Some(CatalogRelease {
            version,
            tag: text(value.get("tag")).unwrap_or_default(),
            published_at: number(value.get("published_at")).map(|n| n as i64),
            notes: text(value.get("notes")).unwrap_or_default(),
            prerelease: value.get("prerelease").and_then(Value::as_bool).unwrap_or(false),
            base: text(value.get("base")).unwrap_or_default(),
            assets: array(value.get("assets")).iter().filter_map(CatalogAsset::from_value).collect(),
        })
    }
}

impl CatalogAsset {
    fn from_value(value: &Value) -> Option<Self> {
        let name = text(value.get("name"))?;
        Some(CatalogAsset { name, size: number(value.get("size")).unwrap_or(0), sha256: text(value.get("sha256")) })
    }
}

impl Notice {
    fn from_value(value: &Value, kind: &str) -> Option<Self> {
        let title = text(value.get("title")).unwrap_or_default();
        let body = text(value.get("body")).unwrap_or_default();
        if title.trim().is_empty() && body.trim().is_empty() {
            return None;
        }
        let created_at = number(value.get("created_at")).map(|n| n as i64);
        // Notices are opened in the system browser: `https` only, so a catalog can never
        // send the user to a plaintext page.
        let link = text(value.get("link")).filter(|l| l.starts_with("https://") && l.len() <= 2000 && !l.chars().any(char::is_whitespace));
        Some(Notice {
            id: text(value.get("id")).unwrap_or_else(|| format!("{kind}:{}:{}", created_at.unwrap_or_default(), title.trim())),
            title: title.trim().to_string(),
            body: body.trim().to_string(),
            link,
            created_at,
        })
    }
}

/// A text field, trimmed and capped.
fn text(value: Option<&Value>) -> Option<String> {
    let s = value?.as_str()?;
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.chars().take(MAX_TEXT).collect())
}

/// Whether `s` is a safe display name: no control characters, no path separators, and
/// never `.` or `..` (it becomes a folder name for MSI installs).
fn is_safe_name(s: &str) -> bool {
    !s.is_empty() && s.len() <= 128 && !s.chars().any(|c| c.is_control() || matches!(c, '/' | '\\')) && s != "." && s != ".."
}

/// Whether `s` is a safe package id (`com.bdtecnologia.<app>`): dot-separated labels of
/// letters, digits, `-` and `_`. Anything else (notably `/`, `\` and `..`) is rejected, so
/// it can never become a path that escapes the folder it is written in.
fn is_safe_package_id(s: &str) -> bool {
    if s.is_empty() || s.len() > 128 || s.starts_with('.') || s.ends_with('.') || s.contains("..") {
        return false;
    }
    let mut labels = s.split('.');
    labels.all(|label| {
        !label.is_empty()
            && label.len() <= 64
            && label.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
            && label.bytes().next().is_some_and(|b| b.is_ascii_alphanumeric())
    })
}

/// Whether `s` is a safe menu category list (`Utility;Photo;`): letters, digits, `-` and `;`.
fn is_safe_categories(s: &str) -> bool {
    !s.is_empty() && s.len() <= 256 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b"-;".contains(&b))
}

/// Whether `s` is a safe window class: letters, digits, `.`, `-` and `_`.
fn is_safe_wm_class(s: &str) -> bool {
    !s.is_empty() && s.len() <= 128 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b".-_".contains(&b))
}

/// Whether `s` is a safe `owner/repo` reference on GitHub.
fn is_safe_repo(s: &str) -> bool {
    let Some((owner, repo)) = s.split_once('/') else { return false };
    let part = |p: &str| !p.is_empty() && p.len() <= 64 && p.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b));
    part(owner) && part(repo) && !s.contains("..")
}

/// A list of strings.
fn strings(value: Option<&Value>) -> Vec<String> {
    array(value).iter().filter_map(|v| v.as_str().map(|s| s.trim().to_string())).filter(|s| !s.is_empty()).take(20).collect()
}

/// A list, or nothing.
fn array(value: Option<&Value>) -> Vec<Value> {
    value.and_then(Value::as_array).cloned().unwrap_or_default()
}

/// The entries of a list that may also be an object (the Realtime Database's shape for a list).
fn entries(value: Option<&Value>) -> Vec<Value> {
    match value {
        Some(Value::Array(items)) => items.clone(),
        Some(Value::Object(map)) => map.values().cloned().collect(),
        _ => Vec::new(),
    }
}

/// A number.
fn number(value: Option<&Value>) -> Option<u64> {
    value.and_then(Value::as_u64)
}

/// A colour: `"#0071ff"` or `[0, 113, 255]`.
fn color(value: Option<&Value>) -> Option<[u8; 3]> {
    let value = value?;
    if let Some(s) = value.as_str() {
        let hex = s.trim().trim_start_matches('#');
        if hex.len() == 6 && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            let n = u32::from_str_radix(hex, 16).ok()?;
            return Some([(n >> 16) as u8, (n >> 8) as u8, n as u8]);
        }
        return None;
    }
    let items = value.as_array()?;
    if items.len() != 3 {
        return None;
    }
    let mut out = [0u8; 3];
    for (i, item) in items.iter().enumerate() {
        out[i] = u8::try_from(item.as_u64()?).ok()?;
    }
    Some(out)
}

/// Whether downloads may be taken from the host of `url`: the storage service, the apps'
/// own GitHub repositories (icons and pictures live beside the code, and the web app
/// already loads them from there), GitHub release downloads (the free-tier publishing
/// path), or the developer's own machine in a `local-mirror` build. The catalog names
/// the host; without this, any `https` host on the internet would do.
pub fn storage_host_allowed(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://")) else { return false };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = authority.split('@').next_back().unwrap_or_default().split(':').next().unwrap_or_default();
    if host.is_empty() {
        return false;
    }
    if host == "storage.googleapis.com"
        || host.ends_with(".googleapis.com")
        || host.ends_with(".firebasestorage.app")
        || host == "raw.githubusercontent.com"
    {
        return true;
    }
    // Release assets only — versioned, immutable files — rather than all of github.com.
    if host == "github.com" && rest.split(['?', '#']).next().unwrap_or_default().contains("/releases/download/") {
        return true;
    }
    cfg!(feature = "local-mirror") && matches!(host, "127.0.0.1" | "localhost" | "[::1]")
}

/// Whether the catalog may be read from `url`: this project's Realtime Database (or the
/// developer's own machine in a `local-mirror` build). Pinned to the project's own host:
/// any other database — even another `*.firebaseio.com` one, such as a previous project's —
/// is refused, so a catalog left behind elsewhere can never steer this build.
pub fn catalog_host_allowed(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else {
        return cfg!(feature = "local-mirror")
            && (url.starts_with("http://127.0.0.1") || url.starts_with("http://localhost") || url.starts_with("http://[::1]"));
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = authority.split('@').next_back().unwrap_or_default().split(':').next().unwrap_or_default().to_ascii_lowercase();
    host == CATALOG_HOST
}

/// A path in the storage bucket, turned into an address; an address is left alone.
fn absolute(path: &str, storage: &str) -> Option<String> {
    let path = path.trim();
    if path.is_empty() {
        return None;
    }
    let url = if path.starts_with("https://") || path.starts_with("http://") {
        path.to_string()
    } else {
        let base = storage.trim().trim_end_matches('/');
        if base.is_empty() {
            return None;
        }
        // A base that is already an address is used as it is; a bare bucket name gets the
        // storage service's address.
        let base = if base.starts_with("https://") || base.starts_with("http://") {
            base.to_string()
        } else {
            format!("https://storage.googleapis.com/{base}")
        };
        format!("{base}/{}", path.trim_start_matches('/'))
    };
    // Whatever it came out as, it has to be an address a download may come from, on a
    // host the launcher trusts: the storage service, or the developer's own machine in a
    // local-mirror build.
    (release::safe_download(&url) && storage_host_allowed(&url)).then_some(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_catalog_and_ignores_a_bad_entry() {
        let json = r##"{
            "storage": "https://storage.googleapis.com/bucket",
            "apps": [
                {"id": "bom", "name": "Bom", "color": "#1876ff", "icon": "icons/bom.png",
                 "release": {"version": "1.0.0", "base": "apps/bom/1.0.0/",
                             "assets": [{"name": "bom-1.0.0-windows-x64.msi", "size": 7, "sha256": "8a5edab282632443219e051e4ade2d1d5bbc671c781051bf1437897cbdfea0f1"}]}},
                {"id": "id inválido"},
                {"id": "sem-nome"}
            ],
            "announcements": [{"title": "Novidade", "body": "Já chegou."}],
            "alerts": [{"title": "Alerta", "body": "Atualize.", "link": "javascript:alert(1)"}]
        }"##;
        let catalog = Catalog::parse(json).unwrap();
        assert_eq!(catalog.apps.len(), 2);
        let app = catalog.by_id("bom").unwrap();
        assert_eq!(app.name, "Bom");
        assert_eq!(app.color, BRAND_BLUE);
        assert_eq!(app.icon_url(&catalog.storage).as_deref(), Some("https://storage.googleapis.com/bucket/icons/bom.png"));
        assert_eq!(app.bundle_id(), "com.bdtecnologia.bom");
        let release = app.release(&catalog.storage).unwrap();
        assert_eq!(release.version_label(), "1.0.0");
        assert_eq!(release.assets.len(), 1);
        assert_eq!(release.assets[0].url, "https://storage.googleapis.com/bucket/apps/bom/1.0.0/bom-1.0.0-windows-x64.msi");
        assert_eq!(release.assets[0].sha256.as_deref(), Some("8a5edab282632443219e051e4ade2d1d5bbc671c781051bf1437897cbdfea0f1"));
        // The app without a name falls back to its id; the invalid one is dropped.
        assert_eq!(catalog.by_id("sem-nome").unwrap().name, "sem-nome");
        assert_eq!(catalog.announcements.len(), 1);
        // A link that is not a web link is dropped.
        assert_eq!(catalog.alerts[0].link, None);
    }

    #[test]
    fn reads_the_social_accounts_and_drops_the_bad_ones() {
        let json = r##"{
            "social": [
                {"name": "YouTube", "url": "https://www.youtube.com/@bdtecnologia"},
                {"name": "Instagram", "url": "https://www.instagram.com/officialbdstudios"},
                {"name": "Sem endereco"},
                {"name": "Javascript", "url": "javascript:alert(1)"},
                {"name": "Http", "url": "http://example.com"}
            ]
        }"##;
        let catalog = Catalog::parse(json).unwrap();
        // Only the accounts with a name and a real web address survive.
        assert_eq!(catalog.social.len(), 2);
        assert_eq!(catalog.social[0].name, "YouTube");
        assert_eq!(catalog.social[0].url, "https://www.youtube.com/@bdtecnologia");
        assert_eq!(catalog.social[1].name, "Instagram");
    }

    #[test]
    fn survives_anything_the_server_might_send() {
        for junk in ["", "null", "[]", "42", "<html>502 Bad Gateway</html>", "{\"apps\":"] {
            let catalog = Catalog::parse(junk);
            if let Some(catalog) = catalog {
                assert!(catalog.apps.is_empty(), "{junk}");
            }
        }
        // Every field the wrong type: nothing panics, nothing is trusted.
        let weird = r#"{"storage": 7, "apps": "none", "launcher": "no", "announcements": [null, {"title": 3}], "alerts": {"a": 1}}"#;
        let catalog = Catalog::parse(weird).unwrap();
        assert!(catalog.apps.is_empty() && catalog.launcher.is_none() && catalog.announcements.is_empty());
    }

    #[test]
    fn a_download_has_to_be_a_plain_https_address() {
        let json = r#"{"storage": "https://storage.googleapis.com/bucket", "apps": [
            {"id": "x", "release": {"version": "1.0.0", "base": "apps/x/",
                "assets": [{"name": "x-1.0.0-windows-x64.msi"}, {"name": "../escape.msi"}]}}]}"#;
        let catalog = Catalog::parse(json).unwrap();
        let release = catalog.by_id("x").unwrap().release(&catalog.storage).unwrap();
        assert_eq!(release.assets.len(), 1);
        assert!(release.assets[0].url.starts_with("https://storage.googleapis.com/bucket/apps/x/"));
    }

    #[test]
    fn a_storage_base_that_is_an_address_is_used_as_it_is() {
        // A local mirror (the developer's own machine) is an address, not a bucket name: it
        // must not be turned into a storage.googleapis.com address.
        let json = r#"{"storage": "http://127.0.0.1:8787/", "apps": [
            {"id": "x", "icon": "icons/x.png", "release": {"version": "1.0.0", "base": "apps/x/1.0.0/",
             "assets": [{"name": "x-1.0.0-windows-x64.msi", "size": 1, "sha256": "8a5edab282632443219e051e4ade2d1d5bbc671c781051bf1437897cbdfea0f1"}]}}]}"#;
        let catalog = Catalog::parse(json).unwrap();
        let app = catalog.by_id("x").unwrap();
        // The loopback address is only trusted in a build with the `local-mirror` feature.
        let expected = if cfg!(feature = "local-mirror") { Some("http://127.0.0.1:8787/icons/x.png".to_string()) } else { None };
        assert_eq!(app.icon_url(&catalog.storage), expected);
        let release = app.release(&catalog.storage);
        if cfg!(feature = "local-mirror") {
            let release = release.expect("the release is kept in a local-mirror build");
            assert_eq!(release.assets[0].url, "http://127.0.0.1:8787/apps/x/1.0.0/x-1.0.0-windows-x64.msi");
        } else {
            assert!(release.is_none(), "a plain http address is not trusted outside a local-mirror build");
        }
    }

    #[test]
    fn icons_may_live_beside_the_code_on_github() {
        // The free-tier path that needs no Storage bucket: icons and pictures stay in the
        // apps' own repositories, where the web app already loads them from.
        assert!(storage_host_allowed(
            "https://raw.githubusercontent.com/bd-tecnologia/photocraft/main/assets/app-icon/hicolor/128x128/apps/com.bdtecnologia.photocraft.png"
        ));
        assert!(!storage_host_allowed("https://evil.example/icons/x.png"));
        assert!(!storage_host_allowed("https://raw.githubusercontent.com.evil.com/x.png"));
        assert!(!storage_host_allowed("https://raw.githubusercontent.com@evil.com/x.png"));
    }

    #[test]
    fn only_release_assets_may_come_from_github() {
        // `cargo xtask publish --base-url` points the catalog at a GitHub release.
        assert!(storage_host_allowed("https://github.com/daviloureiro/meuapp/releases/download/v1.2.0/meuapp-1.2.0-windows-x64.msi"));
        // Anything else on github.com is refused: issues, lookalikes, userinfo tricks.
        assert!(!storage_host_allowed("https://github.com/daviloureiro/meuapp/issues/1"));
        assert!(!storage_host_allowed("https://github.com.evil.com/daviloureiro/meuapp/releases/download/v1/x.msi"));
        assert!(!storage_host_allowed("https://github.com@evil.com/daviloureiro/meuapp/releases/download/v1/x.msi"));
    }

    #[test]
    fn a_bucket_name_gets_the_storage_service_address() {
        let json = r#"{"storage": "bdesk-bdtecnologia.firebasestorage.app", "apps": [{"id": "x", "icon": "icons/x.png"}]}"#;
        let catalog = Catalog::parse(json).unwrap();
        assert_eq!(
            catalog.by_id("x").unwrap().icon_url(&catalog.storage).as_deref(),
            Some("https://storage.googleapis.com/bdesk-bdtecnologia.firebasestorage.app/icons/x.png")
        );
    }

    #[test]
    fn only_this_projects_database_is_trusted() {
        // The compiled-in address reads.
        assert!(catalog_host_allowed(CATALOG_URL));
        // The host and the address can't drift apart.
        let host = CATALOG_URL.strip_prefix("https://").unwrap().split('/').next().unwrap();
        assert_eq!(host, CATALOG_HOST);
        // A previous project's database, or anyone else's, does not.
        assert!(!catalog_host_allowed("https://bd-tecnologia-default-rtdb.firebaseio.com/catalog.json"));
        assert!(!catalog_host_allowed("https://attacker-default-rtdb.firebaseio.com/catalog.json"));
        assert!(!catalog_host_allowed("https://bdesk-bdtecnologia-default-rtdb.firebaseio.com.evil.com/catalog.json"));
        assert!(!catalog_host_allowed("https://evil.com/x"));
        assert!(!catalog_host_allowed("http://bdesk-bdtecnologia-default-rtdb.firebaseio.com/catalog.json"));
        assert!(!catalog_host_allowed(""));
        // Userinfo resolves to the real host; an explicit port is fine.
        assert!(!catalog_host_allowed("https://bdesk-bdtecnologia-default-rtdb.firebaseio.com@evil.com/catalog.json"));
        assert!(catalog_host_allowed("https://bdesk-bdtecnologia-default-rtdb.firebaseio.com:443/catalog.json"));
    }

    #[test]
    fn a_hostile_package_id_never_becomes_a_path() {
        let json = r#"{"storage": "https://storage.googleapis.com/b", "apps": [
            {"id": "x", "package_id": "../../../../tmp/evil"},
            {"id": "y", "package_id": "com.bdtecnologia.y", "categories": "Utility;Evil\nExec=x;", "wm_class": "x\nExec=y"}
        ]}"#;
        let catalog = Catalog::parse(json).unwrap();
        // Traversal is dropped: the default id is used instead.
        assert_eq!(catalog.by_id("x").unwrap().bundle_id(), "com.bdtecnologia.x");
        // A newline in a menu field is dropped: the empty default stands in (the menu
        // builder falls back to `Utility;`).
        let y = catalog.by_id("y").unwrap();
        assert!(y.categories.is_empty());
        assert!(y.wm_class.is_empty());
        assert!(is_safe_package_id("com.bdtecnologia.app"));
        assert!(!is_safe_package_id("../../../../tmp/evil"));
        assert!(!is_safe_package_id("com.bdtecnologia/app"));
        assert!(!is_safe_categories("Utility;Evil\nExec=x;"));
        assert!(!is_safe_wm_class("x\nExec=y"));
    }

    #[test]
    fn only_https_pages_and_valid_repos_survive() {
        let json = r#"{"storage": "https://storage.googleapis.com/b", "apps": [
            {"id": "x", "website": "file:///etc/passwd", "github": "not a repo"},
            {"id": "y", "website": "https://bdtecnolog.ia.br/apps/y", "github": "bd-tecnologia/y"}
        ]}"#;
        let catalog = Catalog::parse(json).unwrap();
        let x = catalog.by_id("x").unwrap();
        assert!(x.website.is_empty(), "a non-https page is dropped");
        assert!(x.github.is_empty(), "a malformed repo is dropped");
        let y = catalog.by_id("y").unwrap();
        assert_eq!(y.website, "https://bdtecnolog.ia.br/apps/y");
        assert_eq!(y.github, "bd-tecnologia/y");
        assert!(!is_safe_repo("not a repo") && !is_safe_repo("../escape/x"));
    }

    #[test]
    fn the_first_app_with_an_id_wins_and_lists_are_capped() {
        let apps = String::from("{\"id\": \"x\"},{\"id\": \"x\", \"name\": \"Second\"}");
        let json = format!("{{\"storage\": \"https://storage.googleapis.com/b\", \"apps\": [{apps}]}}");
        let catalog = Catalog::parse(&json).unwrap();
        assert_eq!(catalog.apps.len(), 1);
        assert!(catalog.by_id("x").unwrap().name.is_empty() || catalog.by_id("x").unwrap().name == "x");
        let _ = apps;
    }

    #[test]
    fn the_sample_catalog_has_two_apps() {
        let catalog = sample();
        assert_eq!(catalog.apps.len(), 2);
        assert!(catalog.by_id("exemplo").unwrap().release(&catalog.storage).is_some());
        assert!(catalog.by_id("amostra").unwrap().release(&catalog.storage).is_none());
    }
}
