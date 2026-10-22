//! [`NativeBackend`]: the desktop implementation of the UI's [`Backend`].

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant};

use bd_launcher_core::catalog::{App, Catalog, LAUNCHER_ID};
use bd_launcher_core::platform::{Arch, Os, Platform, pick_asset};
use bd_launcher_core::protocol::{
    Backend, BackendOptions, CANCELLED, Capabilities, Command, Event, INSTALL_CANCELLED, JobPhase, LatestRelease, Origin, PAUSED, PausedDownload,
    Source,
};
use bd_launcher_core::release::{Asset, Release};
use bd_launcher_core::{Installed, PackageKind, version};

use crate::catalog_store::{CatalogStore, catalog_path};
use crate::http::{self, Http, Stop};
use crate::install::{self, Job};
use crate::manifest::Manifest;
use crate::paths::Paths;
use crate::process;
use crate::source::{GitHubSource, ReleaseSource};

/// The longest the launcher's own update waits for other installs to end before it gives up
/// (it can be cancelled sooner).
const RESTART_WAIT: Duration = Duration::from_secs(10 * 60);
/// Automatic checks closer together than this reuse the last answers. The UI asks every five
/// minutes; this stops a burst of requests (refresh, focus, timers) from repeating work.
const FRESH_FOR: i64 = 4 * 60;
/// Once every app has loaded, GitHub is asked about an app the catalog can't answer at most this
/// often (pressing Refresh asks right away).
const GITHUB_FALLBACK_EVERY: i64 = 60 * 60;
/// Until every app has loaded once, GitHub may be asked a little more eagerly.
const EARLY_GITHUB_EVERY: i64 = 5 * 60;
/// Requests in flight at once during a check.
const WORKERS: usize = 4;

type Waker = Arc<dyn Fn() + Send + Sync>;

struct Shared {
    events: Mutex<Vec<Event>>,
    waker: Waker,
    /// The catalog, as last read from Firebase.
    catalog: Mutex<Catalog>,
    latest: Mutex<HashMap<String, Release>>,
    installed: Mutex<HashMap<String, Installed>>,
    jobs: Mutex<HashMap<String, Arc<Stop>>>,
    /// The launcher is replacing itself and will restart: no new job starts.
    restarting: AtomicBool,
    manifest: Mutex<Manifest>,
    options: Mutex<BackendOptions>,
    checking: AtomicBool,
    /// A detection pass is running; another request waits for the next one.
    detecting: AtomicBool,
    last_check: Mutex<Option<i64>>,
    /// The newer BD Launcher release, once a check has found one.
    launcher_release: Mutex<Option<Release>>,
    /// How this launcher is installed, when it can replace itself. Worked out on the check thread
    /// just before a newer launcher is announced: finding out can ask the host's Flatpak, which
    /// mustn't hold up the first frame.
    self_install: OnceLock<Option<crate::self_update::Install>>,
    /// When GitHub was last asked about each app (Unix seconds), to keep the fallback rare.
    github_asked: Mutex<HashMap<String, i64>>,
    /// Apps with an answer (a release or "none yet") from either service.
    answered: Mutex<std::collections::HashSet<String>>,
    /// Every app has had an answer at least once: from then on GitHub is asked sparingly.
    loaded: AtomicBool,
    /// Apps whose details pictures are on their way (a second request meanwhile does nothing).
    pictures_busy: Mutex<std::collections::HashSet<String>>,
}

impl Shared {
    fn emit(&self, event: Event) {
        self.events.lock().unwrap_or_else(PoisonError::into_inner).push(event);
        (self.waker)();
    }

    /// The catalog, as last read.
    fn catalog(&self) -> Catalog {
        self.catalog.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// One app of the catalog.
    fn app(&self, id: &str) -> Option<App> {
        self.catalog.lock().unwrap_or_else(PoisonError::into_inner).by_id(id).cloned()
    }
}

pub struct NativeBackend {
    shared: Arc<Shared>,
    paths: Paths,
    http: Option<Http>,
    source: Option<Arc<dyn ReleaseSource>>,
    /// Where the catalog is read from and kept.
    catalog: CatalogStore,
    platform: Option<Platform>,
}

impl NativeBackend {
    /// `waker` is called (from any thread) whenever events are waiting; it should repaint the UI.
    pub fn new(waker: impl Fn() + Send + Sync + 'static) -> Self {
        let waker: Waker = Arc::new(waker);
        let paths = Paths::new().unwrap_or_else(|e| {
            log::error!("{e}; keeping files in the temporary folder");
            Paths::under(&std::env::temp_dir().join("bd-launcher"))
        });
        let http = match Http::new() {
            Ok(h) => Some(h),
            Err(e) => {
                log::error!("HTTPS unavailable: {e}");
                None
            }
        };
        let source: Option<Arc<dyn ReleaseSource>> =
            http.clone().map(|h| Arc::new(GitHubSource::new(h, paths.release_cache())) as Arc<dyn ReleaseSource>);
        let catalog = CatalogStore::new(http.clone(), catalog_path(&paths.cache));
        let shared = Arc::new(Shared {
            events: Mutex::new(Vec::new()),
            waker,
            catalog: Mutex::new(Catalog::builtin()),
            latest: Mutex::new(HashMap::new()),
            installed: Mutex::new(HashMap::new()),
            jobs: Mutex::new(HashMap::new()),
            restarting: AtomicBool::new(false),
            manifest: Mutex::new(Manifest::load(&paths.manifest())),
            options: Mutex::new(BackendOptions::default()),
            checking: AtomicBool::new(false),
            detecting: AtomicBool::new(false),
            launcher_release: Mutex::new(None),
            self_install: OnceLock::new(),
            github_asked: Mutex::new(HashMap::new()),
            answered: Mutex::new(std::collections::HashSet::new()),
            loaded: AtomicBool::new(false),
            pictures_busy: Mutex::new(std::collections::HashSet::new()),
            last_check: Mutex::new(None),
        });
        let backend = Self { shared, paths, http, source, catalog, platform: native_platform() };
        backend.replay_catalog();
        backend.detect_all();
        backend
    }

    /// Shows the saved catalog straight away, before any network request: the apps it lists and
    /// the versions it last saw, so the window isn't empty while Firebase is being asked.
    fn replay_catalog(&self) {
        let Some(catalog) = self.catalog.cached() else { return };
        *self.shared.catalog.lock().unwrap_or_else(PoisonError::into_inner) = catalog.clone();
        self.shared.emit(Event::Catalog(catalog.clone()));
        self.shared.emit(Event::Notices { announcements: catalog.announcements.clone(), alerts: catalog.alerts.clone() });
        for app in &catalog.apps {
            let Some(release) = app.release(&catalog.storage) else { continue };
            self.shared.latest.lock().unwrap_or_else(PoisonError::into_inner).insert(app.id.clone(), release.clone());
            self.shared.emit(Event::Latest { app: app.id.clone(), latest: LatestRelease::Found(release), from: Origin::Catalog });
        }
    }

    fn options(&self) -> BackendOptions {
        self.shared.options.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    fn detect_all(&self) {
        if self.shared.detecting.swap(true, Ordering::SeqCst) {
            return;
        }
        let shared = self.shared.clone();
        let paths = self.paths.clone();
        let platform = self.platform;
        let options = self.options();
        let apps = self.shared.catalog().apps;
        std::thread::spawn(move || {
            detect_apps(&shared, &paths, platform, &options, &apps);
            shared.detecting.store(false, Ordering::SeqCst);
        });
    }

    /// Reads the catalog from Firebase and works out every app's latest release.
    ///
    /// The catalog is the authority: it carries the apps and the files of their latest release,
    /// so one request settles the whole list. GitHub is asked only about an app the catalog
    /// doesn't carry a release for but that has a repository there, and at most once an hour
    /// unless the user pressed Refresh.
    fn check(&self, force: bool) {
        let now = crate::now();
        let recent = self.shared.last_check.lock().unwrap_or_else(PoisonError::into_inner).is_some_and(|t| now - t < FRESH_FOR);
        if (recent && !force) || self.shared.checking.swap(true, Ordering::SeqCst) {
            return;
        }
        let shared = self.shared.clone();
        let paths = self.paths.clone();
        let platform = self.platform;
        let options = self.options();
        let pre = options.prereleases;
        let source = self.source.clone();
        let catalog_store = self.catalog.clone();
        std::thread::spawn(move || {
            shared.emit(Event::CheckStarted);
            let Some(catalog) = read_catalog(&catalog_store, &shared) else {
                // The reason was announced; there is nothing to show.
                let at = crate::now();
                *shared.last_check.lock().unwrap_or_else(PoisonError::into_inner) = Some(at);
                shared.checking.store(false, Ordering::SeqCst);
                shared.emit(Event::CheckFinished { at, source: Source::Cache, rate_limited_until: None });
                return;
            };
            let storage = catalog.storage.clone();
            *shared.catalog.lock().unwrap_or_else(PoisonError::into_inner) = catalog.clone();
            // The list may name apps the machine has: look again now that it is known.
            if !shared.detecting.swap(true, Ordering::SeqCst) {
                detect_apps(&shared, &paths, platform, &options, &catalog.apps);
                shared.detecting.store(false, Ordering::SeqCst);
            }
            shared.emit(Event::Catalog(catalog.clone()));
            shared.emit(Event::Notices { announcements: catalog.announcements.clone(), alerts: catalog.alerts.clone() });
            // The launcher's own update, when the catalog carries one.
            if let Some(release) = catalog.launcher_release()
                && let (Some(newest), Some(current)) = (release.version.as_ref(), crate::self_update::current_version())
                && version::is_newer(newest, &current)
                && !crate::self_update::just_updated_to(&paths, &release)
            {
                shared.self_install.get_or_init(crate::self_update::current);
                *shared.launcher_release.lock().unwrap_or_else(PoisonError::into_inner) = Some(release.clone());
                shared.emit(Event::LauncherUpdate(release));
            }
            let now = crate::now();
            let sources = Mutex::new(Vec::new());
            std::thread::scope(|scope| {
                for _ in 0..WORKERS {
                    scope.spawn(|| {
                        for app in &catalog.apps {
                            let (latest, from, used) = latest_for(app, &storage, source.as_deref(), &shared, pre, force, now);
                            if let Some(used) = used {
                                sources.lock().unwrap_or_else(PoisonError::into_inner).push(used);
                            }
                            let Some(latest) = latest else { continue };
                            if let LatestRelease::Found(r) = &latest {
                                shared.latest.lock().unwrap_or_else(PoisonError::into_inner).insert(app.id.clone(), r.clone());
                            }
                            if !matches!(latest, LatestRelease::Failed(_)) {
                                shared.answered.lock().unwrap_or_else(PoisonError::into_inner).insert(app.id.clone());
                            }
                            shared.emit(Event::Latest { app: app.id.clone(), latest, from });
                        }
                    });
                }
            });
            if let Some(source) = source.as_ref() {
                source.save();
            }
            {
                let answered = shared.answered.lock().unwrap_or_else(PoisonError::into_inner);
                if catalog.apps.iter().all(|a| answered.contains(&a.id)) {
                    shared.loaded.store(true, Ordering::SeqCst);
                }
            }
            // A newer release changes which file an install would use.
            let options = shared.options.lock().unwrap_or_else(PoisonError::into_inner).clone();
            for app in &catalog.apps {
                report_partial(app, &shared, &paths, platform, &options);
            }
            let sources = sources.into_inner().unwrap_or_else(PoisonError::into_inner);
            let source_used = if sources.contains(&Source::Cache) {
                Source::Cache
            } else if sources.contains(&Source::GitHubPages) {
                Source::GitHubPages
            } else if sources.contains(&Source::GitHubApi) {
                Source::GitHubApi
            } else {
                Source::Catalog
            };
            let at = crate::now();
            *shared.last_check.lock().unwrap_or_else(PoisonError::into_inner) = Some(at);
            shared.checking.store(false, Ordering::SeqCst);
            let rate_limited_until = limit_notice(source_used, source.as_ref().and_then(|s| s.rate_limited_until()));
            shared.emit(Event::CheckFinished { at, source: source_used, rate_limited_until });
        });
    }

    fn start_job(&self, app_id: &str, work: Work) {
        let uninstall = work == Work::Uninstall;
        let Some(app) = self.shared.app(app_id) else { return };
        log::info!("{}: starting {} job", app.name, if uninstall { "uninstall" } else { "install/update" });
        let stop = Arc::new(Stop::default());
        let restarting = {
            let mut jobs = self.shared.jobs.lock().unwrap_or_else(PoisonError::into_inner);
            if jobs.contains_key(&app.id) {
                return;
            }
            let restarting = self.shared.restarting.load(Ordering::SeqCst);
            if !restarting {
                jobs.insert(app.id.clone(), stop.clone());
            }
            restarting
        };
        if restarting {
            let result = Err("BD Launcher is restarting to finish its update. Try again once it's back.".into());
            self.shared.emit(Event::JobFinished { app: app.id.clone(), result });
            return;
        }
        let shared = self.shared.clone();
        let paths = self.paths.clone();
        let http = self.http.clone();
        let platform = self.platform;
        let options = self.options();
        std::thread::spawn(move || {
            let existing = shared.installed.lock().unwrap_or_else(PoisonError::into_inner).get(&app.id).cloned();
            let record = shared.manifest.lock().unwrap_or_else(PoisonError::into_inner).apps.get(&app.id).cloned();
            let result = if uninstall {
                shared.emit(Event::Progress { app: app.id.clone(), phase: JobPhase::Removing });
                let mut phase = |phase| shared.emit(Event::Progress { app: app.id.clone(), phase });
                match &existing {
                    Some(inst) => install::uninstall(&app, inst, record.as_ref(), &paths, &stop, &mut phase).map(|()| {
                        let mut m = shared.manifest.lock().unwrap_or_else(PoisonError::into_inner);
                        if m.apps.remove(&app.id).is_some() {
                            let _ = m.save(&paths.manifest());
                        }
                        format!("{} was removed", app.name)
                    }),
                    None => Err(format!("{} isn't installed", app.name)),
                }
            } else {
                let r = run_install(
                    &app,
                    &shared,
                    &paths,
                    http.as_ref(),
                    platform,
                    &options,
                    existing.as_ref(),
                    record.as_ref(),
                    &stop,
                    work == Work::Replace,
                );
                // Stopped by the connection rather than by the user: say so, and how to go on.
                match r {
                    Err(e) if e == PAUSED && stop.check().is_ok() => {
                        Err("The download paused because the connection keeps dropping. Press Resume to continue where it stopped.".into())
                    }
                    other => other,
                }
            };
            report_partial(&app, &shared, &paths, platform, &options);
            // Look again: the result is whatever is on disk now.
            let manifest = shared.manifest.lock().unwrap_or_else(PoisonError::into_inner).clone();
            let found = crate::detect::detect(&app, &manifest, &crate::detect::Snapshot::take(std::slice::from_ref(&app)));
            // One installer call can cover several apps, and fail after some of them went in.
            let latest = shared.latest.lock().unwrap_or_else(PoisonError::into_inner).get(&app.id).cloned();
            let result = match (result, latest) {
                (Err(e), Some(release))
                    if !uninstall && release.version.as_ref().is_some_and(|v| landed_despite(&e, existing.as_ref(), found.as_ref(), v)) =>
                {
                    log::warn!("{}: {e}; the new version is installed all the same", app.id);
                    Ok(installed_text(&app, existing.is_some(), &release))
                }
                (result, _) => result,
            };
            {
                let mut map = shared.installed.lock().unwrap_or_else(PoisonError::into_inner);
                match &found {
                    Some(i) => map.insert(app.id.clone(), i.clone()),
                    None => map.remove(&app.id),
                };
            }
            shared.emit(Event::Detected { app: app.id.clone(), installed: found });
            shared.jobs.lock().unwrap_or_else(PoisonError::into_inner).remove(&app.id);
            shared.emit(Event::JobFinished { app: app.id.clone(), result });
        });
    }

    /// Downloads the newer launcher, puts it in place and asks the UI to quit so it can start.
    fn update_launcher(&self) {
        let id = LAUNCHER_ID.to_string();
        let (Some(install), Some(platform)) = (self.shared.self_install.get().cloned().flatten(), self.platform) else {
            self.shared.emit(Event::JobFinished {
                app: id,
                result: Err("this copy of BD Launcher can't update itself; download the new version instead".into()),
            });
            return;
        };
        let Some(release) = self.shared.launcher_release.lock().unwrap_or_else(PoisonError::into_inner).clone() else { return };
        let stop = Arc::new(Stop::default());
        {
            let mut jobs = self.shared.jobs.lock().unwrap_or_else(PoisonError::into_inner);
            if jobs.contains_key(&id) {
                return;
            }
            jobs.insert(id.clone(), stop.clone());
        }
        let (shared, paths, http) = (self.shared.clone(), self.paths.clone(), self.http.clone());
        std::thread::spawn(move || {
            let mut last_emit = Instant::now() - Duration::from_secs(1);
            let mut phase = |p: JobPhase| {
                if matches!(p, JobPhase::Downloading { done, total } if done != total && done != 0) && last_emit.elapsed() < Duration::from_millis(50)
                {
                    return;
                }
                last_emit = Instant::now();
                shared.emit(Event::Progress { app: LAUNCHER_ID.to_string(), phase: p });
            };
            let last = || {
                let mut waiting = || shared.emit(Event::Progress { app: LAUNCHER_ID.to_string(), phase: JobPhase::Waiting });
                claim_restart(&shared, &shared.jobs, &shared.restarting, &stop, RESTART_WAIT, &mut waiting)
            };
            let result = match http.as_ref() {
                Some(http) => crate::self_update::update(&install, &release, platform, http, &paths, &stop, &mut phase, &last),
                None => Err("secure connections aren't available on this system".into()),
            };
            shared.jobs.lock().unwrap_or_else(PoisonError::into_inner).remove(LAUNCHER_ID);
            match result {
                Ok(()) => shared.emit(Event::LauncherRestarting),
                Err(e) => {
                    shared.restarting.store(false, Ordering::SeqCst);
                    shared.emit(Event::JobFinished { app: LAUNCHER_ID.to_string(), result: Err(e) });
                }
            }
        });
    }

    fn installed(&self, app: &str) -> Option<Installed> {
        self.shared.installed.lock().unwrap_or_else(PoisonError::into_inner).get(app).cloned()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Work {
    Install,
    Replace,
    Uninstall,
}

#[allow(clippy::too_many_arguments)]
fn run_install(
    app: &App,
    shared: &Shared,
    paths: &Paths,
    http: Option<&Http>,
    platform: Option<Platform>,
    options: &BackendOptions,
    existing: Option<&Installed>,
    record: Option<&crate::manifest::Record>,
    stop: &Stop,
    replace: bool,
) -> Result<String, String> {
    if let Some(msg) = existing.and_then(|e| install::managed_elsewhere(app, e, false)) {
        return Err(msg);
    }
    let http = http.ok_or("secure connections aren't available on this system")?;
    let platform = platform.ok_or("this system isn't supported")?;
    let release = shared
        .latest
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&app.id)
        .cloned()
        .ok_or("the latest release hasn't been found yet. Check for updates first.")?;
    let (asset, kind) = planned_asset(app, &release, platform, options, existing, record).ok_or_else(|| {
        if platform.os == bd_launcher_core::Os::Linux && options.install_locations.folder(&app.id).is_some() && existing.is_none() && record.is_none()
        {
            "this release has no AppImage for a custom install folder".to_string()
        } else {
            format!("{} {} has no download for {}", app.name, release.version_label(), platform.label())
        }
    })?;
    if !replace && let Some(path) = install::occupied(app, &asset, kind, existing, record, &options.install_locations, paths)? {
        shared.emit(Event::Occupied { app: app.id.clone(), path: path.display().to_string() });
        return Err(format!("{} already exists", path.display()));
    }
    let mut last_emit = Instant::now() - Duration::from_secs(1);
    let mut phase = |p: JobPhase| {
        // Downloads report thousands of times; the UI needs ~20 a second.
        if matches!(p, JobPhase::Downloading { done, total } if done != total && done != 0) && last_emit.elapsed() < Duration::from_millis(50) {
            return;
        }
        last_emit = Instant::now();
        shared.emit(Event::Progress { app: app.id.clone(), phase: p });
    };
    let mut job = Job { http, paths, stop, phase: &mut phase, install_locations: &options.install_locations, replace };
    let mut commit_record = |record: &crate::manifest::Record| {
        let mut manifest = shared.manifest.lock().unwrap_or_else(PoisonError::into_inner);
        let mut updated = manifest.clone();
        updated.apps.insert(app.id.clone(), record.clone());
        updated.save(&paths.manifest())?;
        *manifest = updated;
        Ok(())
    };
    let new_record = install::install_at(app, &release, &asset, kind, existing, record, &mut job, Some(&mut commit_record))?;
    {
        let mut m = shared.manifest.lock().unwrap_or_else(PoisonError::into_inner);
        let changed = match new_record {
            Some(r) => {
                m.apps.insert(app.id.clone(), r);
                true
            }
            // Installed the platform's own way: any old launcher record is stale.
            None => m.apps.remove(&app.id).is_some(),
        };
        if changed && let Err(e) = m.save(&paths.manifest()) {
            log::warn!("couldn't save the install record: {e}");
        }
    }
    Ok(installed_text(app, existing.is_some(), &release))
}

/// What a finished install tells the user.
fn installed_text(app: &App, update: bool, release: &Release) -> String {
    let verb = if update { "updated to" } else { "installed:" };
    format!("{} {verb} {}", app.name, release.version_label())
}

/// Whether an install whose job failed with `error` went in all the same. Only a failed call
/// shared by several apps (Linux packages installed together) can have installed some of them;
/// any other error, a cancellation or a pause stands, whatever is found afterwards.
fn landed_despite(error: &str, before: Option<&Installed>, after: Option<&Installed>, latest: &version::Version) -> bool {
    error != PAUSED && error != CANCELLED && error != INSTALL_CANCELLED && install::failed_in_shared_call(error) && landed(before, after, latest)
}

/// Whether an install took, whatever its job reported: `latest` is there now and wasn't before.
fn landed(before: Option<&Installed>, after: Option<&Installed>, latest: &version::Version) -> bool {
    let has_latest = |copy: Option<&Installed>| copy.and_then(|i| i.version.as_ref()).is_some_and(|v| !version::is_update(latest, v));
    has_latest(after) && !has_latest(before)
}

/// For the launcher's own update, before it replaces itself: waits (saying so through `waiting`)
/// until every other job has ended, then sets `restarting`, which keeps new jobs from starting.
/// The restart that follows would otherwise cut their installs short. The user can cancel the
/// wait (`stop`); after `limit` it gives up and names the apps still busy, since one may be
/// stuck behind a password prompt nobody has answered.
fn claim_restart(
    shared: &Shared,
    jobs: &Mutex<HashMap<String, Arc<Stop>>>,
    restarting: &AtomicBool,
    stop: &Stop,
    limit: Duration,
    waiting: &mut dyn FnMut(),
) -> Result<(), String> {
    let started = Instant::now();
    let mut told = false;
    loop {
        stop.check()?;
        let busy: Vec<String> = {
            let jobs = jobs.lock().unwrap_or_else(PoisonError::into_inner);
            let busy =
                jobs.keys().filter(|id| *id != LAUNCHER_ID).map(|id| shared.app(id).map_or_else(|| "another app".to_string(), |a| a.name)).collect();
            if jobs.keys().all(|id| id == LAUNCHER_ID) {
                restarting.store(true, Ordering::SeqCst);
                return Ok(());
            }
            busy
        };
        if started.elapsed() >= limit {
            return Err(restart_blocked(busy));
        }
        if !told {
            told = true;
            waiting();
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Why the launcher didn't restart: `busy` apps were still installing, and what to do about it.
fn restart_blocked(mut busy: Vec<String>) -> String {
    busy.sort_unstable();
    busy.dedup();
    let (names, were, them) = match busy.as_slice() {
        [one] => (one.clone(), "was", "it"),
        _ => (busy.join(", "), "were", "them"),
    };
    format!(
        "BD Launcher didn't restart to update: {names} {were} still installing. Answer or close any password prompt, \
         let {them} finish or cancel {them}, then press Update and restart again."
    )
}

/// The file an install of `app` would download now: same format as the installed copy, else
/// the user's preference, else the best this system supports.
fn planned_asset(
    app: &App,
    release: &Release,
    platform: Platform,
    options: &BackendOptions,
    existing: Option<&Installed>,
    record: Option<&crate::manifest::Record>,
) -> Option<(Asset, PackageKind)> {
    let stems: Vec<&str> = app.stems().collect();
    let custom_linux = platform.os == bd_launcher_core::Os::Linux
        && !process::in_sandbox()
        && existing.is_none()
        && record.is_none()
        && options.install_locations.folder(&app.id).is_some();
    let kinds = if existing.is_none() && record.is_some_and(|r| r.kind == PackageKind::PortableZip) {
        vec![PackageKind::PortableZip]
    } else if custom_linux {
        vec![bd_launcher_core::PackageKind::AppImage]
    } else {
        install::choose_kinds(app, existing, record, &options.kinds)
    };
    pick_asset(release, &stems, platform, &kinds).map(|(a, k)| (a.clone(), k))
}

/// Tells the UI whether `app` has a paused download waiting, or a finished one to install.
fn report_partial(app: &App, shared: &Shared, paths: &Paths, platform: Option<Platform>, options: &BackendOptions) {
    let planned = planned_download(app, shared, paths, platform, options);
    let download = planned.as_ref().and_then(|(dest, size)| Some(PausedDownload { done: http::partial_bytes(dest, *size)?, total: *size }));
    shared.emit(Event::Paused { app: app.id.clone(), download });
    let ready = planned.is_some_and(|(dest, size)| http::finished(&dest, size));
    shared.emit(Event::ReadyToInstall { app: app.id.clone(), ready });
}

/// Where the planned download of `app` is kept, and its size.
fn planned_download(
    app: &App,
    shared: &Shared,
    paths: &Paths,
    platform: Option<Platform>,
    options: &BackendOptions,
) -> Option<(std::path::PathBuf, u64)> {
    let release = shared.latest.lock().unwrap_or_else(PoisonError::into_inner).get(&app.id).cloned()?;
    let existing = shared.installed.lock().unwrap_or_else(PoisonError::into_inner).get(&app.id).cloned();
    let record = shared.manifest.lock().unwrap_or_else(PoisonError::into_inner).apps.get(&app.id).cloned();
    let (asset, _) = planned_asset(app, &release, platform?, options, existing.as_ref(), record.as_ref())?;
    bd_launcher_core::release::valid_file_name(&asset.name).then(|| (install::download_path(paths, &asset), asset.size))
}

/// Looks at the machine for the catalog's apps: which are installed, and what a download left
/// behind in an earlier run.
fn detect_apps(shared: &Arc<Shared>, paths: &Paths, platform: Option<Platform>, options: &BackendOptions, apps: &[App]) {
    // Leftovers from interrupted installs or older versions of the launcher. A paused download
    // stays a month; a finished one kept after a failed install, a week.
    http::clean_downloads(&paths.downloads(), Duration::from_secs(30 * 86_400), Duration::from_secs(7 * 86_400));
    let snapshot = crate::detect::Snapshot::take(apps);
    let manifest = shared.manifest.lock().unwrap_or_else(PoisonError::into_inner).clone();
    for app in apps {
        let found = crate::detect::detect(app, &manifest, &snapshot);
        {
            let mut map = shared.installed.lock().unwrap_or_else(PoisonError::into_inner);
            match &found {
                Some(i) => map.insert(app.id.clone(), i.clone()),
                None => map.remove(&app.id),
            };
        }
        shared.emit(Event::Detected { app: app.id.clone(), installed: found });
    }
    // Downloads paused (or cut off by quitting) in an earlier run.
    for app in apps {
        report_partial(app, shared, paths, platform, options);
    }
}

/// The catalog, read from Firebase. The copy on disk stands in when Firebase can't be reached;
/// with no copy at all the launcher says so, rather than showing an empty list. `None` means
/// there is nothing to show and the reason has been announced.
fn read_catalog(store: &CatalogStore, shared: &Shared) -> Option<Catalog> {
    let catalog = match store.fetch() {
        Ok(Some(catalog)) => Some(catalog),
        // Not changed since the last read: the copy on disk is current.
        Ok(None) => store.cached(),
        Err(e) => {
            log::warn!("the catalog could not be read: {e}");
            match store.cached() {
                Some(catalog) => Some(catalog),
                None => {
                    shared.emit(Event::CatalogFailed(e));
                    None
                }
            }
        }
    }?;
    // A deployment may keep its files in another bucket.
    Some(match std::env::var("BD_LAUNCHER_STORAGE") {
        Ok(storage) if !storage.trim().is_empty() => catalog.with_storage(&storage),
        _ => catalog,
    })
}

/// The latest release of `app`, which service it came from, and the GitHub route used (for the
/// rate-limit notice): the catalog when it carries a release for the app, else GitHub if the app
/// has a repository there and GitHub is due to be asked, else GitHub's last answer. No release
/// when there's nothing new to say.
#[allow(clippy::too_many_arguments)]
fn latest_for(
    app: &App,
    storage: &str,
    source: Option<&dyn ReleaseSource>,
    shared: &Shared,
    pre: bool,
    force: bool,
    now: i64,
) -> (Option<LatestRelease>, Origin, Option<Source>) {
    if let Some(release) = app.release(storage) {
        return (Some(LatestRelease::Found(release)), Origin::Catalog, Some(Source::Catalog));
    }
    // The catalog lists the app but carries no release for it: GitHub may know one.
    if app.github.is_empty() {
        return (Some(LatestRelease::None), Origin::Catalog, Some(Source::Catalog));
    }
    let Some(source) = source else {
        return (Some(LatestRelease::Failed("secure connections aren't available".into())), Origin::GitHub, None);
    };
    let loaded = shared.loaded.load(Ordering::SeqCst);
    let github_every = if loaded { GITHUB_FALLBACK_EVERY } else { EARLY_GITHUB_EVERY };
    let due = {
        let mut asked = shared.github_asked.lock().unwrap_or_else(PoisonError::into_inner);
        let due = force || asked.get(&app.id).is_none_or(|t| now - t >= github_every);
        if due {
            asked.insert(app.id.clone(), now);
        }
        due
    };
    if due {
        let answer = source.latest(&app.github, pre);
        (Some(answer.latest), Origin::GitHub, Some(answer.source))
    } else {
        (source.cached(&app.github, pre), Origin::GitHub, None)
    }
}

/// Until when the footer says GitHub's limit holds details back: only while it does, that is
/// when this check had to settle for the release pages or saved answers. When the catalog (or the
/// API itself) answered everything, nothing is missing, however recently GitHub said no.
fn limit_notice(source_used: Source, limited_until: Option<i64>) -> Option<i64> {
    limited_until.filter(|_| matches!(source_used, Source::GitHubPages | Source::Cache))
}

/// The machine's real architecture: an x64 launcher on Windows on ARM should still pick ARM64
/// builds.
fn native_platform() -> Option<Platform> {
    let mut p = Platform::current()?;
    if p.os == Os::Windows {
        let arch = std::env::var("PROCESSOR_ARCHITEW6432").or_else(|_| std::env::var("PROCESSOR_ARCHITECTURE")).unwrap_or_default();
        if arch.eq_ignore_ascii_case("ARM64") {
            p.arch = Arch::Arm64;
        } else if arch.eq_ignore_ascii_case("AMD64") {
            p.arch = Arch::X64;
        }
    }
    Some(p)
}

/// Clears an app's "pictures on their way" mark when the thread fetching them ends, panic or not.
struct PicturesBusy {
    shared: Arc<Shared>,
    app: String,
}

impl Drop for PicturesBusy {
    fn drop(&mut self) {
        self.shared.pictures_busy.lock().unwrap_or_else(PoisonError::into_inner).remove(&self.app);
    }
}

impl NativeBackend {
    /// Starts fetching the pictures of an app's details page, unless they are already on their way.
    /// An app without a page, or a name that isn't an app, is ignored.
    fn fetch_screenshots(&self, app: &str) {
        let Some(entry) = self.shared.app(app) else { return };
        if entry.details.is_none() {
            return;
        }
        if !self.shared.pictures_busy.lock().unwrap_or_else(PoisonError::into_inner).insert(entry.id.clone()) {
            return;
        }
        let (shared, http, paths) = (self.shared.clone(), self.http.clone(), self.paths.clone());
        let storage = self.shared.catalog().storage;
        std::thread::spawn(move || {
            let _busy = PicturesBusy { shared: shared.clone(), app: entry.id.clone() };
            crate::screenshots::fetch_all(http.as_ref(), &paths, &entry, &storage, |key, result| {
                shared.emit(Event::Screenshot { app: entry.id.clone(), key: key.to_string(), result });
            });
        });
    }

    /// Fetches an app's icon, unless it is already here. The icon is kept on disk, so this costs
    /// a request only the first time.
    fn fetch_icon(&self, app: &str) {
        let Some(entry) = self.shared.app(app) else { return };
        if entry.icon.is_empty() {
            return;
        }
        let (shared, http, paths) = (self.shared.clone(), self.http.clone(), self.paths.clone());
        let storage = self.shared.catalog().storage;
        std::thread::spawn(move || {
            let result = crate::icons::fetch(http.as_ref(), &paths, &entry, &storage);
            shared.emit(Event::Icon { app: entry.id.clone(), result });
        });
    }
}

impl Backend for NativeBackend {
    fn send(&mut self, command: Command) {
        match command {
            Command::CheckForUpdates { force } => self.check(force),
            Command::Detect => self.detect_all(),
            Command::Install { app } => self.start_job(&app, Work::Install),
            Command::Replace { app } => self.start_job(&app, Work::Replace),
            Command::Uninstall { app } => self.start_job(&app, Work::Uninstall),
            Command::Pause { app } => {
                if let Some(stop) = self.shared.jobs.lock().unwrap_or_else(PoisonError::into_inner).get(&app) {
                    stop.pause();
                }
            }
            Command::Cancel { app } => {
                let running = self.shared.jobs.lock().unwrap_or_else(PoisonError::into_inner).get(&app).cloned();
                match running {
                    Some(stop) => stop.cancel(),
                    // A paused download, or one kept after a failed install: throw the file away.
                    None => {
                        if let Some(def) = self.shared.app(&app)
                            && let Some((dest, size)) = planned_download(&def, &self.shared, &self.paths, self.platform, &self.options())
                        {
                            http::discard(&dest, size);
                        }
                        self.shared.emit(Event::Paused { app: app.clone(), download: None });
                        self.shared.emit(Event::ReadyToInstall { app, ready: false });
                    }
                }
            }
            Command::Launch { app } => {
                let Some(def) = self.shared.app(&app) else { return };
                let result = match self.installed(&app) {
                    Some(inst) => install::launch(&def, &inst, &self.paths),
                    None => Err(format!("{} isn't installed", def.name)),
                };
                if let Err(e) = result {
                    self.shared.emit(Event::JobFinished { app, result: Err(e) });
                }
            }
            Command::Reveal { app } => {
                if let Some(inst) = self.installed(&app) {
                    let path = std::path::PathBuf::from(&inst.location);
                    let result = if path.exists() { crate::process::reveal(&path) } else { Err("it isn't a file on this computer".into()) };
                    if let Err(e) = result {
                        self.shared.emit(Event::JobFinished { app, result: Err(format!("Couldn't show it: {e}")) });
                    }
                }
            }
            Command::UpdateLauncher => self.update_launcher(),
            Command::FetchScreenshots { app } => self.fetch_screenshots(&app),
            Command::FetchIcon { app } => self.fetch_icon(&app),
            Command::Configure(options) => {
                let changed_pre = {
                    let mut o = self.shared.options.lock().unwrap_or_else(PoisonError::into_inner);
                    let changed = o.prereleases != options.prereleases;
                    *o = options;
                    changed
                };
                // Another package format means another file to download.
                let options = self.options();
                for app in &self.shared.catalog().apps {
                    report_partial(app, &self.shared, &self.paths, self.platform, &options);
                }
                if changed_pre {
                    self.check(true);
                }
            }
        }
    }

    fn poll(&mut self) -> Vec<Event> {
        std::mem::take(&mut *self.shared.events.lock().unwrap_or_else(PoisonError::into_inner))
    }

    fn capabilities(&self) -> Capabilities {
        let reveal_label = match self.platform.map(|p| p.os) {
            Some(Os::MacOs) => "Show in Finder",
            Some(Os::Windows) => "Show in Explorer",
            _ => "Show in Files",
        };
        Capabilities { manage_apps: true, reveal_label, self_update: self.shared.self_install.get().is_some_and(Option::is_some) }
    }

    fn platform(&self) -> Option<Platform> {
        self.platform
    }

    fn now(&self) -> i64 {
        crate::now()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;
    use std::sync::mpsc;

    use bd_launcher_core::InstallSource;

    use bd_launcher_core::protocol::LatestRelease;

    use super::*;
    use crate::source::Answer;

    /// GitHub, counting how often it's asked.
    #[derive(Default)]
    struct CountingGitHub(AtomicUsize);

    impl ReleaseSource for CountingGitHub {
        fn latest(&self, _repo: &str, _pre: bool) -> Answer {
            self.0.fetch_add(1, Ordering::SeqCst);
            Answer { latest: LatestRelease::None, source: Source::GitHubApi }
        }
        fn cached(&self, _repo: &str, _pre: bool) -> Option<LatestRelease> {
            None
        }
        fn rate_limited_until(&self) -> Option<i64> {
            None
        }
        fn save(&self) {}
    }

    /// The catalog the tests work with: two apps, one with a release and one without.
    fn catalog() -> Catalog {
        bd_launcher_core::catalog::sample()
    }

    /// The app the tests install.
    fn app() -> App {
        catalog().apps.first().cloned().expect("the sample catalog has an app")
    }

    fn shared() -> Shared {
        Shared {
            events: Mutex::new(Vec::new()),
            waker: Arc::new(|| {}),
            catalog: Mutex::new(catalog()),
            latest: Mutex::new(HashMap::new()),
            installed: Mutex::new(HashMap::new()),
            jobs: Mutex::new(HashMap::new()),
            restarting: AtomicBool::new(false),
            manifest: Mutex::new(Manifest::default()),
            options: Mutex::new(BackendOptions::default()),
            checking: AtomicBool::new(false),
            detecting: AtomicBool::new(false),
            last_check: Mutex::new(None),
            launcher_release: Mutex::new(None),
            self_install: OnceLock::new(),
            github_asked: Mutex::new(HashMap::new()),
            answered: Mutex::new(std::collections::HashSet::new()),
            loaded: AtomicBool::new(false),
            pictures_busy: Mutex::new(std::collections::HashSet::new()),
        }
    }

    const NOW: i64 = 1_791_500_000;

    #[test]
    fn the_catalog_answers_for_every_app_it_carries() {
        let catalog = catalog();
        let shared = shared();
        let github = CountingGitHub::default();
        let storage = catalog.storage.clone();
        let (latest, from, used) = latest_for(&catalog.apps[0], &storage, Some(&github), &shared, false, false, NOW);
        assert!(matches!(latest, Some(LatestRelease::Found(ref r)) if r.version_label() == "1.2.0"));
        assert_eq!(from, Origin::Catalog);
        assert_eq!(used, Some(Source::Catalog));
        assert_eq!(github.0.load(Ordering::SeqCst), 0, "the catalog answers, so GitHub is not asked");
    }

    #[test]
    fn an_app_without_a_release_is_coming_soon() {
        let catalog = catalog();
        let shared = shared();
        let github = CountingGitHub::default();
        let storage = catalog.storage.clone();
        let app = catalog.apps.iter().find(|a| a.release.is_none()).unwrap();
        let (latest, from, used) = latest_for(app, &storage, Some(&github), &shared, false, false, NOW);
        assert!(matches!(latest, Some(LatestRelease::None)));
        assert_eq!(from, Origin::Catalog);
        assert_eq!(used, Some(Source::Catalog));
    }

    #[test]
    fn github_is_only_the_fallback_and_is_asked_rarely() {
        let github = CountingGitHub::default();
        let shared = shared();
        // Everything has loaded, so GitHub is asked at most once an hour.
        shared.loaded.store(true, Ordering::SeqCst);
        let mut app = app();
        // The catalog knows nothing about this one, but its releases live on GitHub.
        app.release = None;
        app.github = "bd-tecnologia/exemplo".to_string();

        let (_, _, used) = latest_for(&app, "", Some(&github), &shared, false, false, NOW);
        assert_eq!(used, Some(Source::GitHubApi));
        assert_eq!(github.0.load(Ordering::SeqCst), 1);
        // Not again within the hour.
        let (_, _, used) = latest_for(&app, "", Some(&github), &shared, false, false, NOW + 30 * 60);
        assert_eq!(used, None);
        assert_eq!(github.0.load(Ordering::SeqCst), 1);
        // But Refresh asks right away, and an hour later it is due again.
        latest_for(&app, "", Some(&github), &shared, false, true, NOW + 31 * 60);
        assert_eq!(github.0.load(Ordering::SeqCst), 2);
        latest_for(&app, "", Some(&github), &shared, false, false, NOW + 31 * 60 + GITHUB_FALLBACK_EVERY);
        assert_eq!(github.0.load(Ordering::SeqCst), 3);
    }

    const APP: &str = "exemplo";

    /// A release with one file per platform and format, so whichever this machine installs is
    /// there.
    fn release(version: &str) -> Release {
        let suffixes = ["-portable.zip", ".tar.gz", ".msi", ".dmg", ".AppImage", ".deb", ".rpm", ".flatpak"];
        let tokens = [
            "windows-x64",
            "windows-x86",
            "windows-arm64",
            "macos-universal",
            "linux-x86_64",
            "linux-aarch64",
            "linux-riscv64",
            "freebsd-x86_64",
            "freebsd-aarch64",
        ];
        let assets = tokens
            .iter()
            .flat_map(|t| suffixes.iter().map(move |s| format!("{APP}-{version}-{t}{s}")))
            .map(|name| Asset { url: format!("https://example.invalid/{name}"), name, size: 3, sha256: None })
            .collect();
        Release { assets, ..Release::default() }
    }

    #[cfg(windows)]
    #[test]
    fn windows_preference_selects_portable_and_existing_installs_keep_their_format() {
        let app = app();
        let platform = Platform { os: Os::Windows, arch: Arch::X64 };
        let release = release("0.5.0");
        let portable_options = BackendOptions { kinds: vec![PackageKind::PortableZip], ..Default::default() };
        assert_eq!(planned_asset(&app, &release, platform, &portable_options, None, None).unwrap().1, PackageKind::PortableZip);
        let msi =
            Installed { version: None, source: bd_launcher_core::InstallSource::Msi, location: r"C:\Apps\Exemplo\exemplo.exe".into(), package: None };
        assert_eq!(planned_asset(&app, &release, platform, &portable_options, Some(&msi), None).unwrap().1, PackageKind::Msi);
        let launcher = Installed {
            version: None,
            source: bd_launcher_core::InstallSource::Launcher,
            location: r"D:\Craft Apps\Exemplo\exemplo.exe".into(),
            package: None,
        };
        let record = crate::manifest::Record {
            version: "0.4.0".into(),
            kind: PackageKind::PortableZip,
            path: launcher.location.clone().into(),
            installed_at: 1,
            files: vec![],
            needs_root: false,
        };
        let msi_options = BackendOptions { kinds: vec![PackageKind::Msi], ..Default::default() };
        assert_eq!(planned_asset(&app, &release, platform, &msi_options, Some(&launcher), Some(&record)).unwrap().1, PackageKind::PortableZip);
        assert_eq!(planned_asset(&app, &release, platform, &msi_options, None, Some(&record)).unwrap().1, PackageKind::PortableZip);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_custom_linux_folder_requires_appimage_without_package_fallback() {
        if process::in_sandbox() {
            return;
        }
        let app = app();
        let platform = Platform { os: Os::Linux, arch: Arch::X64 };
        let options = BackendOptions {
            kinds: vec![PackageKind::Deb],
            install_locations: bd_launcher_core::protocol::InstallLocations { root: Some("/custom/apps".into()), ..Default::default() },
            ..Default::default()
        };
        let mut release = release("0.5.0");
        assert_eq!(planned_asset(&app, &release, platform, &options, None, None).unwrap().1, PackageKind::AppImage);
        release.assets.retain(|a| !a.name.ends_with(".AppImage"));
        assert!(planned_asset(&app, &release, platform, &options, None, None).is_none());
        let existing = Installed { version: None, source: bd_launcher_core::InstallSource::Deb, location: "/usr/bin/exemplo".into(), package: None };
        let system = BackendOptions { kinds: options.kinds.clone(), ..Default::default() };
        assert_eq!(
            planned_asset(&app, &release, platform, &options, Some(&existing), None),
            planned_asset(&app, &release, platform, &system, Some(&existing), None)
        );
    }

    /// A backend that knows `release("0.5.0")` and keeps its files under `root`.
    fn backend(root: &std::path::Path, platform: Platform, source: Option<Arc<dyn ReleaseSource>>) -> NativeBackend {
        let shared = shared();
        shared.latest.lock().unwrap().insert(APP.to_string(), release("0.5.0"));
        let shared = Arc::new(shared);
        let paths = Paths::under(root);
        let catalog = CatalogStore::new(None, catalog_path(&paths.cache));
        NativeBackend { shared, paths, http: None, source, catalog, platform: Some(platform) }
    }

    /// Reports again, and returns what the UI was last told about a kept download.
    fn ready(backend: &mut NativeBackend) -> Option<bool> {
        let app = app();
        report_partial(&app, &backend.shared, &backend.paths, backend.platform, &BackendOptions::default());
        last_ready(backend.poll())
    }

    fn last_ready(events: Vec<Event>) -> Option<bool> {
        events.into_iter().rev().find_map(|e| match e {
            Event::ReadyToInstall { app, ready } if app == APP => Some(ready),
            _ => None,
        })
    }

    /// Puts the planned download on disk, as a failed install leaves it.
    fn keep_file(backend: &NativeBackend) -> std::path::PathBuf {
        let app = app();
        let (dest, size) = planned_download(&app, &backend.shared, &backend.paths, backend.platform, &BackendOptions::default()).unwrap();
        std::fs::create_dir_all(backend.paths.downloads()).unwrap();
        std::fs::write(&dest, vec![0u8; size as usize]).unwrap();
        dest
    }

    #[test]
    fn a_kept_download_is_reported_and_can_be_thrown_away() {
        let Some(platform) = native_platform() else { return };
        let dir = tempfile::tempdir().unwrap();
        let mut backend = backend(dir.path(), platform, None);
        assert_eq!(ready(&mut backend), Some(false));
        let dest = keep_file(&backend);
        assert_eq!(ready(&mut backend), Some(true));

        backend.send(Command::Cancel { app: APP.to_string() });
        assert!(!dest.exists());
        assert_eq!(last_ready(backend.poll()), Some(false));
    }

    #[test]
    fn changing_the_settings_reports_kept_downloads_again() {
        let Some(platform) = native_platform() else { return };
        let dir = tempfile::tempdir().unwrap();
        let mut backend = backend(dir.path(), platform, None);
        keep_file(&backend);
        assert_eq!(ready(&mut backend), Some(true));
        // The planned file changed under the UI; only a new report can tell it.
        backend.shared.latest.lock().unwrap().insert(APP.to_string(), release("0.6.0"));
        backend.send(Command::Configure(BackendOptions::default()));
        assert_eq!(last_ready(backend.poll()), Some(false));
    }

    #[test]
    fn a_check_that_finds_a_newer_release_ends_the_retry_offer() {
        struct Newer;
        impl ReleaseSource for Newer {
            fn latest(&self, repo: &str, _prereleases: bool) -> crate::source::Answer {
                let found = repo == "bd-tecnologia/exemplo";
                let latest = if found { LatestRelease::Found(release("0.6.0")) } else { LatestRelease::None };
                crate::source::Answer { latest, source: Source::GitHubApi }
            }
            fn cached(&self, _repo: &str, _prereleases: bool) -> Option<LatestRelease> {
                None
            }
            fn rate_limited_until(&self) -> Option<i64> {
                None
            }
            fn save(&self) {}
        }
        let Some(platform) = native_platform() else { return };
        let dir = tempfile::tempdir().unwrap();
        let mut backend = backend(dir.path(), platform, Some(Arc::new(Newer)));
        // The catalog says the app has no release but has a repository on GitHub, so the check
        // asks GitHub about it.
        let mut catalog = catalog();
        if let Some(app) = catalog.apps.first_mut() {
            app.release = None;
            app.github = "bd-tecnologia/exemplo".to_string();
        }
        std::fs::create_dir_all(backend.paths.cache.clone()).unwrap();
        std::fs::write(crate::catalog_store::catalog_path(&backend.paths.cache), serde_json::to_vec(&catalog).unwrap()).unwrap();
        backend.http = Some(Http::new().unwrap());
        keep_file(&backend);
        assert_eq!(ready(&mut backend), Some(true));

        backend.send(Command::CheckForUpdates { force: true });
        let mut events = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(20);
        while !events.iter().any(|e| matches!(e, Event::CheckFinished { .. })) {
            assert!(Instant::now() < deadline, "the check never finished");
            std::thread::sleep(Duration::from_millis(10));
            events.extend(backend.poll());
        }
        assert_eq!(last_ready(events), Some(false));
    }

    fn copy(v: &str) -> Installed {
        Installed { version: version::parse(v), source: InstallSource::Deb, location: String::new(), package: None }
    }

    #[test]
    fn a_failed_job_counts_as_installed_only_when_the_new_version_arrived() {
        let latest = version::parse("0.5.0").unwrap();
        // One app of a shared installer call went in before another failed the call.
        assert!(landed(Some(&copy("0.4.1")), Some(&copy("0.5.0")), &latest));
        assert!(landed(None, Some(&copy("0.5.0")), &latest));
        // Nothing changed, so the error stands.
        assert!(!landed(Some(&copy("0.4.1")), Some(&copy("0.4.1")), &latest));
        assert!(!landed(None, None, &latest));
        // It was current before the job ran: the job did not install it.
        assert!(!landed(Some(&copy("0.5.0")), Some(&copy("0.5.0")), &latest));
        // A copy whose version can't be read proves nothing.
        let unreadable = Installed { version: None, ..copy("0.5.0") };
        assert!(!landed(Some(&copy("0.4.1")), Some(&unreadable), &latest));
    }

    #[test]
    fn only_a_failed_shared_call_can_count_as_installed() {
        let latest = version::parse("0.5.0").unwrap();
        let (before, after) = (Some(copy("0.4.1")), Some(copy("0.5.0")));
        let landed_after = |error: &str| landed_despite(error, before.as_ref(), after.as_ref(), &latest);
        // An app's own failure stands, even if its new version shows up (an MSI or AppImage that
        // reported an error, a cancelled or dismissed prompt, a pause).
        for error in ["the Windows Installer stopped with error 1603", "couldn't copy the AppImage", CANCELLED, INSTALL_CANCELLED, PAUSED] {
            assert!(!landed_after(error), "{error}");
        }
        // A call shared with other apps may have installed this one before it failed.
        let shared = "the system installer stopped with error 100 while handling 2 apps together. Try them one at a time.";
        assert_eq!(landed_after(shared), cfg!(target_os = "linux"));
        assert!(!landed_despite(shared, before.as_ref(), before.as_ref(), &latest));
    }

    #[test]
    fn the_launcher_gives_up_waiting_and_names_the_busy_apps() {
        let shared = shared();
        let jobs = Mutex::new(HashMap::new());
        for id in [LAUNCHER_ID, APP] {
            jobs.lock().unwrap().insert(id.to_string(), Arc::new(Stop::default()));
        }
        let restarting = AtomicBool::new(false);
        let error = claim_restart(&shared, &jobs, &restarting, &Stop::default(), Duration::from_millis(300), &mut || {}).unwrap_err();
        assert!(error.contains("Exemplo was still installing") && error.contains("cancel it"), "{error}");
        assert!(!restarting.load(Ordering::SeqCst));
        assert!(restart_blocked(vec!["WordCraft".to_string(), "FilmCraft".to_string()]).contains("FilmCraft, WordCraft were still installing"));
    }

    #[test]
    fn the_launcher_restarts_only_once_every_other_job_has_ended() {
        let shared = shared();
        let jobs = Mutex::new(HashMap::new());
        for id in [LAUNCHER_ID, APP, "outro"] {
            jobs.lock().unwrap().insert(id.to_string(), Arc::new(Stop::default()));
        }
        let restarting = AtomicBool::new(false);
        let stop = Stop::default();
        let (waiting, waits) = mpsc::channel();
        std::thread::scope(|scope| {
            let (shared, jobs, restarting, stop) = (&shared, &jobs, &restarting, &stop);
            let claim = scope.spawn(move || claim_restart(shared, jobs, restarting, stop, RESTART_WAIT, &mut || waiting.send(()).unwrap()));
            waits.recv().unwrap();
            // One install ends; another is still running or queued.
            jobs.lock().unwrap().remove(APP);
            std::thread::sleep(Duration::from_millis(250));
            assert!(!claim.is_finished() && !restarting.load(Ordering::SeqCst));
            jobs.lock().unwrap().remove("outro");
            assert_eq!(claim.join().unwrap(), Ok(()));
        });
        assert!(restarting.load(Ordering::SeqCst));
        assert!(waits.try_recv().is_err(), "it says it is waiting once");

        // With nothing else running there is nothing to wait for.
        let restarting = AtomicBool::new(false);
        assert_eq!(claim_restart(&shared, &jobs, &restarting, &stop, RESTART_WAIT, &mut || panic!("it never waits")), Ok(()));
        assert!(restarting.load(Ordering::SeqCst));
    }

    #[test]
    fn a_cancelled_launcher_update_stops_waiting_and_claims_nothing() {
        let shared = shared();
        let jobs = Mutex::new(HashMap::from([(LAUNCHER_ID.to_string(), Arc::new(Stop::default())), (APP.to_string(), Arc::new(Stop::default()))]));
        let restarting = AtomicBool::new(false);
        let stop = Stop::default();
        let (waiting, waits) = mpsc::channel();
        std::thread::scope(|scope| {
            let (shared, jobs, restarting, stop) = (&shared, &jobs, &restarting, &stop);
            let claim = scope.spawn(move || claim_restart(shared, jobs, restarting, stop, RESTART_WAIT, &mut || waiting.send(()).unwrap()));
            waits.recv().unwrap();
            stop.cancel();
            assert_eq!(claim.join().unwrap(), Err(CANCELLED.to_string()));
        });
        assert!(!restarting.load(Ordering::SeqCst));
    }
}
