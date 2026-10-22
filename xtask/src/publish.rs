//! Publishing an app release to Firebase.
//!
//! One command does the whole job: it hashes the files, uploads them to the storage bucket and
//! writes the app's entry in the catalog, so the launcher sees the new version on its next
//! check. Nothing has to be edited by hand.
//!
//! The uploads go through the Firebase CLI (which knows the credentials); the catalog is
//! read and written through the Realtime Database's REST API with a token the caller provides.
//!
//! The app's entry is **merged**, not replaced: the name, the colour, the icon and the page of
//! an app that is already in the catalog stay as they are, and only its release changes. The
//! first time an app is published, its metadata comes from `--name`, `--tagline` and
//! `--color`, or from an `app.json` file beside the release.

use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

/// The Firebase project the catalog and the bucket live in.
const DEFAULT_PROJECT: &str = "bd-tecnologia";
/// The storage bucket the icons, the pictures and the downloads live in.
const DEFAULT_BUCKET: &str = "bd-tecnologia.firebasestorage.app";

const USAGE: &str = "\
usage: cargo xtask publish --app <id> --version <x.y.z> --dir <folder> [options]

  --app <id>        the app's machine name (its download files' stem)
  --version <v>     the version being published, without a leading v
  --dir <folder>    the folder with the release files
  --icon <file>     the app's icon (a PNG); uploaded to icons/<id>.png
  --notes <text>    the release notes
  --name <text>     the app's name (only needed the first time it is published)
  --tagline <text>  the app's one-line description (only needed the first time)
  --color <hex>     the app's colour, as #1876ff (only needed the first time)
  --app-file <file> a JSON file with the app's metadata (the same fields as the catalog)
  --social <list>   BD's social accounts, as Name=address,Name=address
  --launcher        publish BD Launcher's own release, not an app's
  --project <id>    the Firebase project (default: bd-tecnologia)
  --bucket <name>   the storage bucket (default: bd-tecnologia.firebasestorage.app)
  --token <token>   a Realtime Database token (or set BD_LAUNCHER_FIREBASE_TOKEN)
  --dry-run         print what would be done and stop

The files are uploaded to apps/<id>/<version>/ in the bucket, and the app's entry is
written to the Realtime Database at catalog/apps/<id>.
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
    println!("bucket:  {}", options.bucket);
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

    // 1. The files, in the bucket.
    upload(&options, &options.dir, &prefix)?;

    // 2. The icon, in the bucket.
    if let Some(icon) = &options.icon {
        upload(&options, icon, &format!("icons/{id}.png"))?;
    }

    // 3. The catalog entry, in the Realtime Database.
    let entry = entry(&options, &files);
    let path = if options.launcher { "catalog/launcher".to_string() } else { format!("catalog/apps/{}", options.app.clone().unwrap_or_default()) };
    write_catalog(&options, &path, &entry)?;

    // 4. BD's social accounts, when the run says which they are.
    if !options.social.is_empty() {
        let links: Vec<String> =
            options.social.iter().map(|(name, url)| format!("{{\"name\":{},\"url\":{}}}", json_string(name), json_string(url))).collect();
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
    let mut launcher = false;
    let mut project = DEFAULT_PROJECT.to_string();
    let mut bucket = DEFAULT_BUCKET.to_string();
    let mut token = std::env::var("BD_LAUNCHER_FIREBASE_TOKEN").ok();
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
            "--launcher" => launcher = true,
            "--project" => project = value()?,
            "--bucket" => bucket = value()?,
            "--token" => token = Some(value()?),
            "--dry-run" => dry_run = true,
            "--help" | "-h" => {
                print!("{USAGE}");
                return Err(String::new());
            }
            other => return Err(format!("unknown option `{other}`\n\n{USAGE}")),
        }
    }

    let version = version.ok_or_else(|| format!("`--version` is needed\n\n{USAGE}"))?;
    let dir = dir.ok_or_else(|| format!("`--dir` is needed\n\n{USAGE}"))?;
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
            launcher,
            project,
            bucket,
            token,
            dry_run,
        });
    }
    Ok(Options { app: None, version, dir, icon, notes, name, tagline, color, app_file, social, launcher, project, bucket, token, dry_run })
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

/// The release files, with their sizes and checksums.
fn collect(dir: &Path) -> Result<Vec<File>, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("couldn't read {}: {e}", dir.display()))?;
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("couldn't read {}: {e}", dir.display()))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
        if name.starts_with('.') || name == "SHA256SUMS.txt" {
            continue;
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
    let id = options.app.clone().unwrap_or_else(|| "bd-launcher".to_string());
    let existing = options.token.as_deref().and_then(|token| read_catalog(options, &id, token)).unwrap_or_default();
    let mut app = existing;
    // The app's own fields: what the catalog already had, then what this run says.
    set_field(&mut app, "id", &json_string(&id));
    if let Some(name) = &options.name {
        set_field(&mut app, "name", &json_string(name));
    }
    if let Some(tagline) = &options.tagline {
        set_field(&mut app, "tagline", &json_string(tagline));
    }
    if let Some(color) = &options.color {
        set_field(&mut app, "color", &json_string(color));
    }
    if let Some(file) = &options.app_file
        && let Ok(text) = std::fs::read_to_string(file)
    {
        for (key, value) in json_object(&text) {
            set_field(&mut app, &key, &value);
        }
    }
    if options.launcher {
        // The launcher's own entry is the release itself.
        return release(options, files);
    }
    set_field(&mut app, "release", &release(options, files));
    app
}

/// The release, as the catalog describes it.
fn release(options: &Options, files: &[File]) -> String {
    let id = options.app.clone().unwrap_or_else(|| "bd-launcher".to_string());
    let assets: Vec<String> =
        files.iter().map(|f| format!("{{\"name\":{},\"size\":{},\"sha256\":{}}}", json_string(&f.name), f.size, json_string(&f.sha256))).collect();
    format!(
        "{{\"version\":{},\"tag\":{},\"published_at\":{},\"notes\":{},\"base\":{},\"assets\":[{}]}}",
        json_string(&options.version),
        json_string(&format!("v{}", options.version)),
        now(),
        json_string(&options.notes),
        json_string(&format!("apps/{id}/{}/", options.version)),
        assets.join(",")
    )
}

/// A JSON string, quotes and all.
fn json_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The top-level fields of a JSON object, as `key: value` pairs (the values are JSON already).
fn json_object(text: &str) -> Vec<(String, String)> {
    let Ok(value) = serde_json_lite(text) else { return Vec::new() };
    value
}

/// A tiny JSON reader: enough for the flat objects the catalog and `app.json` hold.
fn serde_json_lite(text: &str) -> Result<Vec<(String, String)>, String> {
    let bytes: Vec<char> = text.chars().collect();
    let mut at = 0usize;
    skip_space(&bytes, &mut at);
    if at >= bytes.len() || bytes[at] != '{' {
        return Err("not an object".into());
    }
    at += 1;
    let mut out = Vec::new();
    loop {
        skip_space(&bytes, &mut at);
        if at < bytes.len() && bytes[at] == '}' {
            return Ok(out);
        }
        let key = read_string(&bytes, &mut at)?;
        skip_space(&bytes, &mut at);
        if at >= bytes.len() || bytes[at] != ':' {
            return Err("expected a colon".into());
        }
        at += 1;
        skip_space(&bytes, &mut at);
        let value = read_value(&bytes, &mut at)?;
        out.push((key, value));
        skip_space(&bytes, &mut at);
        if at < bytes.len() && bytes[at] == ',' {
            at += 1;
        }
    }
}

fn skip_space(bytes: &[char], at: &mut usize) {
    while *at < bytes.len() && bytes[*at].is_whitespace() {
        *at += 1;
    }
}

fn read_string(bytes: &[char], at: &mut usize) -> Result<String, String> {
    skip_space(bytes, at);
    if *at >= bytes.len() || bytes[*at] != '"' {
        return Err("expected a string".into());
    }
    *at += 1;
    let mut out = String::new();
    while *at < bytes.len() {
        let c = bytes[*at];
        *at += 1;
        match c {
            '"' => return Ok(out),
            '\\' => {
                let Some(next) = bytes.get(*at) else { return Err("a string that ends in the middle".into()) };
                *at += 1;
                out.push(match next {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    'b' => '\u{8}',
                    'f' => '\u{c}',
                    'u' => {
                        let hex: String = bytes.get(*at..*at + 4).ok_or("a short escape")?.iter().collect();
                        *at += 4;
                        char::from_u32(u32::from_str_radix(&hex, 16).map_err(|_| "a bad escape")?).unwrap_or('\u{fffd}')
                    }
                    other => *other,
                });
            }
            c => out.push(c),
        }
    }
    Err("a string that never ends".into())
}

fn read_value(bytes: &[char], at: &mut usize) -> Result<String, String> {
    skip_space(bytes, at);
    let Some(&first) = bytes.get(*at) else { return Err("nothing there".into()) };
    if first == '"' {
        let text = read_string(bytes, at)?;
        return Ok(json_string(&text));
    }
    if first == '{' || first == '[' {
        let close = if first == '{' { '}' } else { ']' };
        let mut depth = 0usize;
        let mut in_string = false;
        let mut escaped = false;
        let start = *at;
        while *at < bytes.len() {
            let c = bytes[*at];
            *at += 1;
            if in_string {
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == '"' {
                    in_string = false;
                }
                continue;
            }
            match c {
                '"' => in_string = true,
                '{' | '[' => depth += 1,
                '}' | ']' => {
                    depth -= 1;
                    if depth == 0 && c == close {
                        return Ok(bytes[start..*at].iter().collect());
                    }
                }
                _ => {}
            }
        }
        return Err("an object that never ends".into());
    }
    // A number, a boolean or null.
    let start = *at;
    while *at < bytes.len() && !",}]\n\r ".contains(bytes[*at]) {
        *at += 1;
    }
    if start == *at {
        return Err("an empty value".into());
    }
    Ok(bytes[start..*at].iter().collect())
}

/// Puts a field in a JSON object, replacing it when it is already there.
fn set_field(object: &mut String, key: &str, value: &str) {
    let needle: Vec<char> = format!("\"{key}\":").chars().collect();
    let chars: Vec<char> = object.chars().collect();
    // Where the field is, if it is there.
    let mut at = None;
    let mut i = 0usize;
    while i + needle.len() <= chars.len() {
        if chars[i..i + needle.len()] == needle[..] {
            at = Some(i);
            break;
        }
        i += 1;
    }
    let Some(at) = at else {
        // Not there yet: add it.
        let trimmed = object.trim_end();
        let body = trimmed.strip_suffix('}').unwrap_or(trimmed);
        let inner = body.trim_start().strip_prefix('{').unwrap_or(body).trim();
        let comma = if inner.is_empty() { "" } else { "," };
        *object = format!("{body}{comma}{}{value}}}", needle.iter().collect::<String>());
        return;
    };
    // Where the field's value ends.
    let mut i = at + needle.len();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    while i < chars.len() {
        let c = chars[i];
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        match c {
            '"' => in_string = true,
            '{' | '[' => depth += 1,
            '}' | ']' => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            ',' if depth == 0 => break,
            _ => {}
        }
        i += 1;
    }
    let mut out: String = chars[..at].iter().collect();
    out.extend(needle.iter());
    out.push_str(value);
    out.extend(chars[i..].iter());
    *object = out;
}

/// Uploads a file or a folder to the bucket, with the Firebase CLI.
fn upload(options: &Options, source: &Path, prefix: &str) -> Result<(), String> {
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

/// The app's entry in the catalog, as it is now.
fn read_catalog(options: &Options, id: &str, token: &str) -> Option<String> {
    let url = format!("https://{}.firebaseio.com/catalog/apps/{}.json?auth={}", options.project, id, token);
    let output = Command::new("curl").args(["-sS", url.as_str()]).output().ok()?;
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
             Run the command again with `--token <token>` (or set BD_LAUNCHER_FIREBASE_TOKEN),\n\
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
    let url = format!("https://{}.firebaseio.com/{path}.json?auth={}", options.project, token);
    eprintln!("writing {path}");
    let output = Command::new("curl")
        .args(["-sS", "-X", "PUT", "-H", "Content-Type: application/json", "-d", entry, &url])
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
            launcher: false,
            project: "p".into(),
            bucket: "b".into(),
            token: None,
            dry_run: true,
        }
    }

    #[test]
    fn a_field_is_added_and_replaced() {
        let mut object = String::from("{\"id\":\"meuapp\"}");
        set_field(&mut object, "name", "\"Meu App\"");
        assert_eq!(object, "{\"id\":\"meuapp\",\"name\":\"Meu App\"}");
        set_field(&mut object, "id", "\"outro\"");
        assert_eq!(object, "{\"id\":\"outro\",\"name\":\"Meu App\"}");
    }

    #[test]
    fn a_field_with_an_object_in_it_is_replaced_whole() {
        let mut object = String::from("{\"release\":{\"version\":\"1.0.0\",\"assets\":[]},\"name\":\"x\"}");
        set_field(&mut object, "release", "{\"version\":\"1.2.0\"}");
        assert_eq!(object, "{\"release\":{\"version\":\"1.2.0\"},\"name\":\"x\"}");
    }

    #[test]
    fn a_flat_json_object_is_read() {
        let fields = json_object("{\"name\":\"Meu App\",\"color\":\"#1876ff\"}");
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].0, "name");
        assert_eq!(fields[0].1, "\"Meu App\"");
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
        assert!(text.starts_with("{\"version\":\"1.2.0\""), "{text}");
        assert!(text.contains("\"base\":\"apps/bd-launcher/1.2.0/\""), "{text}");
    }

    #[test]
    fn an_apps_entry_keeps_what_it_already_had() {
        let mut opts = options();
        opts.app_file = None;
        let files = vec![File { name: "meuapp-1.2.0-windows-x64.msi".into(), size: 7, sha256: "ab".repeat(32) }];
        // The catalog already knows the app: the entry starts from what is there.
        let mut existing = String::from("{\"id\":\"meuapp\",\"name\":\"Meu App\",\"color\":\"#123456\"}");
        set_field(&mut existing, "release", &release(&opts, &files));
        assert!(existing.contains("\"name\":\"Meu App\""));
        assert!(existing.contains("\"color\":\"#123456\""));
        assert!(existing.contains("\"version\":\"1.2.0\""));
    }
}
