//! The browser backend and entry point.

use std::cell::RefCell;
use std::rc::Rc;

use bd_launcher_core::catalog::{App, Catalog};
use bd_launcher_core::details::{MAX_SCREENSHOT_BYTES, looks_like_jpeg};
use bd_launcher_core::protocol::{Backend, Capabilities, Command, Event, LatestRelease, Origin, Source};
use bd_launcher_core::release::{self, Release};
use bd_launcher_core::{Arch, Os, Platform, version};
use bd_launcher_ui_egui::{Host, LauncherUi, Settings, i18n};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

const SETTINGS_KEY: &str = "settings";
/// The largest icon the browser accepts.
const MAX_ICON_BYTES: usize = 4 << 20;

/// Tell the UI the browser's languages (`?lang=ru` in the address first, as `BD_LAUNCHER_LOCALE`
/// does on the desktop) before the first frame, so the Automatic interface language follows the
/// browser; and show the loading text in that language.
fn detect_language() {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };
    let document = match window.document() {
        Some(d) => d,
        None => return,
    };
    let mut tags = Vec::new();

    // ?lang=xx in the URL takes priority.
    if let Ok(href) = window.location().href()
        && let Some(idx) = href.find('?')
    {
        for pair in href[idx + 1..].split('&') {
            if let Some((k, v)) = pair.split_once('=')
                && k == "lang"
                && !v.is_empty()
            {
                tags.push(v.to_string());
            }
        }
    }

    // Then the browser's preferred languages.
    let navigator = window.navigator();
    let langs: js_sys::Array = navigator.languages();
    for i in 0..langs.length() {
        if let Some(v) = langs.get(i).as_string() {
            tags.push(v);
        }
    }
    if let Some(lang) = navigator.language() {
        tags.push(lang);
    }
    i18n::set_system_locales(&tags);

    // Screen readers and spell-check follow the page language: match <html lang>
    // to what was detected (`en` when the browser asks for nothing supported).
    if let Some(root) = document.document_element() {
        let _ = root.set_attribute("lang", i18n::system_lang().code());
    }

    // Update loading text in the detected language.
    if let Some(el) = document.get_element_by_id("launcher_loading") {
        el.set_text_content(Some(i18n::tr(i18n::system_lang(), "Loading BD Launcher…")));
    }
}
pub fn start() {
    eframe::WebLogger::init(log::LevelFilter::Warn).ok();
    // Detect language before starting the UI.
    detect_language();
    wasm_bindgen_futures::spawn_local(async {
        let Some(document) = web_sys::window().and_then(|w| w.document()) else { return };
        let Some(canvas) = document.get_element_by_id("launcher_canvas").and_then(|c| c.dyn_into::<web_sys::HtmlCanvasElement>().ok()) else {
            log::error!("no canvas");
            return;
        };
        let result = eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|cc| {
                    let ctx = cc.egui_ctx.clone();
                    let settings: Settings = cc.storage.and_then(|s| eframe::get_value(s, SETTINGS_KEY)).unwrap_or_default();
                    let backend = WebBackend::new(ctx.clone());
                    Ok(Box::new(WebApp { ui: LauncherUi::new(&ctx, Box::new(backend), settings) }))
                }),
            )
            .await;
        if let Some(loading) = document.get_element_by_id("launcher_loading") {
            loading.remove();
        }
        if let Err(e) = result {
            log::error!("couldn't start: {e:?}");
        }
    });
}

/// The browser: nothing of the desktop's, but it knows whether the visitor asked for fewer animations.
struct WebHost;

impl Host for WebHost {
    fn reduce_motion(&self) -> bool {
        web_sys::window().and_then(|w| w.match_media("(prefers-reduced-motion: reduce)").ok().flatten()).is_some_and(|query| query.matches())
    }
}

struct WebApp {
    ui: LauncherUi,
}

impl eframe::App for WebApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui.logic(ctx, &mut WebHost);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.ui.ui(ui, &mut WebHost);
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        self.ui.raw_input(ctx, raw_input);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, SETTINGS_KEY, &self.ui.settings);
    }
}

struct WebBackend {
    events: Rc<RefCell<Vec<Event>>>,
    ctx: egui::Context,
    platform: Option<Platform>,
    checking: Rc<RefCell<bool>>,
    prereleases: bool,
    /// The catalog, as last read from Firebase.
    catalog: Rc<RefCell<Catalog>>,
    /// Apps whose details pictures are on their way (a second request meanwhile does nothing).
    pictures_busy: Rc<RefCell<std::collections::HashSet<String>>>,
}

impl WebBackend {
    fn new(ctx: egui::Context) -> Self {
        Self {
            events: Rc::default(),
            ctx,
            platform: visitor_platform(),
            checking: Rc::default(),
            prereleases: false,
            catalog: Rc::default(),
            pictures_busy: Rc::default(),
        }
    }

    /// Fetches the pictures of an app's details page, lead first. The browser keeps them in its own
    /// cache, so there is nothing to save here.
    fn fetch_screenshots(&self, app: &str) {
        let (details, storage) = {
            let catalog = self.catalog.borrow();
            let Some(entry) = catalog.apps.iter().find(|a| a.id == app) else { return };
            (entry.details.clone(), catalog.storage.clone())
        };
        let Some(details) = details else { return };
        if !self.pictures_busy.borrow_mut().insert(details.id.clone()) {
            return;
        }
        let (events, ctx, busy) = (self.events.clone(), self.ctx.clone(), self.pictures_busy.clone());
        wasm_bindgen_futures::spawn_local(async move {
            // Once the network has failed the rest would fail the same way: say so at once.
            let mut offline: Option<String> = None;
            for shot in &details.shots {
                let url = absolute(&format!("screenshots/{}/{}.jpg", details.id, shot.key), &storage);
                let result = match (&offline, url) {
                    (Some(why), _) => Err(why.clone()),
                    (None, Some(url)) => picture(&url).await.map_err(|failure| match failure {
                        PictureError::Network(why) => {
                            offline = Some(why.clone());
                            why
                        }
                        PictureError::Picture(why) => why,
                    }),
                    (None, None) => Err("this page has no such picture".to_string()),
                };
                events.borrow_mut().push(Event::Screenshot { app: details.id.clone(), key: shot.key.clone(), result });
                ctx.request_repaint();
            }
            busy.borrow_mut().remove(&details.id);
        });
    }

    /// Fetches an app's icon.
    fn fetch_icon(&self, app: &str) {
        let (icon, storage) = {
            let catalog = self.catalog.borrow();
            let Some(entry) = catalog.apps.iter().find(|a| a.id == app) else { return };
            (entry.icon_url(&catalog.storage), catalog.storage.clone())
        };
        let Some(url) = icon else { return };
        let (events, ctx) = (self.events.clone(), self.ctx.clone());
        let id = app.to_string();
        wasm_bindgen_futures::spawn_local(async move {
            let result = icon_bytes(&url).await;
            events.borrow_mut().push(Event::Icon { app: id, result });
            ctx.request_repaint();
        });
        let _ = storage;
    }

    fn check(&self) {
        if self.checking.replace(true) {
            return;
        }
        let events = self.events.clone();
        let ctx = self.ctx.clone();
        let checking = self.checking.clone();
        let catalog = self.catalog.clone();
        let pre = self.prereleases;
        wasm_bindgen_futures::spawn_local(async move {
            events.borrow_mut().push(Event::CheckStarted);
            let read = read_catalog().await;
            let current = match read {
                Ok(c) => {
                    *catalog.borrow_mut() = c.clone();
                    Some(c)
                }
                Err(why) => {
                    log::warn!("the catalog could not be read: {why}");
                    let saved = catalog.borrow().clone();
                    if saved.apps.is_empty() {
                        events.borrow_mut().push(Event::CatalogFailed(why));
                    }
                    Some(saved)
                }
            };
            let Some(current) = current else {
                *checking.borrow_mut() = false;
                ctx.request_repaint();
                return;
            };
            events.borrow_mut().push(Event::Catalog(current.clone()));
            let storage = current.storage.clone();
            for app in &current.apps {
                let latest = match app.release(&storage) {
                    Some(release) => LatestRelease::Found(release),
                    None if !app.github.is_empty() => latest_github(&app.github, pre).await,
                    None => LatestRelease::None,
                };
                events.borrow_mut().push(Event::Latest { app: app.id.clone(), latest, from: Origin::Catalog });
                ctx.request_repaint();
            }
            if let Some(release) = current.launcher_release()
                && let (Some(l), Some(c)) = (release.version.as_ref(), version::parse(env!("CARGO_PKG_VERSION")))
                && version::is_newer(l, &c)
            {
                events.borrow_mut().push(Event::LauncherUpdate(release));
            }
            let at = (js_sys::Date::now() / 1000.0) as i64;
            events.borrow_mut().push(Event::CheckFinished { at, source: Source::Catalog, rate_limited_until: None });
            *checking.borrow_mut() = false;
            ctx.request_repaint();
        });
    }
}

/// A path in the storage bucket, turned into an address.
fn absolute(path: &str, storage: &str) -> Option<String> {
    if path.starts_with("https://") {
        return Some(path.to_string());
    }
    let base = storage.trim().trim_end_matches('/');
    if base.is_empty() {
        return None;
    }
    let base = if base.starts_with("https://") { base.to_string() } else { format!("https://storage.googleapis.com/{base}") };
    let path = path.trim_start_matches('/');
    Some(format!("{base}/{path}"))
}

/// Reads the catalog from Firebase.
async fn read_catalog() -> Result<Catalog, String> {
    let url = bd_launcher_core::catalog::CATALOG_URL;
    let text = get_text(url).await.map_err(|why| format!("couldn't reach the catalog ({why})"))?;
    Catalog::parse(&text).ok_or_else(|| "the catalog isn't readable".to_string())
}

/// A GET of a small text file.
async fn get_text(url: &str) -> Result<String, String> {
    let window = web_sys::window().ok_or("no window")?;
    let init = web_sys::RequestInit::new();
    init.set_method("GET");
    init.set_mode(web_sys::RequestMode::Cors);
    let request = web_sys::Request::new_with_str_and_init(url, &init).map_err(|_| "bad request")?;
    let resp: web_sys::Response =
        JsFuture::from(window.fetch_with_request(&request)).await.map_err(|_| "no connection")?.dyn_into().map_err(|_| "no answer")?;
    match resp.status() {
        200 => {
            let promise = resp.text().map_err(|_| "unreadable answer".to_string())?;
            let text = JsFuture::from(promise).await.map_err(|_| "unreadable answer".to_string())?;
            Ok(text.as_string().unwrap_or_default())
        }
        status => Err(format!("the server answered HTTP {status}")),
    }
}

/// A GET of a small binary file.
async fn get_bytes(url: &str, max: usize) -> Result<Vec<u8>, String> {
    let window = web_sys::window().ok_or("no window")?;
    let init = web_sys::RequestInit::new();
    init.set_method("GET");
    init.set_mode(web_sys::RequestMode::Cors);
    let request = web_sys::Request::new_with_str_and_init(url, &init).map_err(|_| "bad request")?;
    let resp: web_sys::Response =
        JsFuture::from(window.fetch_with_request(&request)).await.map_err(|_| "no connection")?.dyn_into().map_err(|_| "no answer")?;
    match resp.status() {
        200 => {}
        404 => return Err("not published".into()),
        status => return Err(format!("the server answered HTTP {status}")),
    }
    let buffer = JsFuture::from(resp.array_buffer().map_err(|_| "unreadable")?).await.map_err(|_| "unreadable")?;
    let bytes = js_sys::Uint8Array::new(&buffer).to_vec();
    if bytes.len() > max {
        return Err(format!("more than {max} bytes"));
    }
    Ok(bytes)
}

/// An app's icon.
async fn icon_bytes(url: &str) -> Result<Vec<u8>, String> {
    let bytes = get_bytes(url, MAX_ICON_BYTES).await?;
    if !bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) && !bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Err("the answer isn't a picture".into());
    }
    Ok(bytes)
}

/// Why a picture couldn't be had.
enum PictureError {
    /// This picture only: it isn't published, or what came back isn't a picture.
    Picture(String),
    /// The network or the server: the other pictures would fail the same way.
    Network(String),
}

/// One picture of a details page. The size cap is checked on the announced length first and again
/// on what arrived; the bytes must start like a JPEG.
async fn picture(url: &str) -> Result<Vec<u8>, PictureError> {
    let offline = |what: &str| PictureError::Network(format!("couldn't load a picture ({what})"));
    let window = web_sys::window().ok_or_else(|| offline("no window"))?;
    let init = web_sys::RequestInit::new();
    init.set_method("GET");
    init.set_mode(web_sys::RequestMode::Cors);
    let request = web_sys::Request::new_with_str_and_init(url, &init).map_err(|_| offline("bad request"))?;
    let resp: web_sys::Response = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|_| offline("no connection"))?
        .dyn_into()
        .map_err(|_| offline("no answer"))?;
    match resp.status() {
        200 => {}
        404 => return Err(PictureError::Picture("this picture isn't published".into())),
        status => return Err(PictureError::Network(format!("the server answered HTTP {status}"))),
    }
    let too_big = || PictureError::Picture(format!("a picture of more than {MAX_SCREENSHOT_BYTES} bytes"));
    let announced = resp.headers().get("content-length").ok().flatten().and_then(|n| n.parse::<usize>().ok());
    if announced.is_some_and(|n| n > MAX_SCREENSHOT_BYTES) {
        return Err(too_big());
    }
    let buffer = JsFuture::from(resp.array_buffer().map_err(|_| offline("unreadable"))?).await.map_err(|_| offline("unreadable"))?;
    let bytes = js_sys::Uint8Array::new(&buffer).to_vec();
    if bytes.len() > MAX_SCREENSHOT_BYTES {
        return Err(too_big());
    }
    if !looks_like_jpeg(&bytes) {
        return Err(PictureError::Picture("the answer isn't a JPEG picture".into()));
    }
    Ok(bytes)
}

/// The latest release on GitHub, for an app whose releases live there.
async fn latest_github(repo: &str, prereleases: bool) -> LatestRelease {
    let url = if prereleases {
        format!("https://api.github.com/repos/{repo}/releases?per_page=30")
    } else {
        format!("https://api.github.com/repos/{repo}/releases/latest")
    };
    let text = match get_text(&url).await {
        Ok(t) => t,
        Err(_) => return LatestRelease::Failed("couldn't reach GitHub".into()),
    };
    let found: Option<Release> = if prereleases {
        release::parse_github_release_list(&text, repo, true).ok().flatten()
    } else {
        release::parse_github_release(&text, repo).ok()
    };
    found.map_or(LatestRelease::None, LatestRelease::Found)
}

/// The visitor's OS and CPU, as far as the browser says.
fn visitor_platform() -> Option<Platform> {
    let nav = web_sys::window()?.navigator();
    let ua = nav.user_agent().ok()?.to_lowercase();
    let os = if ua.contains("windows") {
        Os::Windows
    } else if ua.contains("mac os") || ua.contains("macintosh") {
        Os::MacOs
    } else if ua.contains("freebsd") {
        Os::FreeBsd
    } else if ua.contains("linux") && !ua.contains("android") {
        Os::Linux
    } else {
        return None;
    };
    let arch = if ua.contains("aarch64") || ua.contains("arm64") || ua.contains("armv8") {
        Arch::Arm64
    } else if ua.contains("riscv") {
        Arch::Riscv64
    } else {
        Arch::X64
    };
    Some(Platform { os, arch })
}

impl Backend for WebBackend {
    fn send(&mut self, command: Command) {
        match command {
            Command::CheckForUpdates { .. } => self.check(),
            Command::FetchScreenshots { app } => self.fetch_screenshots(&app),
            Command::FetchIcon { app } => self.fetch_icon(&app),
            Command::Configure(o) => {
                let changed = o.prereleases != self.prereleases;
                self.prereleases = o.prereleases;
                if changed {
                    self.check();
                }
            }
            _ => {}
        }
    }

    fn poll(&mut self) -> Vec<Event> {
        std::mem::take(&mut *self.events.borrow_mut())
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities { manage_apps: false, reveal_label: "", self_update: false }
    }

    fn platform(&self) -> Option<Platform> {
        self.platform
    }

    fn now(&self) -> i64 {
        (js_sys::Date::now() / 1000.0) as i64
    }
}

/// The apps the catalog knows about, for the tests.
#[allow(dead_code)]
fn apps_of(catalog: &Catalog) -> &[App] {
    &catalog.apps
}
