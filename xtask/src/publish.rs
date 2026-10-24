//! Publishing an app release to Firebase.
//!
//! One command does the whole job: it hashes the files, records where they were published and
//! writes the app's entry in the catalog, so the launcher sees the new version on its next
//! check. Nothing has to be edited by hand.
//!
//! The files go either to the storage bucket (through the Firebase CLI) or to a GitHub release,
//! whose download address is passed with `--base-url`; the catalog is read and written through
//! the Realtime Database's REST API with a token the caller provides.
//!
//! The app's entry is **merged**, not replaced: the name, the colour, the icon and the page of
//! an app that is already in the catalog stay as they are, and only its release changes. The
//! first time an app is published, its metadata comes from `--name`, `--tagline` and
//! `--color`, or from an `app.json` file beside the release.

use std::path::{Path, PathBuf};
use std::process::Command;

use bd_launcher_core::catalog::{CATALOG_URL, PROJECT, STORAGE_BUCKET};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

/// The Firebase project the catalog and the bucket live in.
const DEFAULT_PROJECT: &str = PROJECT;
/// The storage bucket the icons, the pictures and the downloads live in.
const DEFAULT_BUCKET: &str = STORAGE_BUCKET;

const USAGE: &str = "\
usage: cargo xtask publish --app <id> --version <x.y.z> --dir <folder> [options]

  --app <id>        the app's machine name (its download files' stem)
  --version <v>     the version being published, without a leading v
  --dir <folder>    the folder with the release files
  --icon <file|url> the app's icon: a PNG uploaded to icons/<id>.png, or an https address
  --notes <text>    the release notes
  --name <text>     the app's name (only needed the first time it is published)
  --tagline <text>  the app's one-line description (only needed the first time)
  --color <hex>     the app's colour, as #1876ff (only needed the first time)
  --app-file <file> a JSON file with the app's metadata (the same fields as the catalog)
  --social <list>   BD's social accounts, as Name=address,Name=address
  --base-url <url>  where the files were published (a GitHub release); implies --no-upload
  --no-upload       the files are already published; only the catalog is written
  --launcher        publish BD Launcher's own release, not an app's
  --project <id>    the Firebase project (default: bdesk-bdtecnologia)
  --bucket <name>   the storage bucket (default: bdesk-bdtecnologia.firebasestorage.app)
  --dry-run         print what would be done and stop

The Realtime Database token always comes from the `BD_LAUNCHER_FIREBASE_TOKEN`
environment variable: it is never a command-line flag (so it never lands in the
shell history or in another user's process list), and it is sent as an
`Authorization: Bearer` header (so it never lands in a URL or a proxy log).

The database's address is https://<project>.firebaseio.com. Set
BD_LAUNCHER_FIREBASE_DATABASE_URL to point somewhere else (the emulator, a
staging database).

The files go to apps/<id>/<version>/ in the bucket unless --base-url says where they
were published instead (a GitHub release, for a deployment that doesn't use Storage).
The app's entry is written to the Realtime Database at catalog/apps/<id>.
";

/// What the command was asked to do.
struct Options {
    app: Option<String>,
    version: String,
    dir: PathBuf,
    icon: Option<PathBuf>,
    notes: String,
    name: Option<String>,
    tagline: Option<String>,
    color: Option<String>,
    app_file: Option<PathBuf>,
    social: Vec<(String, String)>,
    base_url: Option<String>,
    no_upload: bool,
    launcher: bool,
    project: String,
    bucket: String,
    token: Option<String>,
    dry_run: bool,
}

/// The files of one release: their names, sizes and checksums.
struct File {
    name: String,
    size: u64,
    sha256: String,
}

pub fn run(args: &[&str]) -> Result<(), String> {
    let options = parse(args)?;
    let files = collect(&options.dir)?;
    warn_about_names(&options, &files);

    let id = options.app.clone().unwrap_or_default();
    let prefix = if options.launcher { format!("apps/bd-launcher/{}", options.version) } else { format!("apps/{id}/{}", options.version) };

    println!("app:     {}", if options.launcher { "BD Launcher".to_string() } else { id.clone() });
    println!("version: {}", options.version);
    if let Some(base) = &options.base_url {
        println!("base:    {base}");
    } else {
        println!("bucket:  {}", options.bucket);
    }
    println!("files:   {} in {}", files.len(), options.dir.display());
    for file in &files {
        println!("  {:<52} {:>10} bytes  {}", file.name, file.size, &file.sha256[..16]);
    }
    if let Some(icon) = &options.icon {
        println!("icon:    {}", icon.display());
    }
    if options.dry_run {
        println!("\n(dry run: nothing was uploaded)");
        return Ok(());
    }

    // 1. The files, in the bucket: only the files that were hashed, one by one, so the
    // bucket and the catalog can never disagree about what was published.
    if !options.no_upload {
        for file in &files {
            upload(&options, &options.dir.join(&file.name), &format!("{prefix}/{}", file.name))?;
        }

        // 2. The icon, in the bucket.
        if let Some(icon) = &options.icon
            && !is_web_address(icon)
        {
            upload(&options, icon, &format!("icons/{id}.png"))?;
        }
    }

    // 3. The catalog entry, in the Realtime Database.
    let entry = entry(&options, &files);
    let path = if options.launcher { "catalog/launcher".to_string() } else { format!("catalog/apps/{}", options.app.clone().unwrap_or_default()) };
    write_catalog(&options, &path, &entry)?;

    // 4. BD's social accounts, when the run says which they are.
    if !options.social.is_empty() {
        let links: Vec<String> = options.social.iter().map(|(name, url)| serde_json::json!({"name": name, "url": url}).to_string()).collect();
        write_catalog(&options, "catalog/social", &format!("[{}]", links.join(",")))?;
    }
    println!("\npublished {} {} to {}", if options.launcher { "BD Launcher" } else { &id }, options.version, id);
    Ok(())
}

/// Reads the options.
fn parse(args: &[&str]) -> Result<Options, String> {
    let mut app = None;
    let mut version = None;
    let mut dir = None;
    let mut icon = None;
    let mut notes = String::new();
    let mut name = None;
    let mut tagline = None;
    let mut color = None;
    let mut app_file = None;
    let mut social = Vec::new();
    let mut base_url = None;
    let mut no_upload = false;
    let mut launcher = false;
    let mut project = DEFAULT_PROJECT.to_string();
    let mut bucket = DEFAULT_BUCKET.to_string();
    let token = std::env::var("BD_LAUNCHER_FIREBASE_TOKEN").ok().map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
    let mut dry_run = false;

    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        let arg = *arg;
        let mut value = || rest.next().map(|v| (*v).to_string()).ok_or_else(|| format!("`{arg}` needs a value"));
        match arg {
            "--app" => app = Some(value()?),
            "--version" => version = Some(value()?),
            "--dir" => dir = Some(PathBuf::from(value()?)),
            "--icon" => icon = Some(PathBuf::from(value()?)),
            "--notes" => notes = value()?,
            "--name" => name = Some(value()?),
            "--tagline" => tagline = Some(value()?),
            "--color" => color = Some(value()?),
            "--app-file" => app_file = Some(PathBuf::from(value()?)),
            "--social" => social = parse_social(&value()?)?,
            "--base-url" => base_url = Some(value()?),
            "--no-upload" => no_upload = true,
            "--launcher" => launcher = true,
            "--project" => project = value()?,
            "--bucket" => bucket = value()?,
            "--dry-run" => dry_run = true,
            "--token" => {
                return Err(
                    "`--token` is gone: put the token in BD_LAUNCHER_FIREBASE_TOKEN instead, so it never lands in the shell history or another user's process list"
                        .to_string(),
                );
            }
            "--help" | "-h" => {
                print!("{USAGE}");
                return Err(String::new());
            }
            other => return Err(format!("unknown option `{other}`\n\n{USAGE}")),
        }
    }

    let version = version.ok_or_else(|| format!("`--version` is needed\n\n{USAGE}"))?;
    // The version becomes a bucket path segment: anything but a plain version could escape
    // it, so only a real version is accepted (no leading `v`, no paths).
    if version.starts_with(['v', 'V'])
        || version.bytes().any(|b| !(b.is_ascii_alphanumeric() || b".-+".contains(&b)))
        || bd_launcher_core::version::parse(&version).is_none()
    {
        return Err(format!("`{version}` is not a usable version: use `1.2.0` (or `1.2.0-rc.1`) without a leading `v`"));
    }
    let dir = dir.ok_or_else(|| format!("`--dir` is needed\n\n{USAGE}"))?;
    let base_url = match base_url {
        Some(url) => {
            let url = url.trim();
            if !url.starts_with("https://")
                || url.len() > 2000
                || url.chars().any(|c| c.is_control() || c.is_whitespace())
            {
                return Err(format!("`{url}` is not an https address the launcher can download from"));
            }
            Some(if url.ends_with('/') { url.to_string() } else { format!("{url}/") })
        }
        None => None,
    };
    // A published base means the files are already elsewhere: uploading them to the bucket
    // as well would leave the catalog pointing at one copy and the bucket holding another.
    let no_upload = no_upload || base_url.is_some();
    if !launcher {
        let app = app.ok_or_else(|| format!("`--app` is needed\n\n{USAGE}"))?;
        if !app.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b)) {
            return Err(format!("`{app}` is not a usable app name: letters, digits, `-` and `_` only"));
        }
        return Ok(Options {
            app: Some(app),
            version,
            dir,
            icon,
            notes,
            name,
            tagline,
            color,
            app_file,
            social,
            base_url,
            no_upload,
            launcher,
            project,
            bucket,
            token,
            dry_run,
        });
    }
    Ok(Options { app: None, version, dir, icon, notes, name, tagline, color, app_file, social, base_url, no_upload, launcher, project, bucket, token, dry_run })
}

/// BD's social accounts, as `Name=address,Name=address`.
fn parse_social(list: &str) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    for pair in list.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let Some((name, url)) = pair.split_once('=') else {
            return Err(format!("`{pair}` is not a social account: it should be `Name=address`"));
        };
        let (name, url) = (name.trim(), url.trim());
        if name.is_empty() || url.is_empty() {
            return Err(format!("`{pair}` is not a social account: it should be `Name=address`"));
        }
        if !url.starts_with("https://") {
            return Err(format!("`{url}` is not an https address"));
        }
        out.push((name.to_string(), url.to_string()));
    }
    Ok(out)
}

/// The release files, with their sizes and checksums. Only plain files in the folder
/// itself: symlinks are refused (their target's contents would otherwise be uploaded to the
/// public bucket under the link's name), and so are dotfiles.
fn collect(dir: &Path) -> Result<Vec<File>, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("couldn't read {}: {e}", dir.display()))?;
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("couldn't read {}: {e}", dir.display()))?;
        let path = entry.path();
        if std::fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(format!("refusing to publish {}: it is a symlink", path.display()));
        }
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
        if name.starts_with('.') || name == "SHA256SUMS.txt" {
            continue;
        }
        if name.bytes().any(|b| b == b'/' || b == b'\\') || name.contains("..") {
            return Err(format!("refusing to publish {name}: the name could escape the folder"));
        }
        let bytes = std::fs::read(&path).map_err(|e| format!("couldn't read {}: {e}", path.display()))?;
        files.push(File { name: name.to_string(), size: bytes.len() as u64, sha256: hex(&Sha256::digest(&bytes)) });
    }
    if files.is_empty() {
        return Err(format!("no files in {}", dir.display()));
    }
    files.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(files)
}

/// Warns about files the launcher will not recognise: it reads the platform and the format from
/// the name, so a file called `setup.exe` is published but never offered.
fn warn_about_names(options: &Options, files: &[File]) {
    let Some(app) = options.app.as_deref() else { return };
    for file in files {
        if !file.name.starts_with(&format!("{app}-")) {
            eprintln!("warning: {} doesn't start with `{app}-`; the launcher may not offer it", file.name);
        }
    }
}

/// The checksum, in lowercase hex.
fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// The catalog entry of the published release: the app's own fields (the ones it already had,
/// plus what this run says) and the new release.
fn entry(options: &Options, files: &[File]) -> String {
    if options.launcher {
        // The launcher's own entry is the release itself.
        return release(options, files);
    }
    let existing = options.token.as_deref().and_then(|token| {
        let id = options.app.clone().unwrap_or_else(|| "bd-launcher".to_string());
        read_catalog(options, &id, token)
    });
    build_entry(existing.as_deref(), options, files)
}

/// Merges `existing` (the app's entry in the catalog, if any) with this run's fields and the
/// new release. Anything that isn't a JSON object starts fresh, so a corrupt entry can never
/// poison the merge.
fn build_entry(existing: Option<&str>, options: &Options, files: &[File]) -> String {
    let id = options.app.clone().unwrap_or_else(|| "bd-launcher".to_string());
    let mut app: Map<String, Value> =
        existing.and_then(|text| serde_json::from_str::<Value>(text).ok()).and_then(|v| v.as_object().cloned()).unwrap_or_default();
    // The app's own fields: what the catalog already had, then what this run says.
    app.insert("id".into(), Value::String(id.clone()));
    if let Some(name) = &options.name {
        app.insert("name".into(), Value::String(name.clone()));
    }
    if let Some(tagline) = &options.tagline {
        app.insert("tagline".into(), Value::String(tagline.clone()));
    }
    if let Some(color) = &options.color {
        app.insert("color".into(), Value::String(color.clone()));
    }
    if let Some(icon) = &options.icon {
        if is_web_address(icon) {
            app.insert("icon".into(), Value::String(icon.to_string_lossy().into_owned()));
        } else if !app.contains_key("icon") {
            // The icon was uploaded to this address: the card has to know where to look for it.
            app.insert("icon".into(), Value::String(format!("icons/{id}.png")));
        }
    }
    if let Some(file) = &options.app_file
        && let Ok(text) = std::fs::read_to_string(file)
        && let Ok(Value::Object(extra)) = serde_json::from_str::<Value>(&text)
    {
        for (key, value) in extra {
            app.insert(key, value);
        }
    }
    app.insert("release".into(), release_value(options, files));
    Value::Object(app).to_string()
}

/// The release, as the catalog describes it.
fn release(options: &Options, files: &[File]) -> String {
    release_value(options, files).to_string()
}

/// The release, as a value (so the app's entry can carry it).
fn release_value(options: &Options, files: &[File]) -> Value {
    serde_json::json!({
        "version": options.version,
        "tag": format!("v{}", options.version),
        "published_at": now(),
        "notes": options.notes,
        "base": release_base(options),
        "assets": files.iter().map(|f| serde_json::json!({"name": f.name, "size": f.size, "sha256": f.sha256})).collect::<Vec<_>>(),
    })
}

/// Where the release's files are: the published address, or the release's folder in the bucket.
fn release_base(options: &Options) -> String {
    if let Some(base) = &options.base_url {
        return base.clone();
    }
    let id = options.app.clone().unwrap_or_else(|| "bd-launcher".to_string());
    format!("apps/{id}/{}/", options.version)
}

/// Whether the `--icon` value is already an address instead of a file to upload.
fn is_web_address(path: &Path) -> bool {
    path.to_string_lossy().starts_with("https://")
}

/// Uploads one file to the bucket, with the Firebase CLI.
fn upload(options: &Options, source: &Path, prefix: &str) -> Result<(), String> {
    if std::fs::symlink_metadata(source).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(format!("refusing to upload {}: it is a symlink", source.display()));
    }
    let full = format!("gs://{}/{}", options.bucket, prefix);
    eprintln!("uploading {} to {full}", source.display());
    // The CLI takes the bucket in the destination; older versions want the path alone.
    if run_firebase(source, &full, &["storage:upload", "--project", &options.project]) {
        return Ok(());
    }
    eprintln!("retrying with the path alone");
    if run_firebase(source, prefix, &["storage:upload", "--project", &options.project, "--recursive"]) {
        return Ok(());
    }
    Err(format!(
        "the Firebase CLI failed to upload {}\n(or upload it yourself: gcloud storage rsync -r {} {full})",
        source.display(),
        source.display()
    ))
}

/// Runs the Firebase CLI. `true` when it worked.
fn run_firebase(source: &Path, destination: &str, args: &[&str]) -> bool {
    let mut command = Command::new("firebase");
    command.args(args).arg(source.to_str().unwrap_or_default()).arg(destination);
    match command.status() {
        Ok(status) => status.success(),
        Err(e) => {
            eprintln!("couldn't run the Firebase CLI ({e}); install it with `npm i -g firebase-tools` and run `firebase login`");
            false
        }
    }
}

/// The database's address: the same one the launcher reads the catalog from, unless the
/// environment says otherwise.
fn database_url(_options: &Options) -> String {
    match std::env::var("BD_LAUNCHER_FIREBASE_DATABASE_URL") {
        Ok(url) if !url.trim().is_empty() => url.trim().trim_end_matches('/').to_string(),
        _ => CATALOG_URL.trim_end_matches('/').rsplit_once('/').map_or_else(|| CATALOG_URL.to_string(), |(base, _)| base.to_string()),
    }
}

/// The app's entry in the catalog, as it is now.
fn read_catalog(options: &Options, id: &str, token: &str) -> Option<String> {
    let url = format!("{}/catalog/apps/{id}.json", database_url(options));
    let output = Command::new("curl").args(["-sS", "-H", &format!("Authorization: Bearer {token}"), url.as_str()]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() || text == "null" {
        return None;
    }
    Some(text)
}

/// Writes a part of the catalog in the Realtime Database.
fn write_catalog(options: &Options, path: &str, entry: &str) -> Result<(), String> {
    let Some(token) = options.token.as_deref() else {
        eprintln!(
            "\nThe files are uploaded, but the catalog was not updated: no Realtime Database token.\n\
             Set BD_LAUNCHER_FIREBASE_TOKEN and run the command again,\n\
             or write this to {path} in the database:\n\n{entry}\n"
        );
        if !entry.contains("\"name\"") && !options.launcher {
            eprintln!(
                "The app has no name yet: add one with `--name \"Meu App\"` (and `--tagline`,\n\
                 `--color`), or with an `--app-file`, so the card has something to show.\n"
            );
        }
        return Ok(());
    };
    let url = format!("{}/{path}.json", database_url(options));
    eprintln!("writing {path}");
    let output = Command::new("curl")
        .args(["-sS", "-X", "PUT", "-H", "Content-Type: application/json", "-H", &format!("Authorization: Bearer {token}"), "-d", entry, &url])
        .output()
        .map_err(|e| format!("couldn't run curl ({e})"))?;
    if !output.status.success() {
        return Err("the catalog could not be written".into());
    }
    let body = String::from_utf8_lossy(&output.stdout);
    if body.contains("Permission denied") || body.contains("\"error\"") {
        return Err(format!("the catalog was not written: {body}"));
    }
    Ok(())
}

/// Unix seconds now.
fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> Options {
        Options {
            app: Some("meuapp".into()),
            version: "1.2.0".into(),
            dir: PathBuf::from("dist"),
            icon: None,
            notes: "O que mudou".into(),
            name: None,
            tagline: None,
            color: None,
            app_file: None,
            social: Vec::new(),
            base_url: None,
            no_upload: false,
            launcher: false,
            project: "p".into(),
            bucket: "b".into(),
            token: None,
            dry_run: true,
        }
    }

    #[test]
    fn the_database_is_the_one_the_launcher_reads() {
        // The publishing step and the launcher have to agree: the same document, in the same
        // database, or the launcher never sees what was published.
        let base = database_url(&options());
        assert!(CATALOG_URL.starts_with(&base), "{CATALOG_URL} is not under {base}");
        assert_eq!(base, "https://bdesk-bdtecnologia-default-rtdb.firebaseio.com");
    }

    #[test]
    fn only_plain_versions_become_bucket_paths() {
        let ok = ["1.2.0", "1.2.0-rc.1", "0.7.0+build"];
        for v in ok {
            assert!(parse(&["--app", "x", "--version", v, "--dir", "d"]).is_ok(), "{v}");
        }
        for v in ["v1.2.0", "../escape", "1.2.0/../../x", "1.2 0", "1.2.0\n", ""] {
            assert!(parse(&["--app", "x", "--version", v, "--dir", "d"]).is_err(), "{v:?} must be refused");
        }
    }

    #[test]
    fn the_token_never_comes_from_the_command_line() {
        // `--token` would land in the shell history and in another user's process list.
        assert!(parse(&["--app", "x", "--version", "1.0.0", "--dir", "d", "--token", "abc"]).is_err());
    }

    #[test]
    fn an_apps_entry_keeps_what_it_already_had() {
        let mut opts = options();
        opts.app_file = None;
        let files = vec![File { name: "meuapp-1.2.0-windows-x64.msi".into(), size: 7, sha256: "ab".repeat(32) }];
        // The catalog already knows the app: the entry starts from what is there.
        let text = build_entry(Some("{\"id\":\"meuapp\",\"name\":\"Meu App\",\"color\":\"#123456\"}"), &opts, &files);
        let value: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["name"], "Meu App");
        assert_eq!(value["color"], "#123456");
        assert_eq!(value["release"]["version"], "1.2.0");
    }

    #[test]
    fn a_corrupt_existing_entry_starts_fresh() {
        let opts = options();
        let files = vec![File { name: "meuapp-1.2.0-windows-x64.msi".into(), size: 7, sha256: "ab".repeat(32) }];
        for existing in ["", "not json", "[\"an\",\"array\"]", "42", "{\"unclosed\":"] {
            let text = build_entry(Some(existing), &opts, &files);
            let value: Value = serde_json::from_str(&text).unwrap();
            assert_eq!(value["id"], "meuapp");
            assert_eq!(value["release"]["version"], "1.2.0", "{existing:?}");
        }
        let text = build_entry(None, &opts, &files);
        let value: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["id"], "meuapp");
    }

    #[test]
    fn the_new_release_replaces_the_old_one_whole() {
        let opts = options();
        let files = vec![File { name: "meuapp-1.2.0-windows-x64.msi".into(), size: 9, sha256: "cd".repeat(32) }];
        let old = "{\"id\":\"meuapp\",\"release\":{\"version\":\"1.0.0\",\"assets\":[{\"name\":\"old.msi\"}],\"extra\":\"gone\"}}";
        let text = build_entry(Some(old), &opts, &files);
        let value: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["release"]["version"], "1.2.0");
        assert_eq!(value["release"]["assets"][0]["name"], "meuapp-1.2.0-windows-x64.msi");
        assert!(value["release"].get("extra").is_none(), "{text}");
    }

    #[test]
    fn an_app_file_merges_nested_objects() {
        let dir = std::env::temp_dir().join(format!("bd-publish-test-{}-appfile", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("app.json");
        std::fs::write(&file, "{\"name\":\"Meu App\",\"details\":{\"shots\":[{\"image\":\"a.png\"}],\"featured\":true}}").unwrap();
        let mut opts = options();
        opts.app_file = Some(file);
        let files = vec![File { name: "meuapp-1.2.0-windows-x64.msi".into(), size: 7, sha256: "ab".repeat(32) }];
        let text = build_entry(Some("{\"id\":\"meuapp\",\"name\":\"Old\"}"), &opts, &files);
        let value: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["name"], "Meu App");
        assert_eq!(value["details"]["shots"][0]["image"], "a.png");
        assert_eq!(value["details"]["featured"], true);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_social_accounts_are_read_and_the_bad_ones_refused() {
        let accounts = parse_social("YouTube=https://youtube.com/@bd, Instagram=https://instagram.com/bd").unwrap();
        assert_eq!(
            accounts,
            [("YouTube".to_string(), "https://youtube.com/@bd".to_string()), ("Instagram".to_string(), "https://instagram.com/bd".to_string())]
        );
        assert!(parse_social("YouTube").is_err(), "a name without an address");
        assert!(parse_social("YouTube=ftp://x").is_err(), "an address that isn't https");
        assert!(parse_social("=https://x").is_err(), "an account without a name");
    }

    #[test]
    fn the_release_carries_the_files_and_their_checksums() {
        let files = vec![File { name: "meuapp-1.2.0-windows-x64.msi".into(), size: 7, sha256: "ab".repeat(32) }];
        let text = release(&options(), &files);
        assert!(text.contains("\"version\":\"1.2.0\""));
        assert!(text.contains("\"base\":\"apps/meuapp/1.2.0/\""));
        assert!(text.contains("\"name\":\"meuapp-1.2.0-windows-x64.msi\""));
        assert!(text.contains(&"ab".repeat(32)));
    }

    #[test]
    fn the_launchers_own_entry_is_the_release_itself() {
        let mut opts = options();
        opts.launcher = true;
        opts.app = None;
        let files = vec![File { name: "bd-launcher-1.2.0-macos-universal.dmg".into(), size: 7, sha256: "ab".repeat(32) }];
        let text = entry(&opts, &files);
        let value: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["version"], "1.2.0");
        assert_eq!(value["base"], "apps/bd-launcher/1.2.0/");
    }

    #[test]
    fn the_icon_is_registered_when_one_is_published() {
        let mut opts = options();
        opts.icon = Some(PathBuf::from("icon.png"));
        let files = vec![File { name: "meuapp-1.2.0-windows-x64.msi".into(), size: 7, sha256: "ab".repeat(32) }];
        let text = build_entry(None, &opts, &files);
        let value: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["icon"], "icons/meuapp.png");
        // An icon the catalog already has is kept, not overwritten.
        let text = build_entry(Some("{\"id\":\"meuapp\",\"icon\":\"icons/custom.png\"}"), &opts, &files);
        let value: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["icon"], "icons/custom.png");
    }

    #[test]
    fn a_published_base_replaces_the_bucket_path() {
        let mut opts = options();
        opts.base_url = Some("https://github.com/daviloureiro/meuapp/releases/download/v1.2.0/".into());
        let files = vec![File { name: "meuapp-1.2.0-windows-x64.msi".into(), size: 7, sha256: "ab".repeat(32) }];
        let text = build_entry(None, &opts, &files);
        let value: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["release"]["base"], "https://github.com/daviloureiro/meuapp/releases/download/v1.2.0/");
    }

    #[test]
    fn a_published_base_skips_the_upload() {
        let parsed = parse(&[
            "--app", "meuapp", "--version", "1.2.0", "--dir", ".",
            "--base-url", "https://github.com/daviloureiro/meuapp/releases/download/v1.2.0",
        ])
        .unwrap();
        assert!(parsed.no_upload);
        assert_eq!(release_base(&parsed), "https://github.com/daviloureiro/meuapp/releases/download/v1.2.0/");
    }

    #[test]
    fn a_published_base_must_be_https() {
        assert!(parse(&[
            "--app", "meuapp", "--version", "1.2.0", "--dir", ".",
            "--base-url", "http://example.com/files",
        ])
        .is_err());
    }

    #[test]
    fn an_icon_address_is_used_as_it_is() {
        let mut opts = options();
        opts.icon = Some(PathBuf::from("https://raw.githubusercontent.com/daviloureiro/meuapp/main/assets/icon.png"));
        let files = vec![File { name: "meuapp-1.2.0-windows-x64.msi".into(), size: 7, sha256: "ab".repeat(32) }];
        let text = build_entry(None, &opts, &files);
        let value: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["icon"], "https://raw.githubusercontent.com/daviloureiro/meuapp/main/assets/icon.png");
    }
}
