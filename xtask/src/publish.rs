//! Publishing an app release to Firebase.
//!
//! One command does the whole job: it hashes the files, uploads them to the storage bucket and
//! writes the app's entry in the catalog, so the launcher sees the new version on its next
//! check. Nothing has to be edited by hand.
//!
//! The uploads go through the Firebase CLI (which knows the credentials); the catalog is
//! written through the Realtime Database's REST API with a token the caller provides.

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
  --project <id>    the Firebase project (default: bd-tecnologia)
  --bucket <name>   the storage bucket (default: bd-tecnologia.firebasestorage.app)
  --token <token>   a Realtime Database token (or set BD_LAUNCHER_FIREBASE_TOKEN)
  --dry-run         print what would be done and stop

The files are uploaded to apps/<id>/<version>/ in the bucket, and the catalog entry is
written to the Realtime Database at catalog/apps/<id>.
";

/// What the command was asked to do.
struct Options {
    app: String,
    version: String,
    dir: PathBuf,
    icon: Option<PathBuf>,
    notes: String,
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

    let prefix = format!("apps/{}/{}", options.app, options.version);
    println!("app:     {}", options.app);
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
        upload(&options, icon, &format!("icons/{}.png", options.app))?;
    }

    // 3. The catalog entry, in the Realtime Database.
    let entry = entry(&options, &files);
    write_catalog(&options, &entry)?;
    println!("\npublished {} {} to {}", options.app, options.version, options.app);
    Ok(())
}

/// Reads the options.
fn parse(args: &[&str]) -> Result<Options, String> {
    let mut app = None;
    let mut version = None;
    let mut dir = None;
    let mut icon = None;
    let mut notes = String::new();
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

    let app = app.ok_or_else(|| format!("`--app` is needed\n\n{USAGE}"))?;
    let version = version.ok_or_else(|| format!("`--version` is needed\n\n{USAGE}"))?;
    let dir = dir.ok_or_else(|| format!("`--dir` is needed\n\n{USAGE}"))?;
    if !app.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b)) {
        return Err(format!("`{app}` is not a usable app name: letters, digits, `-` and `_` only"));
    }
    Ok(Options { app, version, dir, icon, notes, project, bucket, token, dry_run })
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

/// The checksum, in lowercase hex.
fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// The catalog entry of the published release.
fn entry(options: &Options, files: &[File]) -> String {
    let assets: Vec<String> =
        files.iter().map(|f| format!("{{\"name\":{},\"size\":{},\"sha256\":{}}}", json_string(&f.name), f.size, json_string(&f.sha256))).collect();
    format!(
        "{{\"version\":{},\"tag\":{},\"published_at\":{},\"notes\":{},\"base\":{},\"assets\":[{}]}}",
        json_string(&options.version),
        json_string(&format!("v{}", options.version)),
        now(),
        json_string(&options.notes),
        json_string(&format!("apps/{}/{}/", options.app, options.version)),
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

/// Uploads a file or a folder to the bucket, with the Firebase CLI.
fn upload(options: &Options, source: &Path, prefix: &str) -> Result<(), String> {
    let destination = format!("gs://{}/{}", options.bucket, prefix);
    eprintln!("uploading {} to {destination}", source.display());
    let status = Command::new("firebase")
        .args(["storage:upload", "--project", &options.project, source.to_str().unwrap_or_default(), &destination])
        .status()
        .map_err(|e| format!("couldn't run the Firebase CLI ({e}); install it with `npm i -g firebase-tools` and run `firebase login`"))?;
    if !status.success() {
        return Err(format!("the Firebase CLI failed to upload {}", source.display()));
    }
    Ok(())
}

/// Writes the catalog entry in the Realtime Database.
fn write_catalog(options: &Options, entry: &str) -> Result<(), String> {
    let Some(token) = options.token.as_deref() else {
        eprintln!(
            "\nThe files are uploaded, but the catalog was not updated: no Realtime Database token.\n\
             Run the command again with `--token <token>` (or set BD_LAUNCHER_FIREBASE_TOKEN),\n\
             or write this to catalog/apps/{} in the database:\n\n{entry}\n",
            options.app
        );
        return Ok(());
    };
    let url = format!("https://{}.firebaseio.com/catalog/apps/{}.json?auth={}", options.project, options.app, token);
    eprintln!("writing the catalog entry");
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
