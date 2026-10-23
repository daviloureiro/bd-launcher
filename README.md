<p align="center">
  <a href="https://bdtecnolog.ia.br/">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="docs/brand/bd-logo-white.svg">
      <img alt="BD" src="docs/brand/bd-logo.svg" width="200">
    </picture>
  </a>
</p>

<h1 align="center">BD Launcher</h1>

<p align="center">
  <b>One place to install, update and open every app you publish, rebuilt in pure Rust.</b><br>
  The right build for your computer, kept up to date. The list of apps, their icons and their<br>
  versions live in Firebase, so publishing an app is a write, not a new launcher.
</p>

<p align="center">
  <img alt="100% Rust" src="https://img.shields.io/badge/100%25-Rust-b7410e?style=flat-square&logo=rust">
  <img alt="macOS · Windows · Linux · FreeBSD · Web" src="https://img.shields.io/badge/macOS%20%C2%B7%20Windows%20%C2%B7%20Linux%20%C2%B7%20FreeBSD%20%C2%B7%20Web-native-26282e?style=flat-square">
  <img alt="License: MIT OR Apache-2.0" src="https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-3a3a3a?style=flat-square">
  <img alt="Status: early alpha" src="https://img.shields.io/badge/status-early%20alpha-d69e2e?style=flat-square">
  <br>
  <a href="https://sonarcloud.io/project/overview?id=bd-tecnologia_bd-launcher"><img alt="Scanned by SonarCloud" src="https://img.shields.io/badge/Scanned_by-SonarCloud-4b9fd5?style=flat-square&logo=sonarqubecloud&logoColor=white"></a>
  <a href="https://coderabbit.ai"><img alt="Reviewed by CodeRabbit" src="https://img.shields.io/coderabbit/prs/github/bd-tecnologia/bd-launcher?style=flat-square&label=Reviewed%20by%20CodeRabbit&logo=coderabbit"></a>
</p>

<p align="center">
  <a href="https://wa.me/5511963080423"><img alt="Talk to BD on WhatsApp" src="https://img.shields.io/badge/WhatsApp-25D366?style=for-the-badge&logo=whatsapp&logoColor=white" height="40"></a>
</p>

<p align="center">
  <a href="https://github.com/daviloureiro/bd-launcher/releases/latest"><b>Download BD Launcher</b></a> ·
  <a href="https://bdtecnolog.ia.br/">BD</a> ·
  <a href="https://bdtecnolog.ia.br/apps">The BD Apps</a>
</p>

> [!NOTE]
> **BD is a community of artists from all walks of life.** Painters, photographers,
> filmmakers, illustrators, designers, animators, hobbyists, and people who picked up a pencil
> last week. If you make things, you're one of us. **[Come say hi on WhatsApp](https://wa.me/5511963080423).**

<p align="center">
  <a href="#what-it-does">What it does</a> ·
  <a href="#features">Features</a> ·
  <a href="#downloads">Downloads</a> ·
  <a href="#build-from-source">Build from source</a> ·
  <a href="#the-craft-app-suite">BD Apps</a> ·
  <a href="https://wa.me/5511963080423">WhatsApp</a>
</p>

<p align="center">
  <img src="docs/screenshots/bd-launcher-grid.png" alt="BD Launcher on macOS in the grid view (dark theme): a card for each Craft App with its icon, status, version, release date and an Install or Open button" width="900">
</p>
<p align="center">
  <img src="docs/screenshots/bd-launcher-table.png" alt="The same apps in the table view (light theme), with columns for each app's status, installed and latest versions, and release date" width="900"><br>
  <sub>Switch between the grid and a sortable table.</sub>
</p>

## What it does

The Craft apps each ship their own builds for every platform. BD Launcher saves you from
hunting through a dozen release pages: it lists every app in the suite, shows which ones you have
and which version, installs the right build for your operating system and processor, and tells
you when an update is out. It is a small native app that can sit in your menu bar or system tray.

## Features

<table>
  <tr>
    <td width="33%" valign="top">
      <h3>📦 Install in one click</h3>
      Picks the right download for your OS and CPU (DMG, MSI, AppImage, .deb, .rpm and more), checks it against the release's SHA-256 checksums and installs it.
    </td>
    <td width="33%" valign="top">
      <h3>🔄 Stay up to date</h3>
      Checks each app's GitHub releases and updates the ones you have. A notification tells you when there's something new.
    </td>
    <td width="33%" valign="top">
      <h3>🧭 Always at hand</h3>
      Opens any Craft app from one window, or from the tray and menu-bar icon. Native egui, no Electron, no web view.
    </td>
  </tr>
</table>

The web version shows the same catalog in the browser, with the latest download for each app.

## How it knows about the apps

There is no list of apps inside the launcher. One JSON document in **Firebase** carries
everything: which apps exist, what they are called, what colour they are, which icon they have,
which version is the newest and where to download it. The icons, the pictures and the installers
live in **Firebase Storage**. So publishing an app or shipping a new version is a write to
Firebase, not a new launcher: every copy picks the change up on its next check, and keeps
working from its last copy when Firebase can't be reached.

See [`docs/catalog.md`](docs/catalog.md) for the document's shape, how the launcher checks for
updates, and the Firebase rules to set.

## Interface languages

The interface is in English, Brazilian Portuguese (Português (Brasil)), Czech (Čeština), French
(Français), German (Deutsch), Hungarian (Magyar), Japanese (日本語), Polish (Polski), Russian
(Русский), Simplified Chinese (简体中文) or Spanish (Español). **Auto**, the default, follows the
system or browser language; **Settings → Language** picks one and switches at once, and the choice
is kept between runs. Czech locales (`cs-CZ`, `cs_CZ.UTF-8`…) select Czech, German ones (`de-DE`,
`de-AT`, `de-CH`, `de_DE.UTF-8`…) German, French ones (`fr-FR`, `fr-CA`…) French, Hungarian ones
(`hu`, `hu-HU`, `hu_HU.UTF-8`…) Hungarian, Japanese ones (`ja`, `ja-JP`, `ja_JP.UTF-8`…) Japanese,
Polish ones (`pl`, `pl-PL`, `pl_PL.UTF-8`) Polish, Spanish ones (`es-ES`, `es-MX`, `es-419`…)
Spanish, and Portuguese ones (`pt-BR`, and `pt-PT` too, for now) Brazilian Portuguese. Simplified
Chinese locales (`zh-CN`, `zh-SG`, `zh-Hans`, and POSIX forms such as `zh_CN.UTF-8`) select
Chinese; Traditional Chinese ones don't. To try a language without changing the setting, start the
desktop app with `BD_LAUNCHER_LOCALE=pt-BR`, or add `?lang=ru` to the web address.

The CJK languages are drawn with the system's own CJK font (Hiragino Sans or PingFang on macOS,
Yu Gothic, Meiryo or Microsoft YaHei on Windows, Noto Sans CJK, IPA, Takao or WenQuanYi on Linux and
FreeBSD). Where the system has no font for one, and in the web version, the interface stays in
English and Settings says why. Product names and release notes stay as published; text without a
translation falls back to English.

## Where apps are installed

BD Launcher installs each app where your system normally keeps applications, using the
app's official release package, so an app installed by the launcher is the same as one you
installed by hand (and the launcher finds apps you installed yourself):

| System | Where | How |
|---|---|---|
| macOS | `/Applications/<App>.app` | Copied from the signed, notarized release DMG |
| Windows | `C:\Program Files\<App>` | The release MSI, through Windows Installer (with Start menu entry and Apps & features entry) |
| Linux | `/usr/bin`, `/usr/share/applications` | The release `.deb` (apt) or `.rpm` (dnf, zypper), with your system's password prompt. Without those, a Flatpak, or an AppImage in `~/Applications` |
| Linux, inside the launcher's Flatpak, with no Flatpak on the host | `~/.var/app/com.bdtecnologia.bd-launcher/data/bd-launcher/apps/<app>` | The release tarball, unpacked into the launcher's own folder and run inside the same sandbox, with its settings in a `home/` folder beside it. Nothing is installed on the host, and each app's configuration stays in the sandbox |
| FreeBSD | `/usr/local` | The release tree, with a password prompt (polkit); `~/.local` when polkit isn't installed |

BD Launcher keeps itself current too: when a new version is out, **Update and restart**
downloads it, verifies it, puts it where this copy is installed (the `.app`, the Windows Installer
package or portable folder, the AppImage, or the `.deb`/`.rpm`) and reopens it.

Updates use the same format an app was installed with. Every download is checked against the
release's published SHA-256 before it is installed, and a running app is never replaced.

The launcher writes its log to `logs/bd-launcher.log` in its data folder
(`~/.local/share/bd-launcher` on Linux, `~/Library/Application Support/BD Launcher`
on macOS, `%LOCALAPPDATA%\BD Launcher` on Windows, `BDLauncherData` beside the
portable zip's executable) and keeps the previous two runs as `bd-launcher.1.log` and
`bd-launcher.2.log`. `RUST_LOG` changes what it records (`debug`, `info,zbus=warn`, …).

## Downloads

Download the build for your computer from the [latest release](https://github.com/daviloureiro/bd-launcher/releases/latest), or browse [all releases](https://github.com/daviloureiro/bd-launcher/releases). Every release has a `SHA256SUMS.txt` to check the files against. `<ver>` is the version number, for example `0.1.0`.

**Windows** (10 and 11)

| File | What it is |
|---|---|
| `bd-launcher-<ver>-windows-x64.msi` | Installer for most PCs (Intel and AMD, 64-bit). Installs for the current user, no admin rights needed |
| `bd-launcher-<ver>-windows-arm64.msi` | Installer for Windows on Arm (Snapdragon and other Arm PCs) |
| `bd-launcher-<ver>-windows-x86.msi` | Installer for 32-bit Windows |
| `bd-launcher-<ver>-windows-{x64,arm64,x86}-portable.zip` | No install: unzip and run `bd-launcher.exe`. Keeps its settings in the folder |

**macOS** (11 Big Sur or later)

| File | What it is |
|---|---|
| `bd-launcher-<ver>-macos-universal.dmg` | Universal app for Apple silicon and Intel Macs, signed and notarized. Drag BD Launcher to Applications |

**Linux** (x86_64 and aarch64; glibc 2.35 or newer, e.g. Ubuntu 22.04+, Debian 12+, Fedora 36+)

| File | What it is |
|---|---|
| `bd-launcher-<ver>-linux-{x86_64,aarch64}.AppImage` | Runs on almost any distro without installing; `chmod +x` and run. The `.AppImage.zsync` beside it lets AppImageUpdate fetch only what changed |
| `bd-launcher-<ver>-linux-{x86_64,aarch64}.deb` | Debian, Ubuntu, Mint, Pop!_OS: `sudo apt install ./bd-launcher-…deb` |
| `bd-launcher-<ver>-linux-{x86_64,aarch64}.rpm` | Fedora, openSUSE, RHEL: `sudo dnf install ./bd-launcher-…rpm` |
| `bd-launcher-<ver>-linux-{x86_64,aarch64}.flatpak` | Flatpak bundle: `flatpak install --user bd-launcher-…flatpak`. Inside the sandbox the launcher installs apps as Flatpaks on the host through `flatpak-spawn --host`; where the host has no Flatpak it falls back to each app's release tarball, unpacked and run inside the sandbox with the app's settings kept in it. Either way it can only reach the host as far as that permission allows; the AppImage, .deb and .rpm builds are the recommended way to run the launcher on Linux |
| `bd-launcher-<ver>-linux-{x86_64,aarch64}.tar.gz` | Plain tree (`bin/`, `share/`) for everyone else and for scripts |

RISC-V (riscv64): `bd-launcher-<ver>-linux-riscv64.{deb,rpm,tar.gz}`, built against Ubuntu 24.04 (needs glibc 2.39+).

The tray icon uses the StatusNotifierItem standard: KDE, Xfce, Cinnamon and most other desktops show it; on GNOME install the AppIndicator extension.

**FreeBSD** (14, x86_64)

| File | What it is |
|---|---|
| `bd-launcher-<ver>-freebsd-x86_64.tar.gz` | A tree laid out like `/usr/local` |

```sh
pkg install libxkbcommon wayland libX11 libXcursor libXrandr libXi libxcb mesa-libs vulkan-loader fontconfig freetype2
tar -xzf bd-launcher-<ver>-freebsd-x86_64.tar.gz --strip-components 1 -C /usr/local
bd-launcher
```

**Web (WebAssembly)**

| File | What it is |
|---|---|
| `bd-launcher-web-<ver>.zip` | The catalog as a static site you can host anywhere; see [`packaging/web/README.md`](packaging/web/README.md) |

## Build from source

You need a recent stable Rust (1.90 or newer). On Linux, install the windowing headers first
(Debian/Ubuntu: `sudo apt install libxkbcommon-dev libwayland-dev libx11-dev libxrandr-dev libxi-dev libgl1-mesa-dev`).

```sh
git clone https://github.com/daviloureiro/bd-launcher
cd bd-launcher
cargo run --release -p bd-launcher   # the desktop app
cargo xtask ci                             # fmt, clippy, tests and the wasm check
```

The web version builds with [trunk](https://trunk-rs.github.io/trunk/):

```sh
rustup target add wasm32-unknown-unknown
cd apps/bd-launcher-web && trunk serve
```

### Nix (experimental)

`flake.nix` gives a dev shell (`nix develop`, then the cargo commands above) and a package:

```sh
nix run github:bd-tecnologia/bd-launcher
```

On NixOS this uses the system's graphics drivers. On other distros, a Nix-built program can't load
the host's GPU drivers, so the window may fail to open. Run it through
[nixGL](https://github.com/nix-community/nixGL) instead (NVIDIA needs nixGL's matching NVIDIA
variant), or use the AppImage, `.deb` or `.rpm` release:

```sh
nix run --impure github:nix-community/nixGL -- nix run github:bd-tecnologia/bd-launcher
```

### Installing your own builds

To install and update the Craft apps from checkouts on your machine instead of GitHub, clone them
next to this repo (`../photocraft`, `../wordcraft`, …) and run:

```sh
cargo xtask local sync      # git pull every app, build and package each new version into ../local-mirror
cargo xtask local serve     # serve the builds and a release feed on http://127.0.0.1:8787/
BD_LAUNCHER_DOWNLOAD_BASE=http://127.0.0.1:8787/ cargo run --release -p bd-launcher --features local-mirror
```

Run `sync` again whenever you want updates, or let the server do it: `cargo xtask local serve
--sync-every 1h` pulls and builds on that interval while it serves. An app is rebuilt when its
checkout's version isn't in the mirror yet (`--rebuild` forces it), and the launcher's next check
picks it up. Add `--every-commit` to build every new commit instead, as a dev version between
releases (`0.4.0` builds as `0.4.1-dev.<commit time>`), which the launcher offers as an update. While
`BD_LAUNCHER_DOWNLOAD_BASE` is set (loopback addresses only), the launcher asks only the
mirror, never GitHub. Only builds with the `local-mirror` feature read it; releases never have it. `cargo xtask local help` lists the options (`--only`, `--no-pull`, `--arch`, …).

New contributors and AI agents: start with [`AGENTS.md`](AGENTS.md). Release packaging lives in
[`packaging/`](packaging/) and [`.github/workflows/release.yml`](.github/workflows/release.yml).

> [!IMPORTANT]
> **Status:** BD Launcher is new and in early alpha. Expect rough edges, and please file
> issues (include your OS, which app you were installing and the launcher's log file) or tell us on
> [WhatsApp](https://wa.me/5511963080423).

## The BD Apps

BD Launcher installs the apps published to the **BD catalog**: the tools from the
[BD](https://bdtecnolog.ia.br/) team, each written from scratch in Rust and each able to
stand on its own — and any other app you publish there.

The list is not in this readme because it is not in the launcher either: it is the **catalog**, one
JSON document in Firebase that every copy of BD Launcher reads. Adding an app, renaming one or
shipping a new version is a write to that document.

| | App | What it's for | Code | Learn more |
|:-:|---|---|---|---|
| <img src="https://raw.githubusercontent.com/bd-tecnologia/photocraft/main/assets/app-icon/hicolor/64x64/apps/com.bdtecnologia.photocraft.png" alt="" width="32" height="32"> | **PhotoCraft** | Image editing: layers, masks, type and real PSD files | [GitHub](https://github.com/bd-tecnologia/photocraft) | [Website](https://bdtecnolog.ia.br/apps/photocraft) |
| <img src="https://raw.githubusercontent.com/bd-tecnologia/vectorcraft/main/assets/app-icon/hicolor/64x64/apps/com.bdtecnologia.vectorcraft.png" alt="" width="32" height="32"> | **VectorCraft** | Vector illustration | [GitHub](https://github.com/bd-tecnologia/vectorcraft) | [Website](https://bdtecnolog.ia.br/apps/vectorcraft) |
| <img src="https://raw.githubusercontent.com/bd-tecnologia/filmcraft/main/assets/app-icon/hicolor/64x64/apps/com.bdtecnologia.filmcraft.png" alt="" width="32" height="32"> | **FilmCraft** | Video editing, color and sound | [GitHub](https://github.com/bd-tecnologia/filmcraft) | [Website](https://bdtecnolog.ia.br/apps/filmcraft) |
| <img src="https://raw.githubusercontent.com/bd-tecnologia/lightcraft/main/assets/app-icon/hicolor/64x64/apps/com.bdtecnologia.lightcraft.png" alt="" width="32" height="32"> | **LightCraft** | Photo library and raw development | [GitHub](https://github.com/bd-tecnologia/lightcraft) | [Website](https://bdtecnolog.ia.br/apps/lightcraft) |
| <img src="https://raw.githubusercontent.com/bd-tecnologia/pdfcraft/main/assets/app-icon/hicolor/64x64/apps/com.bdtecnologia.pdfcraft.png" alt="" width="32" height="32"> | **PdfCraft** | Reading, organizing and protecting PDFs | [GitHub](https://github.com/bd-tecnologia/pdfcraft) | [Website](https://bdtecnolog.ia.br/apps/pdfcraft) |
| <img src="https://raw.githubusercontent.com/bd-tecnologia/effectcraft/main/assets/app-icon/hicolor/64x64/apps/com.bdtecnologia.effectcraft.png" alt="" width="32" height="32"> | **EffectCraft** | Motion graphics and visual effects | [GitHub](https://github.com/bd-tecnologia/effectcraft) | [Website](https://bdtecnolog.ia.br/apps/effectcraft) |
| <img src="https://raw.githubusercontent.com/bd-tecnologia/designcraft/main/assets/app-icon/hicolor/64x64/apps/com.bdtecnologia.designcraft.png" alt="" width="32" height="32"> | **DesignCraft** | Page layout and publishing | [GitHub](https://github.com/bd-tecnologia/designcraft) | [Website](https://bdtecnolog.ia.br/apps/designcraft) |
| <img src="https://raw.githubusercontent.com/bd-tecnologia/figcraft/main/assets/app-icon/hicolor/64x64/apps/com.bdtecnologia.figcraft.png" alt="" width="32" height="32"> | **FigCraft** | Interface design and prototyping | [GitHub](https://github.com/bd-tecnologia/figcraft) | [Website](https://bdtecnolog.ia.br/apps/figcraft) |
| <img src="https://raw.githubusercontent.com/bd-tecnologia/wordcraft/main/assets/app-icon/hicolor/64x64/apps/com.bdtecnologia.wordcraft.png" alt="" width="32" height="32"> | **WordCraft** | Word processing and documents | [GitHub](https://github.com/bd-tecnologia/wordcraft) | [Website](https://bdtecnolog.ia.br/apps/wordcraft) |
| <img src="https://raw.githubusercontent.com/bd-tecnologia/soundcraft/main/assets/app-icon/hicolor/64x64/apps/com.bdtecnologia.soundcraft.png" alt="" width="32" height="32"> | **SoundCraft** | Audio recording and editing | [GitHub](https://github.com/bd-tecnologia/soundcraft) | [Website](https://bdtecnolog.ia.br/apps/soundcraft) |
| <img src="https://raw.githubusercontent.com/bd-tecnologia/deckcraft/main/assets/app-icon/hicolor/64x64/apps/com.bdtecnologia.deckcraft.png" alt="" width="32" height="32"> | **DeckCraft** | Presentations and slide decks | [GitHub](https://github.com/bd-tecnologia/deckcraft) | [Website](https://bdtecnolog.ia.br/apps/deckcraft) |
| <img src="https://raw.githubusercontent.com/bd-tecnologia/cadcraft/main/assets/app-icon/hicolor/64x64/apps/com.bdtecnologia.cadcraft.png" alt="" width="32" height="32"> | **CADCraft** | Computer-aided design | [GitHub](https://github.com/bd-tecnologia/cadcraft) | [Website](https://bdtecnolog.ia.br/apps/cadcraft) |
| <img src="https://raw.githubusercontent.com/bd-tecnologia/gridcraft/main/assets/app-icon/hicolor/64x64/apps/com.bdtecnologia.gridcraft.png" alt="" width="32" height="32"> | **GridCraft** | Spreadsheets and data | [GitHub](https://github.com/bd-tecnologia/gridcraft) | [Website](https://bdtecnolog.ia.br/apps/gridcraft) |

### Publishing an app

```sh
cargo xtask publish --app meuapp --version 1.2.0 --dir dist/1.2.0 --icon assets/icon.png
```

One command: it hashes the files, uploads them to the storage bucket and writes the app's entry
in the catalog. Every copy of the launcher shows the new version on its next check — there is no
launcher to rebuild and no release to cut.

The catalog's shape, the Firebase rules and a workflow that does the same from CI are in
[`docs/catalog.md`](docs/catalog.md).

And [**BD**](https://bdtecnolog.ia.br/) itself, our AI image and video studio for artists who want real control.

<br>

<p align="center">
  <a href="https://wa.me/5511963080423"><img alt="Talk to BD on WhatsApp" src="https://img.shields.io/badge/WhatsApp-25D366?style=for-the-badge&logo=whatsapp&logoColor=white" height="40"></a>
</p>

<h3 align="center">Come make things with us</h3>

<p align="center">
  Our WhatsApp is where artists of every kind hang out: people who paint, shoot, draw, cut film,
  set type, and people still figuring out what they like to make. Share what you're working on,
  ask for help, tell us what's broken, or tell us what you wish these tools could do.
  Whatever your medium and however long you've been at it, you're welcome here.
</p>

<p align="center">
  <a href="https://wa.me/5511963080423"><b>(11) 96308-0423</b></a> ·
  <a href="https://bdtecnolog.ia.br/">bdtecnolog.ia.br</a> ·
  <a href="https://bdtecnolog.ia.br/apps">The BD Apps</a> ·
  <a href="https://github.com/bd-tecnologia/bd-launcher">BD Launcher</a>
</p>

---

## Thanks

The community built launchers for the BD Apps before we did, and BD Launcher learned a
lot from them. Thank you to their authors:

| Project | Author | What we learned from it |
|---|---|---|
| [BD Suite Updater](https://github.com/Lyraedan/BD-Suit-Updater) | [@Lyraedan](https://github.com/Lyraedan) | Noticing apps installed or removed outside the updater with a quiet background re-scan, detecting Arch and AUR installs through pacman, and a platform picker that lists the downloads for every OS |
| [craftcenter](https://github.com/avokadosauce/craftcenter) | [@avokadosauce](https://github.com/avokadosauce) | Finding the latest release without spending API quota (the `releases/latest` redirect plus `SHA256SUMS.txt`), and verifying every file |
| [CRAFTSPACE](https://github.com/emircesur/CRAFTSPACE) | [@emircesur](https://github.com/emircesur) | Adopting apps that were already installed, falling back gracefully when GitHub rate-limits, background update notifications |
| [GetCraft](https://github.com/mbirnbach/getcraft) | [@mbirnbach](https://github.com/mbirnbach) | Picking the right download for each OS and CPU, recognising Windows Installer and macOS installs, a native Rust/egui launcher |
| [CraftHub](https://github.com/CraftHubLauncher/CraftHub) | [CraftHubLauncher](https://github.com/CraftHubLauncher) | Strict checks on where downloads come from, clear rate-limit messages, never replacing an app while it runs |
| [bd-suite](https://github.com/erikerikerik/bd-suite) | [@erikerikerik](https://github.com/erikerikerik) | ETag-cached release checks and installs that recover cleanly from interruptions |
| [Craft Apps Launcher & Installer](https://github.com/ZifuM/Craft-apps-launcher-installer) | [@ZifuM](https://github.com/ZifuM) | Release-page fallbacks and starting AppImage apps with a clean environment |
| [BD Launcher](https://github.com/EricVogt93/bd-launcher) | [@EricVogt93](https://github.com/EricVogt93) | Desktop integration for AppImages and a careful self-update design |
| [LandingCraft](https://github.com/rallegretti/LandingCraft) | [@rallegretti](https://github.com/rallegretti) | Working with the Windows Installer directly and offering stable releases only |
| [Craft Apps Manager](https://github.com/CryptoKey98/craft-apps-manager) | [@CryptoKey98](https://github.com/CryptoKey98) | Scheduled background update checks and per-app launch options |
| [BD Hub Launcher](https://github.com/tinyplayerss/bd-hub-launcher) | [@tinyplayerss](https://github.com/tinyplayerss) | A friendly, one-window hub for the whole suite |
| [CraftHub](https://github.com/MegaTheLEGEND/CraftHub) | [@MegaTheLEGEND](https://github.com/MegaTheLEGEND) | Serving the apps' web builds from a self-hosted launcher |

## License and credits

BD Launcher is dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
Copyright (c) 2026 BD Team and the BD Launcher contributors. Required notices are in [NOTICE](NOTICE).

Bundled fonts, icons, images and other assets keep their own open licenses; each one is listed
with its author, source and license in [ATTRIBUTION.md](ATTRIBUTION.md).

The app icons of the Craft apps (`assets/app-icons/`) are each app's own icon, copied unmodified
from its repository and licensed like that app (MIT OR Apache-2.0).

**The BD name, logo and mark are not licensed under MIT or Apache-2.0.** They (everything
in [`docs/brand/`](docs/brand/), plus the app icon in [`assets/app-icon/`](assets/app-icon/), the
logo in the launcher's header and the "A" tray icon)
are trademarks of the BD Team and are not covered by this license. They may be used only unmodified, and only as
part of this repository and BD Launcher, under [`docs/brand/LICENSE-brand.txt`](docs/brand/LICENSE-brand.txt).
Forks and modified versions must remove them.

<p align="center">
  <a href="https://bdtecnolog.ia.br/"><img alt="BD" src="docs/brand/bd-mark.svg" width="96"></a><br>
  <sub>Made by the <a href="https://bdtecnolog.ia.br/">BD</a> team and community.</sub>
</p>
