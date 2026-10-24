# The catalog: how BD Launcher learns about the apps

BD Launcher has no app list inside it. Everything it shows — which apps exist, what they are
called, what colour they are, which icon they have, which version is the newest and where to
download it — comes from **one JSON document in Firebase**, and the files it downloads come from
the apps' **GitHub Releases** (with icons and pictures on `raw.githubusercontent.com`). No
Storage bucket is needed.

That means publishing an app, renaming one, or shipping a new version is a write to Firebase.
No launcher is rebuilt, no release is cut, and every copy of the launcher picks the change up on
its next check.

```
                    ┌──────────────────────────────┐
                    │  Firebase Realtime Database   │
   launcher  ──────▶│  /catalog.json                │────▶ the app list
   (GET, ETag)      │  one document, read-only      │       the newest versions
                    └──────────────────────────────┘       the announcements
                    ┌──────────────────────────────┐
                    │  GitHub Releases              │
   launcher  ──────▶│  <app>/releases/download/...  │────▶ the installers
   (GET)            │  raw.githubusercontent.com    │       the icons, the pictures
                    └──────────────────────────────┘
```

| What | Where | Who writes it |
|---|---|---|
| The app list, the versions, the announcements | Realtime Database, `/catalog.json` | `cargo xtask publish`, or the Firebase console |
| The icons, the app's pictures | `raw.githubusercontent.com/<repo>/...` | the app's repository |
| The installers | GitHub Releases, `<repo>/releases/download/<tag>/` | the app's release workflow |

The addresses are compiled in (`catalog::CATALOG_URL`). The files can come from the app's own
GitHub repository — the free path that needs no Storage bucket, since icons and pictures already
live beside the code — or from a Storage bucket (`BD_LAUNCHER_STORAGE`) if a deployment prefers
one. The catalog side only trusts this project's own database (`catalog::CATALOG_HOST`): a
`BD_LAUNCHER_CATALOG_URL` naming any other host is ignored, so nothing that can set the
environment can steer a release build at another catalog.

## The document

```jsonc
{
  // "storage": "https://…",                // optional: base for relative icon/shot paths
  "apps": {
    "photocraft": {                          // keyed by the app's id
      "id": "photocraft",                 // the download's stem and the package id's last part
      "name": "PhotoCraft",               // what the card says
      "tagline": "Image editing",         // one line, under the name
      "color": "#2f7bf5",                 // the app's brand colour, for its surfaces
      "ink_light": "#1a5bc4",             // the same colour, for text on a light background
      "ink_dark": "#7fb0ff",              // ... and on a dark one
      "icon": "https://raw.githubusercontent.com/daviloureiro/photocraft/main/assets/icon.png",
      "website": "https://bdtecnolog.ia.br/apps/photocraft",
      "github": "daviloureiro/photocraft",
      "package_id": "com.bdtecnologia.photocraft",
      "categories": "Graphics;Photo;",    // the Linux menu categories
      "former_ids": [],                   // older names its files may carry
      "former_names": [],
      "details": {                        // the app's page: the story and the pictures
        "headline": "Image editing",
        "about": "The first paragraph.\n\nThe second.",
        "features": [{ "title": "Layers", "text": "Real PSD files." }],
        "shots": [{ "key": "main", "caption": "The main screen" }],
        "platforms": "macOS, Windows, Linux, web"
      },
      "release": {
        "version": "1.2.0",
        "published_at": 1791500000,       // Unix seconds
        "notes": "What changed.",
        "base": "https://github.com/daviloureiro/photocraft/releases/download/v1.2.0/",
        "assets": [
          {
            "name": "photocraft-1.2.0-windows-x64.msi",
            "size": 12345678,
            "sha256": "8a5edab282632443219e051e4ade2d1d5bbc671c781051bf1437897cbdfea0f1"
          }
        ]
      }
    }
  },
  "launcher": { /* the same shape, for BD Launcher itself */ },
  "announcements": [{ "id": "…", "title": "…", "body": "…", "link": "https://…" }],
  "alerts": [{ "id": "…", "title": "…", "body": "…" }],
  "social": [
    { "name": "YouTube", "url": "https://www.youtube.com/@bdtecnologia" },
    { "name": "Instagram", "url": "https://www.instagram.com/officialbdstudios" }
  ]
}
```

Every field is optional and read leniently: a field of the wrong type reads as missing, one bad
app never spoils the others, and an unknown field is ignored.

`social` is the list of accounts the footer shows, after the website, the GitHub and the WhatsApp
links. Only a name the interface has an icon for, and only an `https` address, are kept. When the
catalog has no `social`, the launcher's built-in list is used — so a wrong handle is a one-line
write to Firebase, not a new launcher. A `release` may be missing (an app
that isn't published yet), and an app may be listed without one (a coming-soon app).

The apps are an **object keyed by the app's id**, because that is how the Realtime Database
stores them. A plain list works too — the launcher reads both, so a catalog kept in a file is
just as good.

A ready-to-import example: [`catalog.example.json`](catalog.example.json).

### The file names matter

The launcher reads the platform and the format from the name of each file:
`<id>-<version>-<os>-<arch>[-portable].<ext>`, for example
`photocraft-1.2.0-windows-x64.msi`, `photocraft-1.2.0-linux-aarch64.AppImage` or
`photocraft-1.2.0-macos-universal.dmg`. The formats it knows are `.msi`, `-portable.zip`, `.dmg`,
`.AppImage`, `.deb`, `.rpm`, `.pkg.tar.zst`, `.flatpak` and `.tar.gz`.

The **checksum is not optional in practice**: a file without a SHA-256 is refused, because the
launcher would not know whether it got what you published.

## How the launcher checks for updates

1. **On start, and on every refresh**, the launcher asks Firebase for `/catalog.json` with the
   `ETag` it kept from the last time. An unchanged catalog costs one small `304` and nothing
   else; a changed one is a few kilobytes.
2. The answer is **cached on disk** (`<cache>/catalog.json`, with the `ETag` beside it). If
   Firebase can't be reached — offline, down, rate-limited — the launcher shows the catalog it
   last saw, with the versions it last saw. The window is never empty.
3. The catalog already says **which version is the newest** for every app, so the launcher
   doesn't ask GitHub at all: the "update available" badge is the comparison between the
   installed version and the catalog's.
4. The **download** is checked against the `sha256` in the catalog before it is installed, and a
   running app is never replaced.
5. The **icons and the pictures** are fetched from their address when they are first needed and
   kept in the cache folder. They are never run, only decoded, and a file that isn't a picture
   is refused.

The catalog is the source of truth for versions. GitHub is only asked about an app that names a
repository there and carries no release in the catalog, and at most once an hour.

## Publishing an app

One command does the whole job: it hashes the files, records where they were published and
writes the app's entry in the catalog.

```sh
cargo xtask publish --app photocraft --version 1.3.0 --dir dist/1.3.0 \
    --icon https://raw.githubusercontent.com/daviloureiro/photocraft/main/assets/icon.png \
    --base-url https://github.com/daviloureiro/photocraft/releases/download/v1.3.0 \
    --notes "What changed."
```

It:

1. hashes every file in the folder (SHA-256) and notes its size — symlinks are refused;
2. records `--base-url` as the release's base and skips uploads (the flag implies
   `--no-upload`), because the files are already on the GitHub release;
3. records the icon's `https` address (a local file is uploaded to `icons/<id>.png` instead);
4. writes the app's entry to `catalog/apps/photocraft` in the database.

The database write needs a Google OAuth access token. In CI,
[`google-github-actions/auth`](https://github.com/google-github-actions/auth) hands one to the
workflow; locally, `firebase login:print:access-token` (or `gcloud auth print-access-token`)
prints one. Set it — never as a command-line flag, so it never lands in the shell history:

```sh
export BD_LAUNCHER_FIREBASE_TOKEN=<access token>
cargo xtask publish --app photocraft --version 1.3.0 --dir dist/1.3.0 \
    --base-url https://github.com/daviloureiro/photocraft/releases/download/v1.3.0
```

The token is sent as an `Authorization: Bearer` header, never in the URL.

Without a token the command still prints the exact JSON to paste into the Firebase console, so
nothing is lost. `--dry-run` prints what would be done and changes nothing.

Every copy of BD Launcher shows the new version on its next check. There is no release to cut
and no binary to ship.

### When the files are already published

`--no-upload` writes only the catalog: use it when the files were published by other means (a
GitHub release, a CI step, a build that pushes to a bucket itself). `--base-url` implies it and
also records the address the launcher should download from.

The database's address is the one the launcher reads from
(`bd_launcher_core::catalog::CATALOG_URL`, `https://bdesk-bdtecnologia-default-rtdb.firebaseio.com`),
so the two can't drift apart. Set `BD_LAUNCHER_FIREBASE_DATABASE_URL` to point somewhere else —
the Firebase emulator, a staging database — when you want to try a change without touching the
real one.

### The app's own fields

The command **merges**: the name, the colour, the icon and the page an app already has in the
catalog stay as they are, and only its release changes. So the second time you publish an app,
the command is the same one-liner.

The first time, the catalog has no entry for the app yet, so the app needs its own fields. Give
them to the command:

```sh
cargo xtask publish --app meuapp --version 1.0.0 --dir dist/1.0.0 \
    --name "Meu App" --tagline "O que o app faz" --color "#1876ff" --icon assets/icon.png
```

or keep them in a file beside the release and point at it:

```sh
cargo xtask publish --app meuapp --version 1.1.0 --dir dist/1.1.0 --app-file app.json
```

```json
{
  "name": "Meu App",
  "tagline": "O que o app faz",
  "color": "#1876ff",
  "ink_light": "#0b51b8",
  "ink_dark": "#7ab5ff",
  "website": "https://bdtecnolog.ia.br/apps/meuapp",
  "package_id": "com.bdtecnologia.meuapp",
  "categories": "Utility;",
  "details": { "headline": "Uma linha sob o nome." }
}
```

### BD's social accounts

The footer's and the Settings dialog's social icons come from the catalog, so a wrong handle is a
one-line write to Firebase and not a new launcher:

```sh
cargo xtask publish --app meuapp --version 1.3.0 --dir dist/1.3.0 \
    --social "YouTube=https://www.youtube.com/@bdtecnologia,Instagram=https://www.instagram.com/bd"
```

Only a name the launcher has an icon for is kept (`X`, `Instagram`, `LinkedIn`, `YouTube`,
`TikTok`), and only an `https` address. Without `--social` the list in the catalog stands.

### Publishing the launcher's own release

The launcher updates itself from the catalog too, so its release is published the same way:

```sh
cargo xtask publish --launcher --version 0.7.0 --dir dist/0.7.0
```

It goes to `catalog/launcher` in the database, and the launcher picks it up like any other
update.

### Publishing from CI

[`.github/workflows/publish.yml`](../.github/workflows/publish.yml) is a template: copy it into
the repository of the app you publish, and it does the whole job when you cut a release. It
checks the launcher's repository out beside the app (that is where the publishing tool lives),
attaches the built files to the GitHub release, and publishes the catalog entry with the release's
download address. One secret is needed in the app's repository's settings:

- `FIREBASE_SERVICE_ACCOUNT` — the service account's JSON (*Project settings → Service
  accounts*). The workflow trades it for an OAuth access token, which is what gives it permission
  to write in the database.

## Setting the project up

The rules are in this repository, ready to deploy: [`database.rules.json`](../database.rules.json),
with [`firebase.json`](../firebase.json) pointing at them. The project is already set in
[`.firebaserc`](../.firebaserc). So the whole job is:

```sh
firebase use bdesk-bdtecnologia   # already the default
firebase deploy --only database
```

Storage is not part of the deployment: the files live on GitHub Releases.

### The database's rules

The catalog is public: the launcher reads it without an account. Nothing else is.

```json
{
  "rules": {
    "catalog": { ".read": true, ".write": false },
    ".read": false,
    ".write": false
  }
}
```

The launcher never writes to the database; only the publishing step does, with your own
credentials.

### The bucket's rules (optional)

Only relevant if a deployment chooses to keep the files in Storage. The rules are kept in
[`storage.rules`](../storage.rules): the files are public, and nothing is written by a client.

```
rules_version = '2';
service firebase.storage {
  match /b/{bucket}/o {
    match /{allPaths=**} { allow read: true; allow write: false; }
  }
}
```

A download is verified against the catalog's `sha256` anyway, so a tampered file is caught
before it is installed.

### Cost

The launcher reads one small document per check, with an `ETag`, so a thousand installs cost a
few megabytes a day — inside the Realtime Database's free 10 GB/month. The downloads are served
by GitHub Releases, which has no bandwidth quota for public repositories.

## The icons

Each app's icon is an absolute `https` address — usually a PNG in the app's repository
(`raw.githubusercontent.com/<repo>/…/icon.png`) — or a PNG in a bucket at `icons/<id>.png`. The
launcher downloads it once, keeps it in its cache folder and draws it on the card. A card without
an icon is drawn with the app's name alone, so the icon is worth having but is not required. A
square PNG of 128 to 256 pixels is the right size; the launcher scales it.

## The pictures of the details page

The pictures an app's page shows are resolved against the catalog's `storage` base, at
`shots/<id>/<key>.jpg`, where `<key>` is the key in the app's `details.shots`. Without a bucket,
point `storage` at a GitHub raw base that keeps that layout (for example
`https://raw.githubusercontent.com/daviloureiro/bd-launcher/main/media`). They must be JPEGs, and
no more than 2 MB each.

## What still lives in the launcher

- The **brand**: the mark, the wordmark and the colours (see `docs/brand/`).
- The **interface**: the layout, the themes and the translations.
- The **installers**: what a `.msi`, a `.dmg`, an `AppImage` or a `.deb` needs to be installed
  properly on each system.

Everything else — the list of apps, their names, their colours, their icons, their pages and
their versions — is in Firebase, and their files are on GitHub Releases.

## Working on it locally

`cargo xtask local serve` serves a catalog and the builds from `../local-mirror` on
`http://127.0.0.1:8787/`, and `BD_LAUNCHER_CATALOG_URL` (loopback addresses only, in a build
with the `local-mirror` feature) points the launcher at it. It is the way to try a change to the
catalog's shape without publishing one.
