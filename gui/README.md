# capcap desktop app

Drag-and-drop Tauri wrapper around the shared `capcap` pipeline (`../src/pipeline.rs`).
Plain HTML/CSS/vanilla JS frontend (`src/`), Rust backend in `src-tauri/` that depends on
the root `capcap` crate via a path dependency — same Whisper model (embedded at compile
time by `../build.rs`) and mux logic as the CLI, zero duplication.

## Develop

```bash
npm install
npx tauri dev
```

## Build

```bash
npm install
npm run tauri build
```

Produces `src-tauri/target/release/bundle/macos/Capcap.app` and a `.dmg`. Unsigned (no
Apple Developer cert yet) — see the root README for how to open it past Gatekeeper.

## Offline ffmpeg for release builds

For local `dev`/`build`, `ffmpeg-sidecar` auto-downloads ffmpeg into its own cache on
first use, same as the CLI does in development. For the packaged release `.app` to be
offline-first, `.github/workflows/release.yml`'s `build-gui` job downloads a static
ffmpeg binary and copies it into `Capcap.app/Contents/MacOS/ffmpeg` (next to the
`capcap-gui` executable) after `tauri build`, before it's zipped into the `.dmg` — that's
the directory `ffmpeg-sidecar` checks first, matching how the CLI's release job bundles
ffmpeg next to the `capcap` binary.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
