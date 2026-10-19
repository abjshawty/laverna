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

Grab a build from `src-tauri/target/release/bundle/` (see below):

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
