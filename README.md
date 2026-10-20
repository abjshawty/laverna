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

The package is `laverna-dl` (the AUR name `laverna` belongs to an unrelated note-taking app).
The binary is `laverna-dl`; the app still shows up as Laverna.

Build and install it with `makepkg` from the PKGBUILD in this repo:

```sh
git clone https://github.com/abjshawty/laverna
cd laverna/packaging/aur
makepkg -si
```

`-s` installs the build dependencies (`bun`, `cargo`) and `-i` installs the package, which pulls
in `yt-dlp`, `ffmpeg` and `webkit2gtk-4.1`. The PKGBUILD downloads the tagged release from GitHub,
so this builds the version in `pkgver`, not your working tree.

If your Rust toolchain comes from the rustup install script rather than pacman, pacman doesn't
know about `cargo`; install `bun` yourself and run `makepkg -si --nodeps` instead.

### AppImage and .deb

Grab a build from `src-tauri/target/release/bundle/` (see [Build](#build)):

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

4. Commit the updated `PKGBUILD` and `.SRCINFO`, then publish to the AUR (needs an AUR account
   with your SSH key):

   ```sh
   git clone ssh://aur@aur.archlinux.org/laverna-dl.git ~/aur/laverna-dl   # first time only
   cp packaging/aur/{PKGBUILD,.SRCINFO} ~/aur/laverna-dl/
   cd ~/aur/laverna-dl
   git commit -am "Update to <version>"
   git push
   ```

   Only `PKGBUILD` and `.SRCINFO` go to the AUR; the first push needs `git add` for both files.

## Theming

The theme is a set of CSS variables (`--bg`, `--fg`, `--accent`, `--radius`, `--font`, …; see
`src/lib/theme.css`). Override any of them in `~/.config/laverna/theme.css`:

```css
:root {
  --accent: #e0457b;
  --radius: 4px;
}
```

The file reloads live. The in-app theme editor writes to its own marked section at the end of
that file and leaves the rest alone.

## License

MIT, see [LICENSE](LICENSE).
