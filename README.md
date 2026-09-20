# Shadow Creator Studio

A Linux desktop app for recording professional YouTube videos — screen, webcam,
tutorials, and voice — with a simple creator-focused interface. The long-term
engine is OBS Studio (via WebSocket) with FFmpeg + NVIDIA NVENC as the
tooling and fallback path.

**Milestone 2** lists cameras and PipeWire audio, shows a live camera preview
and real meters, and still does **not** record or go live.

## Screenshots

> Add PNG captures under `docs/screenshots/` after you build the GTK shell
> (`libgtk-4-dev` and `libadwaita-1-dev`).

| Screen | Placeholder |
| --- | --- |
| Record | ![Record page](docs/screenshots/record.png) |
| Settings | ![Settings](docs/screenshots/settings.png) |
| Diagnostics | ![Diagnostics](docs/screenshots/diagnostics.png) |
| First-run wizard | ![Wizard](docs/screenshots/wizard.png) |

## What works in Milestone 2

- Everything from Milestone 1 (shell, settings, wizard, diagnostics, dashboard)
- Camera selector with name, resolution, and pixel format (not raw `/dev/videoN` as the only label)
- PipeWire microphone and desktop-monitor lists from `pw-dump`
- Live camera preview in the large preview area, with an optional mirror flip
- Real-time mic and desktop meters (peak + average) plus a clipping warning
- Multi-monitor display tiles from the session; window and region capture stay labeled unavailable
- Last camera, mic, desktop source, display, and recording mode persist in settings
- Background discovery — the UI does not freeze while probing devices

## What is intentionally unavailable

| Control | Label |
| --- | --- |
| START RECORDING | Available in a later milestone (M3) |
| GO LIVE | Available in a later milestone (M8) |
| Window / region capture | Structured, labeled unavailable |
| Live desktop frames | Selected-display placeholder (portal capture is later) |
| Library / Teleprompter | Empty states for M6 / M7 |

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

This repository is the source tree. There is no packaged `.deb` yet.

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
- Separate audio tracks are the plan (mic vs desktop). Not captured yet.
- Preferred future encoder: **NVIDIA NVENC** (`h264_nvenc`, plus HEVC/AV1 when
  FFmpeg lists them).
- Primary future record engine: **OBS via obs-websocket**. FFmpeg is remux /
  probe / fallback. OBS is not required to launch the studio.

## Audio

- PipeWire is the native path. `pactl` showing “PulseAudio (on PipeWire)” is normal.
- Planned chain (M4): Mic → high-pass → noise suppression → gate → EQ →
  compressor → limiter. Default **Natural** (light), plus **Raw**.
- EasyEffects is compatible in concept and optional. Shadow Creator Studio will
  not silently change system mic routing.

## NVIDIA

On a machine with an NVIDIA driver and NVENC-capable FFmpeg, the dashboard reads
GPU load, VRAM, and temperature through NVML when initialization succeeds. It
does not spawn `nvidia-smi` every tick. Idle encoder rows are expected until M3.

## Known limitations (M2)

- START RECORDING and GO LIVE stay disabled. Nothing is written to disk.
- Screen/Presentation modes show the selected display name, not live desktop frames.
  Portal / PipeWire screen capture is later.
- Window and region capture are structured types only.
- Camera preview uses FFmpeg V4L2 → RGB24. If the device is busy or FFmpeg fails,
  the preview shows a human-readable error instead of a fake frame.
- Meters use `pw-record` against a PipeWire node. The app never writes WirePlumber,
  EasyEffects, or default source/sink configuration.
- Encoder “ready” means FFmpeg advertised the encoder, not that a test encode ran.
- OBS WebSocket is detected as a binary/plugin at most; no login, no scenes.

## License

[MIT](LICENSE).
