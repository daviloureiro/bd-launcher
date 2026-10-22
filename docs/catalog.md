# The catalog: how BD Launcher learns about the apps

BD Launcher has no app list inside it. Everything it shows — which apps exist, what they are
called, what colour they are, which icon they have, which version is the newest and where to
download it — comes from **one JSON document in Firebase**, and every file it downloads comes
from **one Storage bucket**.

That means publishing an app, renaming one, or shipping a new version is a write to Firebase.
No launcher is rebuilt, no release is cut, and every copy of the launcher picks the change up on
its next check.

```
                    ┌──────────────────────────────┐
                    │  Firebase Realtime Database   │
   launcher  ──────▶│  /catalog.json                │────▶ the app list
   (GET, ETag)      │  one document, read-only      │       the newest versions
                    └──────────────────────────────┘      the announcements
                    ┌──────────────────────────────┐
                    │  Firebase Storage              │
   launcher  ──────▶│  icons/  shots/  apps/<id>/    │────▶ the icons, the pictures
   (GET)            │  the bucket                   │       the installers
                    └──────────────────────────────┘
```

| What | Where | Who writes it |
|---|---|---|
| The app list, the versions, the announcements | Realtime Database, `/catalog.json` | `cargo xtask publish`, or the Firebase console |
| The icons, the app's pictures | Storage, `icons/` and `shots/` | `cargo xtask publish` |
| The installers | Storage, `apps/<id>/<version>/` | `cargo xtask publish` |

The addresses are compiled in (`catalog::CATALOG_URL`, `catalog::STORAGE_BUCKET`). The
storage side can point at another bucket on the storage service with `BD_LAUNCHER_STORAGE`;
the catalog side only trusts this project's own database (`catalog::CATALOG_HOST`) — a
`BD_LAUNCHER_CATALOG_URL` naming any other host is ignored, so nothing that can set the
environment can steer a release build at another catalog.

## The document

```jsonc
{
  "storage": "https://storage.googleapis.com/bdesk-bdtecnologia.firebasestorage.app",
  "apps": {
    "photocraft": {                          // keyed by the app's id
      "id": "photocraft",                 // the download's stem and the package id's last part
      "name": "PhotoCraft",               // what the card says
      "tagline": "Image editing",         // one line, under the name
      "color": "#2f7bf5",                 // the app's brand colour, for its surfaces
      "ink_light": "#1a5bc4",             // the same colour, for text on a light background
      "ink_dark": "#7fb0ff",              // ... and on a dark one
      "icon": "icons/photocraft.png",     // a path in the bucket, or an absolute https address
      "website": "https://bdtecnolog.ia.br/apps/photocraft",
      "github": "bd-tecnologia/photocraft",
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
        "base": "apps/photocraft/1.2.0/", // where the files are, under the bucket
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
5. The **icons and the pictures** are fetched from the bucket when they are first needed and
   kept in the cache folder. They are never run, only decoded, and a file that isn't a picture
   is refused.

The catalog is the source of truth for versions. GitHub is only asked about an app that names a
repository there and carries no release in the catalog, and at most once an hour.

## Publishing an app

One command does the whole job: it hashes the files, uploads them to the bucket and writes the
app's entry in the catalog.

```sh
cargo xtask publish --app photocraft --version 1.3.0 --dir dist/1.3.0 \
    --icon assets/icon.png --notes "What changed."
```

It:

1. hashes every file in the folder (SHA-256) and notes its size — symlinks are refused;
2. uploads only those hashed files to `apps/photocraft/1.3.0/` in the bucket (through the Firebase CLI);
3. uploads the icon to `icons/photocraft.png`;
4. writes the app's entry to `catalog/apps/photocraft` in the database.

The database write needs a token. Get one with
[`firebase login:ci`](https://firebase.tools/docs/cli) and set it — never as a command-line
flag, so it never lands in the shell history:

```sh
export BD_LAUNCHER_FIREBASE_TOKEN=<token>
cargo xtask publish --app photocraft --version 1.3.0 --dir dist/1.3.0
```

The token is sent as an `Authorization: Bearer` header, never in the URL.

Without a token the files are still uploaded and the command prints the exact JSON to paste into
the Firebase console, so nothing is lost. `--dry-run` prints what would be done and uploads
nothing.

Every copy of BD Launcher shows the new version on its next check. There is no release to cut
and no binary to ship.

### When the files are already in the bucket

`--no-upload` writes only the catalog: use it when the files were uploaded by other means (a
CI step that runs `gcloud storage rsync`, a build that pushes to the bucket itself).

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
checks the launcher's repository out beside the app (that is where the publishing tool lives)
and runs it from there. Two secrets are needed in the app's repository's settings:

- `FIREBASE_TOKEN` — the token from `firebase login:ci`.
- `FIREBASE_SERVICE_ACCOUNT` — the service account's JSON (*Project settings → Service
  accounts*). It is what gives the workflow permission to write in the database.

## Setting the project up

The rules are in this repository, ready to deploy: [`database.rules.json`](../database.rules.json)
and [`storage.rules`](../storage.rules), with [`firebase.json`](../firebase.json) pointing at
them. So the whole job is:

```sh
firebase use bd-tecnologia
firebase deploy --only database,storage
```

The project itself has to be created in the [Firebase console](https://console.firebase.google.com)
(Storage asks for the Blaze plan, which is free until the bucket's traffic grows).

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

### The bucket's rules

The files are public too: the launcher downloads them with a plain `https` request.

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
few megabytes a day. The downloads are what cost, and they are served from Storage's CDN, not
from the database.

## The icons

Each app's icon is a PNG in the bucket, at `icons/<id>.png`, or an absolute `https` address. The
launcher downloads it once, keeps it in its cache folder and draws it on the card. A card without
an icon is drawn with the app's name alone, so the icon is worth having but is not required. A
square PNG of 128 to 256 pixels is the right size; the launcher scales it.

## The pictures of the details page

The pictures an app's page shows live in the bucket too, at `shots/<id>/<key>.jpg`, where
`<key>` is the key in the app's `details.shots`. They must be JPEGs, and no more than 2 MB each.

## What still lives in the launcher

- The **brand**: the mark, the wordmark and the colours (see `docs/brand/`).
- The **interface**: the layout, the themes and the translations.
- The **installers**: what a `.msi`, a `.dmg`, an `AppImage` or a `.deb` needs to be installed
  properly on each system.

Everything else — the list of apps, their names, their colours, their icons, their pages, their
versions and their files — is in Firebase.

## Working on it locally

`cargo xtask local serve` serves a catalog and the builds from `../local-mirror` on
`http://127.0.0.1:8787/`, and `BD_LAUNCHER_CATALOG_URL` (loopback addresses only, in a build
with the `local-mirror` feature) points the launcher at it. It is the way to try a change to the
catalog's shape without publishing one.
