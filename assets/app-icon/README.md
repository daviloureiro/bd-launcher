# BD Launcher app icon

The app icon is the **BD mark** (the letterform the BD team uses, from
[`docs/brand/bd-mark-icon.svg`](../../docs/brand/bd-mark-icon.svg)): the mark in white on the
brand's blue plate, centred on Apple's icon grid. Every file in this directory is rendered from
it by `packaging/icons.sh`.

**These files are a BD trademark. They are not open source and not covered by BD
Launcher's MIT or Apache-2.0 license.** Their terms are in
[`docs/brand/LICENSE-brand.txt`](../../docs/brand/LICENSE-brand.txt): use them only as part of
BD Launcher, never redraw, recolor or crop them, and remove them from forks and modified
versions.

## Files

- `bd-launcher.svg`: a copy of `docs/brand/bd-mark-icon.svg`; also the hicolor scalable icon.
- `bd-launcher-1024.png`: 1024 px render.
- `bd-launcher.icns`: macOS bundle icon (`CFBundleIconFile`).
- `bd-launcher.ico`: Windows icon, 16…256 px, embedded in the `.exe` by
  `apps/bd-launcher/build.rs` and used by the MSI.
- `hicolor/<size>/apps/com.bdtecnologia.bd-launcher.png` (16…512) and
  `hicolor/scalable/apps/com.bdtecnologia.bd-launcher.svg`: Linux icon theme.
- The tray / menu-bar icon is the same mark, rendered into `docs/brand/bd-mark-black-44.png`
  (macOS template, 22 pt @2x) and `docs/brand/bd-mark-64.png` (blue; Windows, Linux, FreeBSD).

## Regenerate

Run `packaging/icons.sh` on any OS. It needs `resvg`; the `.ico` and `.icns` are packed by
`cargo xtask ico`. Never edit these files by hand: change the source only if the BD Team
supplies a new mark in `docs/brand/`.
