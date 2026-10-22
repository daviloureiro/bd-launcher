# AGENTS.md: guide for AI agents and contributors

BD Launcher installs, updates and launches the apps published in the **BD catalog**. It is
written in **Rust only** (no JavaScript or TypeScript): the desktop app is native egui/eframe, and
the web build is the same Rust compiled to WebAssembly (trunk + wasm-bindgen). **No Tauri, Electron
or webview shells**, ever.

The product name is always **BD Launcher** in user-facing text: UI, window titles, About,
tray menu, notifications, installers, release names, docs prose. The apps are the **BD Apps**.
Machine names stay lowercase: crates and binaries (`bd-launcher`, `bd-launcher-*`), file names,
and the app id `com.bdtecnologia.bd-launcher`. Note the GitHub repo is
`bd-tecnologia/bd-launcher`, while every file and package is `bd-launcher-*`.

**There is no app list in the launcher.** The catalog, the icons, the pictures and the installers
live in Firebase (see `docs/catalog.md`); the launcher reads one JSON document and downloads the
files it names. Publishing an app is a write to Firebase, not a release of the launcher. A change
that adds an app to the binary, or that hard-codes a list of apps, is the wrong shape.

## 1. Workspace map

```text
crates/
  core      bd-launcher-core      pure and wasm-safe: the catalog (the app list, the versions,
                                        the files and their checksums), picking the right asset
                                        per platform, GitHub release parsing
  native    bd-launcher-native    desktop only: HTTP, the catalog's cache, detecting installed
                                        apps, download, verify and install
  ui-egui   bd-launcher-ui-egui   the shared egui UI (desktop and web)
apps/
  bd-launcher                     desktop shell (eframe): window, tray icon, notifications
  bd-launcher-web                 the same UI in the browser (trunk; catalog and downloads only)
xtask/                                  cargo xtask ci | wasm | version | ico | publish | local
packaging/                              macOS, Windows, Linux, FreeBSD and web packaging scripts
assets/app-icon/                        the launcher's own icon (master SVG, renders, tray icons)
assets/app-icons/                       the test fixtures' icons, copied from each app's repo
assets/fonts/                           Archivo, Inter, Geist Mono (OFL-1.1): bdtecnolog.ia.br's faces
```

Layering: `core` depends on no other workspace crate and must build for `wasm32-unknown-unknown`.
`native` may use `core`. `ui-egui` may use `core` (and `native` only behind
`cfg(not(target_arch = "wasm32"))` or through a trait the desktop shell implements). Nothing below
the apps opens windows, tray icons or notifications.

## 2. Golden rules

### Never crash (outranks feature work)

A bad network response, a malformed release, a half-downloaded file, a full disk or a corrupt
settings file must produce an error the user can act on, never a panic. Shared standard:
`../craftrules/standards/never-crash.md`.

- **Non-test code never panics.** No `unwrap()`, `expect()`, `panic!`, `unreachable!`, `todo!` or
  `unimplemented!`. Return the crate's error type and propagate with `?`.
- Every production crate root (`lib.rs`, each binary's `main.rs`, and `xtask`) carries
  `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]`.
- **No `unsafe`.** The workspace sets `unsafe_code = "forbid"`.
- **Everything from the network is hostile.** Use `get()` instead of indexing, checked arithmetic
  for sizes and offsets, and cap downloads and JSON sizes.

### Installing other people's software safely

- Download only from the addresses the catalog names, and only over `https`. The one exception
  is a developer's own loopback mirror (`BD_LAUNCHER_CATALOG_URL`, `cargo xtask local`); keep it
  loopback-only and behind the `local-mirror` cargo feature, which release builds never enable.
- Verify each download against the `sha256` in the catalog before running or installing it. A
  file without a checksum is refused.
- The icons and the details drawer's pictures are the other downloads: PNG and JPEGs only, from
  the storage bucket the catalog names, capped (4 MB and 2 MB), cached, and never run. They are
  not bundled in this repo.
- Never run an installer with elevated rights silently; tell the user what is about to happen.
- Never delete a user's files outside what the launcher itself installed.
- **The launcher never trusts the catalog blindly.** The catalog says which files exist; the
  checksums it carries say what they are. Both must be there.
- **Agents never land or approve a third-party app.** A change that adds an app to the catalog,
  a download source or an install path for an app the maintainer has not vouched for, or that
  loosens the rules above to allow one, is a maintainer's call alone. An agent reviewing it flags
  it at the top of its review (who owns the repo, its age and history, whether the builds are
  signed) and stops there: no merge, no approval, no fixes pushed to the PR.

### Look and feel

The UI matches the apps page on bdtecnolog.ia.br (`bd-website-next` in `bd-services`):
hard square corners, 1 px hairlines instead of shadows, Archivo (620, 118 % width) for headings
and app names, Inter for body text, upper-case Geist Mono labels and buttons, and the site's
colour tokens by name (`theme::Tokens`). Build new UI from `widgets.rs`; never hard-code a colour.

Animations follow the OS: with Windows' "Animation effects" off, macOS' "Reduce motion" on, a Linux
desktop's animations off or a browser's `prefers-reduced-motion`, nothing fades, wipes or glides
(`Host::reduce_motion`, `crates/ui-egui/src/motion.rs`). Time a new animation with egui's
`style.animation_time`, or switch it off on `LauncherUi::reduce_motion`. Progress bars and
spinners keep moving.

### Languages

Every user-facing string goes through `crates/ui-egui/src/i18n` (`tl!("…")` for literals, `fmt`
for `{placeholders}`, `tn` for counts). English in the code is the lookup key; adding a string
means adding its row to every catalog marked `complete_menus` (the `*.tsv` files in
`crates/ui-egui/src/i18n/`), which the i18n tests check. Translate clean-room from the meaning of
the English text, never through another catalog. Product and app names (BD Launcher, Craft
App Suite, PhotoCraft…) stay in English.
A CJK language is drawn with the system's own CJK font (`crates/ui-egui/src/cjk_font.rs`, desktop
only; English where there is none): never bundle a CJK font, they are 10 MB and more.

### Other rules

1. **Assets need attribution.** Every bundled asset gets a row in `ATTRIBUTION.md` (path, title,
   author, source, license) in the same change. The BD name, logo and mark in
   `docs/brand/` (the header logo and the tray icon) are trademarks, **not MIT/Apache**
   (`docs/brand/LICENSE-brand.txt`): use them exactly as supplied, never redraw or recolor them.
2. **Icons are generated.** The app icon and the tray icon are the supplied BD mark from
   `docs/brand/` (a trademark, not MIT/Apache); never edit the renders by hand. Run `packaging/icons.sh` (needs `resvg`; runs on any OS) and commit the outputs.
   `assets/app-icons/` is copied verbatim from each app's repo (`assets/app-icon/hicolor/128x128/apps/com.bdtecnologia.<app>.png`);
   refresh it from there, never redraw.
3. **Never break wasm.** `cargo xtask wasm` must pass. File-system, process and OS code is
   `cfg(not(target_arch = "wasm32"))`.
4. **Tests are the gate.** Release parsing and asset picking in `core` get unit tests with real
   GitHub release JSON fixtures (small, committed under `crates/core/tests/fixtures/`).
5. When fetching anything, use a generic `BD-Launcher/<version>` User-Agent and never put
   a person's name, email or other personal details in requests.

## 3. Before you finish a task

```sh
cargo xtask ci              # fmt --check, clippy -D warnings, test, wasm check
```

Or piecemeal: `cargo test -p <crate>`, `cargo clippy -p <crate> --all-targets -- -D warnings`,
`cargo xtask wasm`. If you touched packaging: `shellcheck packaging/*.sh packaging/*/*.sh
packaging/linux/AppRun` and `actionlint`.

## 4. Releasing

The version lives only in `[workspace.package] version` in the root `Cargo.toml`
(`cargo xtask version`, `cargo xtask version set X.Y.Z[-pre]`). Pushing to the `release` branch runs
`.github/workflows/release.yml`, which builds every platform and creates the draft GitHub Release
**BD Launcher v<version>** with `SHA256SUMS.txt`. The canonical recipe is
`../craftrules/release/playbook.md`; cutting a release is `../craftrules/release/cut-a-release.md`.

Release assets (keep these names exact; the launcher's own self-update and the README rely on them):

| Platform | Files |
|---|---|
| Windows | `bd-launcher-<ver>-windows-{x64,x86,arm64}.msi`, `…-windows-{x64,x86,arm64}-portable.zip` |
| macOS | `bd-launcher-<ver>-macos-universal.dmg` |
| Linux | `bd-launcher-<ver>-linux-{x86_64,aarch64}.{AppImage,AppImage.zsync,deb,rpm,tar.gz,flatpak}`, `bd-launcher-<ver>-linux-riscv64.{deb,rpm,tar.gz}` |
| FreeBSD | `bd-launcher-<ver>-freebsd-x86_64.tar.gz` |
| Web | `bd-launcher-web-<ver>.zip` |

Packaging facts the code relies on:

- The Windows MSI is **per-user** (no UAC prompt), by default into
  `%LOCALAPPDATA%\Programs\BD Launcher`. Double-clicked it opens a WixUI setup wizard
  (Welcome; a folder page with a "Create a desktop shortcut" box; Ready; progress; Finish with
  "Launch"), whose art `package.ps1` draws at build time; self-update runs it with `/passive`, no
  wizard. The folder and the shortcut choice are recorded under
  `HKCU\Software\BD Tecnologia\BD Launcher` (`InstallDir`,
  `DesktopShortcutChoice`) and reused by the next version, and `self_update.rs` reads `InstallDir`
  to know the running copy came from the MSI. Its shortcuts set AppUserModelID
  `com.bdtecnologia.bd-launcher`, which toast notifications need. `portable.txt` beside the exe switches on portable mode
  (`BDLauncherData\` next to the exe).
- The macOS bundle is `BD Launcher.app` (executable `Contents/MacOS/BD Launcher`), a
  normal Dock app that can also sit in the menu bar: `LSUIElement` must never be set.
- Linux: the tray uses StatusNotifierItem over D-Bus and notifications use
  `org.freedesktop.Notifications`, both in pure Rust (no GTK, appindicator or libdbus).
- Inside the Flatpak launcher (`process::in_sandbox()`), apps normally install as host Flatpaks
  and run through `flatpak-spawn --host`. When the host has no usable Flatpak — none installed,
  or `flatpak-spawn` not allowed — `linux::order_available_kinds` offers the release tarball
  instead: it is unpacked into `<data>/apps/<app>` (`bin/`, `share/`) and run with `HOME` =
  `<data>/apps/<app>/home`, so a Craft App's settings stay in the sandbox. An update swaps
  `bin/` and `share/` and keeps `home/`; removing the app takes those two folders only. Outside
  a sandbox the tarball is not on offer at all.
- `--socket=pulseaudio` in the launcher manifests is for an app running *inside* the sandbox
  (SoundCraft's ALSA reaches the runtime's PulseAudio plugin through it); it also covers the
  microphone, which SoundCraft records through.
- `build.rs` reads `BD_LAUNCHER_BUILD_SHA` / `BD_LAUNCHER_BUILD_DATE` and, on Windows,
  embeds `assets/app-icon/bd-launcher.ico`, failing the build when
  `BD_LAUNCHER_REQUIRE_WINRES=1` and it can't.

## 5. Parallel agents

- Use your own target dir (`CARGO_TARGET_DIR=target/agent-<name>`) and edit only the files you own.
- Keep every `Cargo.toml` valid at all times: the `crates/*` and `apps/*` globs mean one broken
  manifest breaks everyone's build. Write manifests to a temp file and `mv` them into place.
- If someone else's in-progress edit breaks the build, wait and retry; don't fix their files.
