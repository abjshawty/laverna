# Laverna

A minimal desktop app for downloading videos with [yt-dlp](https://github.com/yt-dlp/yt-dlp).
Paste links, pick a quality (best by default), and they download one after the other.
Colors and shapes are customizable in-app or via `~/.config/laverna/theme.css`.

See [SPEC.md](SPEC.md) for the full design.

## Requirements

- `yt-dlp` and `ffmpeg` on your `PATH` (or set their paths in Settings)
- For YouTube, enabling **Settings → Network → Cookies from browser** often avoids
  "Sign in to confirm you're not a bot" errors

## Install

### Arch Linux

The PKGBUILD lives in its own repo, [gitlab.com/abjshawty/laverna](https://gitlab.com/abjshawty/laverna).
Clone it, then build and install with `makepkg` or `yay`:

```sh
git clone https://gitlab.com/abjshawty/laverna.git laverna-pkgbuild
cd laverna-pkgbuild

makepkg -si    # build and install, pulling in dependencies with pacman
# or
yay -Bi .      # same, via yay
```

To update later: `git pull`, then run the same command again.

`-s` installs the build dependencies (`bun`, `cargo`) and `-i` installs the package, which pulls
in `yt-dlp`, `ffmpeg` and `webkit2gtk-4.1`. The PKGBUILD downloads the tagged release from GitHub,
so this builds the version in `pkgver`, not your working tree. The same PKGBUILD is also in this
repo under `packaging/aur/`, so `cd packaging/aur && makepkg -si` works from a clone of this repo.

If your Rust toolchain comes from the rustup install script rather than pacman, pacman doesn't
know about `cargo`; install `bun` yourself and run `makepkg -si --nodeps` instead (yay resolves
dependencies itself, so use makepkg for this).

### AppImage and .deb

Download them from the [latest release](https://github.com/abjshawty/laverna/releases/latest)
(or build them yourself, see [Build](#build)). Each release includes a `SHA256SUMS` file; check
your download with `sha256sum -c SHA256SUMS --ignore-missing`.

- **AppImage**: `chmod +x Laverna_*.AppImage && ./Laverna_*.AppImage`
- **Debian/Ubuntu**: `sudo apt install ./Laverna_*_amd64.deb` (pulls in yt-dlp and ffmpeg as recommended packages)

## Develop

Needs Rust, [bun](https://bun.sh), and the [Tauri Linux prerequisites](https://v2.tauri.app/start/prerequisites/)
(`webkit2gtk-4.1`, `libsoup3`).

```sh
bun install
bun run tauri dev        # run with hot reload
bun run check            # type-check the frontend
cd src-tauri && cargo test               # unit tests
cd src-tauri && cargo test -- --ignored  # also run real yt-dlp (needs network)
```

## Build

```sh
bun run tauri build
```

Produces `src-tauri/target/release/bundle/appimage/Laverna_<version>_amd64.AppImage` and
`src-tauri/target/release/bundle/deb/Laverna_<version>_amd64.deb`.

The app icon is generated from `assets/icon.svg`:

```sh
rsvg-convert -w 1024 -h 1024 assets/icon.svg -o assets/icon.png
bun run tauri icon assets/icon.png
```

## Release

1. Bump the version in `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, and
   `pkgver` in `packaging/aur/PKGBUILD` (reset `pkgrel=1`).
2. Commit, tag, and push:

   ```sh
   git tag -a v<version> -m "Laverna <version>"
   git push origin main v<version>
   ```

3. Update the PKGBUILD checksum and metadata, then test the package:

   ```sh
   cd packaging/aur
   updpkgsums                          # fetches the new tag's tarball, pins its sha256
   makepkg --printsrcinfo > .SRCINFO
   makepkg -f                          # full build: prepare, build, check (tests), package
   makepkg --verifysource              # quick check that the checksum matches GitHub's tarball
   ```

   For a check that the declared dependencies are complete, build in a clean chroot with
   `pkgctl build` (from `devtools`).

4. Commit the updated `PKGBUILD` and `.SRCINFO` here, then mirror them to the PKGBUILD repo
   (`packaging/aur/` in this repo is the source of truth):

   ```sh
   cp packaging/aur/{PKGBUILD,.SRCINFO} ~/Projects/laverna/
   cd ~/Projects/laverna
   git commit -am "Update to <version>"
   git push
   ```

   Publishing to the AUR later works the same way, with `ssh://aur@aur.archlinux.org/<pkgname>.git`
   as the repo. The AUR name `laverna` is already taken by an unrelated note-taking app that also
   installs `/usr/bin/laverna`, so that will need a different `pkgname` (and binary name).

5. Build the AppImage and .deb, then publish a GitHub release with them and their checksums:

   ```sh
   bun run tauri build
   cd src-tauri/target/release/bundle
   sha256sum appimage/Laverna_<version>_amd64.AppImage deb/Laverna_<version>_amd64.deb \
     | sed 's|appimage/||; s|deb/||' > SHA256SUMS
   gh release create v<version> --verify-tag --title "Laverna <version>" --notes-file <notes.md> \
     appimage/Laverna_<version>_amd64.AppImage deb/Laverna_<version>_amd64.deb SHA256SUMS
   ```

## Theming

The theme is a set of CSS variables, defined in `src/lib/theme.css`. Override any of them in
`~/.config/laverna/theme.css`:

```css
:root {
  --accent: #e0457b;
  --radius: 4px;
}
```

The file reloads live. The in-app theme editor writes to its own marked section at the end of
that file and leaves the rest alone.

Overrides in a plain `:root { … }` rule apply in both light and dark mode, and always win over the
presets. The mode itself (follow system, dark, or light) is set in **Settings → Appearance**.

### Omarchy

On [Omarchy](https://omarchy.org), the theme editor has an **Install Omarchy theme sync** button.
It installs a hook at `~/.config/omarchy/hooks/theme-set.d/sync-laverna-theme` and runs it once,
so Laverna takes the current Omarchy theme's colors now and follows every later
`omarchy theme set`. The hook writes its colors to a marked block at the top of `theme.css`;
your own rules and the editor's section come after it, so they still win. The button disappears
once the hook is installed; to uninstall, delete that file and the `omarchy-sync` block.

### Variables

| Variable | What it styles | Dark | Light |
| --- | --- | --- | --- |
| `--bg` | Window background | `#16161a` | `#f7f7f9` |
| `--surface` | Inputs, buttons, dropdowns, the settings panel and log viewer, thumbnail placeholders | `#1f1f25` | `#ffffff` |
| `--surface-hover` | Buttons and queue rows on hover | `#27272f` | `#f0f0f4` |
| `--border` | Borders of inputs, buttons and panels; dividers around the add bar and footer | `#2e2e37` | `#dedee5` |
| `--fg` | Main text | `#e8e8ec` | `#1b1b22` |
| `--muted` | Secondary text: statuses, labels, hints, the footer, dropdown arrows | `#8d8d99` | `#6b6b78` |
| `--accent` | Primary button (Add), progress bar fill, focus outline, drop-zone outline | `#7c9cff` | `#3b5bdb` |
| `--accent-fg` | Text on the primary button | `#0d1020` | `#ffffff` |
| `--progress-track` | Unfilled part of the progress bar | `#2a2a33` | `#e6e6ec` |
| `--success` | "Done" status | `#5fd38d` | `#2b9a5a` |
| `--warning` | "yt-dlp not found" banner (tinted), "update yt-dlp" hint, MP4 quality warning, missing-tool notes | `#f2c55c` | `#b8860b` |
| `--danger` | Error messages: failed downloads, settings and theme editor errors | `#ff6b6b` | `#d9363e` |
| `--radius` | Corner radius of controls, queue rows and the log viewer (thumbnails use half) | `8px` | same |
| `--font` | Font family for the whole app | `system-ui, -apple-system, "Segoe UI", sans-serif` | same |
| `--font-size` | Base font size; other text sizes scale from it | `14px` | same |

## License

MIT, see [LICENSE](LICENSE).
