# Shadow Creator Studio

A Linux desktop app for recording professional YouTube videos — screen, webcam,
tutorials, and voice — with a simple creator-focused interface. The long-term
engine is OBS Studio (via WebSocket) with FFmpeg + NVIDIA NVENC as the
tooling and fallback path.

**Milestone 1** is the application shell: a polished dark UI, real settings,
honest unavailable states, and tested foundation libraries. It does **not**
record or go live yet.

## Screenshots

> Add PNG captures under `docs/screenshots/` after you build the GTK shell
> (`libgtk-4-dev` and `libadwaita-1-dev`).

| Screen | Placeholder |
| --- | --- |
| Record | ![Record page](docs/screenshots/record.png) |
| Settings | ![Settings](docs/screenshots/settings.png) |
| Diagnostics | ![Diagnostics](docs/screenshots/diagnostics.png) |
| First-run wizard | ![Wizard](docs/screenshots/wizard.png) |

## What works in Milestone 1

- Dark, uncluttered Record / Library / Teleprompter / Settings / Diagnostics navigation
- Recording mode tiles (Camera, Screen, Presentation, Voice, Creator, Custom) — last
  selection is saved
- Settings groups with **Restore Recommended Settings**
- First-run wizard (folder + quality persist; device tests labeled unavailable)
- Diagnostics assembled **at runtime** on the machine that runs the app (OS, Rust,
  FFmpeg, PipeWire, GPU via NVML). Do not commit those reports.
- **Copy Diagnostic Report** with secret redaction
- System dashboard: live CPU, RAM, disk, and GPU/VRAM/temp when NVML works;
  em-dash + “Unavailable” otherwise
- Timestamped filenames, recording metadata, disk estimates (libraries + tests)

## What is intentionally unavailable

| Control | Label |
| --- | --- |
| START RECORDING | Available in a later milestone (M3) |
| GO LIVE | Available in a later milestone (M8) |
| Camera preview | Placeholder canvas — not a live camera |
| Mic / desktop meters | Idle until Milestone 2 |
| Library / Teleprompter | Empty states for M6 / M7 |
| Wizard device tests | Unavailable — never a fake pass |

## Dependencies

**Typical Pop!_OS 24.04 creator desktop**

- Rust 1.80+ (`rustc`, `cargo`)
- GTK4 4.14+ and libadwaita 1.5+ *runtimes* to run the window
- `libgtk-4-dev` and `libadwaita-1-dev` to *compile* `scs-ui`
- PipeWire (Pulse compatibility is fine)
- FFmpeg 6.x; NVIDIA NVENC encoders when an NVIDIA GPU is present
- Optional: OBS Studio with obs-websocket (M3+), EasyEffects (M4 concept)

Foundation crates and tests compile without GTK headers.

## Install

This repository is the source tree. There is no packaged `.deb` in M1.

```bash
git clone https://github.com/ShadowfetchLinux/shadow-creator-studio.git
cd shadow-creator-studio
```

Install GTK development packages if you want the window (requires admin; this
project never runs `sudo` for you):

```bash
sudo apt install build-essential pkg-config libgtk-4-dev libadwaita-1-dev
```

Full package list: [BUILDING.md](BUILDING.md).

## Run

Libraries and tests (works without GTK headers):

```bash
cargo test --workspace --exclude scs-ui --lib
```

(`cargo test` also works; system `rustdoc` on some Pop!_OS images is missing
LLVM and will fail **doctests** only. `--lib` skips that.)

GUI (needs GTK4/libadwaita **dev** packages and a display):

```bash
cargo run -p scs-ui
```

Config is written to:

```text
$XDG_CONFIG_HOME/com.shadowfetch.creatorstudio/settings.json
```

Default recording folder (created when you choose it / finish the wizard):

```text
$XDG_VIDEOS_DIR/Shadow Creator Studio
```

## Recording notes

- Target container: **MKV** (crash-safe). Optional remux to MP4 without re-encode
  starts in M6. The MKV is never deleted until the MP4 verifies.
- Separate audio tracks are the plan (mic vs desktop). Not captured in M1.
- Preferred future encoder: **NVIDIA NVENC** (`h264_nvenc`, plus HEVC/AV1 when
  FFmpeg lists them).
- Primary future record engine: **OBS via obs-websocket**. FFmpeg is remux /
  probe / fallback. OBS is not required to launch M1.

## Audio

- PipeWire is the native path. `pactl` showing “PulseAudio (on PipeWire)” is normal.
- Planned chain (M4): Mic → high-pass → noise suppression → gate → EQ →
  compressor → limiter. Default **Natural** (light), plus **Raw**.
- EasyEffects is compatible in concept and optional. Shadow Creator Studio will
  not silently change system mic routing.

## NVIDIA

On a machine with an NVIDIA driver and NVENC-capable FFmpeg, the dashboard reads
GPU load, VRAM, and temperature through NVML when initialization succeeds. It
does not spawn `nvidia-smi` every tick. Idle encoder rows in M1 are expected.

## Known limitations (M1)

- The GUI crate does not compile until `libgtk-4-dev` and `libadwaita-1-dev`
  are installed.
- No recording, streaming, live preview, or meters.
- Encoder “ready” means FFmpeg advertised the encoder, not that a test encode ran.
- OBS WebSocket is detected as a binary/plugin at most; no login, no scenes.
- YouTube Live, captions, and library playback are future milestones.

## License

[MIT](LICENSE).
