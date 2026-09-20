# Shadow Creator Studio — Architecture

Professional YouTube recording for Linux. OBS-level reliability with a creator-focused
interface: screen, webcam, tutorials, voice, and later YouTube Live.

This document is the source of truth for framework choice, process boundaries, and the
milestone plan. Implementation must stay honest: if a feature is not wired, the UI
disables it and labels the milestone. Nothing pretends to record, stream, or pass a
device test.

## Expected environment

Target class: **Pop!_OS 24.04** (Ubuntu noble) on **GNOME or COSMIC**, Wayland or X11,
**PipeWire**, and an **NVIDIA GPU with NVENC**. A typical creator desktop is enough;
exact CPU, RAM, GPU SKU, driver build, disk layout, and host names are **not**
recorded in this repository.

| Component | Expected / required | Notes |
| --- | --- | --- |
| OS | Pop!_OS 24.04 LTS or similar Ubuntu 24.04 | Other Debian-family desktops may work |
| Desktop | GNOME or COSMIC | GTK4 + libadwaita |
| GPU | NVIDIA GPU with NVENC | Optional; `libx264` is the CPU fallback |
| NVML | `libnvidia-ml.so` when an NVIDIA driver is installed | Dashboard GPU rows use NVML, not a `nvidia-smi` loop |
| Rust | rustc/cargo **1.80+** | `rustfmt` / `clippy` optional |
| GTK4 runtime | GTK 4.14+ | Needed to *run* the window |
| GTK4 **dev** | `libgtk-4-dev` | Needed to *compile* `scs-ui` |
| libadwaita runtime | 1.5+ | Dark shell |
| libadwaita **dev** | `libadwaita-1-dev` | Needed to compile `scs-ui` |
| PipeWire | 1.x with Pulse compatibility | Native capture starts in M2 |
| PipeWire **dev** | `libpipewire-0.3-dev` | Not required; M2 uses `pw-dump` / `pw-record` |
| FFmpeg | 6.x with `h264_nvenc` when NVIDIA is present | Also used for remux/probe |
| OBS Studio | Optional | Planned primary record engine via obs-websocket (later). M3 uses FFmpeg |
| EasyEffects | Optional | Compatible in concept; never auto-configured |
| libsecret | Runtime later for stream keys | Dev package when the keyring backend lands |

**Live host facts belong in Diagnostics, not in git.** The in-app Diagnostics page
(and **Copy Diagnostic Report**) probes the local machine at runtime: OS, rustc,
FFmpeg, PipeWire, NVML/GPU, OBS on `PATH`, encoder advertisements. Reports are
redacted and must not be committed. Do not paste them into issues without a
second pass.

**Implication for M1:** foundation crates and tests compile with a normal Rust
toolchain. `scs-ui` needs the GTK4 / libadwaita development packages listed in
[BUILDING.md](BUILDING.md). This repository never runs `sudo` for you.

## Framework choice

**Rust + GTK4 + libadwaita (gtk-rs).** Workspace binary crate: `scs-ui`.

Reasons this is the default, and why inspection did not overturn it:

1. **Native Linux.** No Electron, no bundled Chromium, no extra GPU compositor.
2. **COSMIC and GNOME.** libadwaita is the native widget set on current Pop!_OS.
   COSMIC hosts GTK4 applications; GNOME is a first-class target.
3. **Rust core.** Settings, filenames, diagnostics, FFmpeg argument construction, and
   later PipeWire/OBS clients stay in Rust. The UI is a thin presentation layer.
4. **Mature widgets.** Preferences, status pages, split navigation, about dialogs, and
   dark style management are already in Adwaita.
5. **Later multimedia.** GStreamer and PipeWire have mature C APIs that gtk-rs / glib
   can host on the same main loop. M1 does not take that dependency.

**Rejected for the shell (not for engines):**

| Option | Why not as the app shell |
| --- | --- |
| Electron / Tauri + web UI | Heavier, less native on COSMIC/GNOME, fights the “appliance” look |
| Iced / Slint | Fine Rust GUIs, weaker platform integration (portals, shortcuts, Adwaita) |
| Qt | Extra toolkit on a GTK/COSMIC desktop |
| Pure FFmpeg CLI wrapper | No path to a polished creator UI |

Missing `-dev` packages are an install step, not an architecture failure. Pure-Rust
crates (`scs-core`, `scs-system`, media stubs) compile and test without GTK.

## Dual-backend recording engines

M1 does **not** record. The contract for later milestones:

```
                    ┌─────────────────────────────────────┐
                    │           scs-ui (GTK4)             │
                    │   state, honesty labels, dashboard  │
                    └──────────────┬──────────────────────┘
                                   │
                    ┌──────────────▼──────────────────────┐
                    │         application services        │
                    │  settings · library · diagnostics   │
                    └──────────────┬──────────────────────┘
                                   │
              ┌────────────────────┼────────────────────┐
              ▼                    ▼                    ▼
     ┌────────────────┐   ┌────────────────┐   ┌────────────────┐
     │ scs-obs        │   │ scs-ffmpeg     │   │ scs-pipewire   │
     │ OBS WebSocket  │   │ remux / probe  │   │ devices +      │
     │ PRIMARY record │   │ FALLBACK rec.  │   │ meters (M2+)   │
     └────────────────┘   └────────────────┘   └────────────────┘
              │                    │
              ▼                    ▼
        OBS Studio 32        FFmpeg 6.1 + NVENC
        + obs-websocket      structured argv only
```

- **Primary recording engine (M3+):** OBS Studio via obs-websocket when OBS is
  installed and the websocket is reachable. OBS already owns PipeWire capture,
  NVENC, scene composition, crash-safe MKV, and long-session behavior.
- **Fallback / tooling engine:** FFmpeg for remux, export, probe, and simple
  recording when OBS is absent. On NVIDIA systems, FFmpeg commonly exposes
  `h264_nvenc` / `hevc_nvenc` / `av1_nvenc`.
- **OBS is not required for M1.** Detection is best-effort and never faked.

Process rules:

- No root.
- Structured process arguments only. Never interpolate user strings into a shell.
- Never delete an MKV until a remuxed MP4 has been probed and verified.
- Default container: **MKV** (crash-safe). Optional remux to MP4 without re-encode.

## Audio plan

Implementation starts in **M4**. Types live in `scs-audio` now.

```
Mic → HPF → noise suppression (RNNoise / EasyEffects-compatible) → gate → EQ → compressor → limiter
```

- Default preset: **Natural** (light). Not aggressive.
- **Raw** remains available (all stages off).
- Desktop audio is a separate track. Mic and desktop stay separable for YouTube edits.
- This app must **never** silently rewrite system mic configuration (EasyEffects,
  WirePlumber, Pulse defaults). User-visible opt-in only, later.

PulseAudio is a compatibility fallback (`pactl` talking to PipeWire is normal).
Native PipeWire is the planned capture path.

## NVIDIA / NVENC plan

- Prefer NVENC (`h264_nvenc` default; HEVC/AV1 as explicit settings).
- Capability parsing lives in `scs-encoder` (FFmpeg encoder list + typed flags).
- Live GPU/VRAM/temp on the dashboard uses **NVML** (`libnvidia-ml.so`) — not a
  tight `nvidia-smi` loop. `nvidia-smi` may be used once for a diagnostic report
  if NVML init fails.
- Software `libx264` is the documented CPU fallback.

## Credentials

- Stream keys, websocket passwords, and OAuth tokens are **secrets**.
- Planned store: **libsecret / Secret Service** (when the runtime is installed).
- M1 settings may hold a *reference slot* only. The Streaming page does not accept
  a live key.
- Diagnostic reports run through `scs-core` redaction. Never log secrets.

## Local-only core

The recording core is local. Whisper, captions, highlights, and YouTube upload/live
are extension points. They do not run in M1 and must not phone home.

## Module map

Cargo workspace. Small crates, no giant sources.

| Crate | Role | Current reality |
| --- | --- | --- |
| `scs-core` | Settings, paths, filenames, markers, secrets, extension registry, local jobs | **M8 registry + local jobs tested** |
| `scs-system` | Cheap host probes: `/proc`, NVML, `statvfs`, hwmon | **Implemented** (live where cheap) |
| `scs-audio` | Devices, meters, track layout, FFmpeg filter graph, calibration | **M4 implemented + tested** |
| `scs-video` | Resolution, FPS, color, format types | Types only |
| `scs-capture` | V4L2 camera listing, display models, inventory | **Cameras + inventory**; window/region unavailable |
| `scs-encoder` | Encoder capability parsing | Parser + types; detection optional |
| `scs-pipewire` | `pw-dump` parse, mic vs desktop split, `pw-record` argv | **Listing + error mapping**; no libpipewire link |
| `scs-ffmpeg` | Typed argv builder, remux/record plans, camera preview argv | Record plans include filter_complex pad graphs |
| `scs-obs` | WebSocket client config + install probe | Probe only; no session |
| `scs-library` | Folder index, sidecar metadata, delete confirm | **M5 implemented + tested** |
| `scs-teleprompter` | Script, font/speed bounds, scroll math | **M6 implemented + tested** |
| `scs-diagnostics` | Redacted report assembly | **Implemented** |
| `scs-ui` | GTK4 + libadwaita shell + live preview/meters | **Implemented**; needs `-dev` packages to compile |

Application id: `com.shadowfetch.creatorstudio`  
Config: `$XDG_CONFIG_HOME/com.shadowfetch.creatorstudio/settings.json`  
Default recordings: `$XDG_VIDEOS_DIR/Shadow Creator Studio` (usually `~/Videos/...`)

## UI honesty rule

Disabled + labeled. Examples already in the shell:

- START RECORDING — Camera / Voice / Creator (FFmpeg MKV). Screen / Presentation stay unavailable
- GO LIVE — later milestone (M8)
- Window / region capture — labeled unavailable
- Live desktop frames — selected-display placeholder until portal capture
- Library / Teleprompter — empty states
- Wizard — lists real devices when the background scan finishes; no fake pass

## Milestone plan

| ID | Name | Includes |
| --- | --- | --- |
| **M1** | Application shell | This milestone. Window, navigation, Record page chrome, mode tiles (persisted), Settings structure + restore defaults, Diagnostics (real probes + copy redacted report), first-run wizard shell, system dashboard (cheap live metrics), foundation libraries + tests. **Does not record.** |
| **M2** | Device discovery + live preview | Cameras (name/resolution/FPS), PipeWire mics, desktop monitors, live camera preview + mirror, peak/average meters + clip, persist last devices/mode. **Does not record.** |
| **M3** | Recording | FFmpeg local MKV for Camera / Voice / Creator, NVENC when listed, timer, stop confirm, optional copy remux (MKV kept). Screen grab and OBS WebSocket still later |
| **M4** | Audio processing | Separate tracks, mute/volume, FFmpeg chain, presets, calibration |
| **M5** | Library + FFmpeg tools | Folder index, sidecar, remux/export/trim/normalize/GIF/thumb |
| **M6** | Library | Index, markers, remux MKV→MP4 without re-encode, never delete MKV until verified |
| **M6** | Teleprompter + markers + hotkeys | In-app accelerators, chapter sidecar, no global grabs |
| **M7** | Streaming architecture | RTMP/RTMPS, secret-tool, tee record+stream, reconnect policy |

### What M1 includes vs later

**In M1**

- Architecture and build docs
- Modular Rust workspace
- Settings serialization + versioned migration stub
- Timestamped, non-overwriting filenames
- Recording / marker metadata types
- Disk-space and bitrate-duration math
- Diagnostic secret redaction
- FFmpeg command *builder* (typed, tested)
- Encoder capability *parser*
- Polished dark UI chrome
- Persist last recording mode, wizard completion, folder, quality
- Live CPU / RAM / disk / GPU (NVML) where implementation is cheap and correct

**In M2**

- Background camera / PipeWire discovery
- Live V4L2 camera preview (FFmpeg RGB24 pipe) with optional hflip
- Real `pw-record` meters (peak, average, clipping). Never fake a moving bar
- Display selector from GDK monitors; honest placeholder for screen modes
- Persist last camera, mic, desktop source, display, mode, and mirror flag

**In M3**

- FFmpeg record plans (structured argv) for Camera, Voice, Creator
- NVENC when FFmpeg lists it; libx264 fallback
- Live timer, encoder name, disk reserve / emergency stop
- Optional copy remux to MP4; MKV is never deleted
- Stop always asks for confirmation

**Explicitly later**

- Portal / live desktop frames and Screen/Presentation recording
- Audio processing
- OBS WebSocket session (still the planned *primary* engine)
- Library playback, teleprompter, Whisper, YouTube APIs
- Writing PipeWire / EasyEffects / system audio configuration

## Security notes

- No shell-string interpolation of paths, titles, or device names.
- Redact `password`, `token`, `secret`, `stream_key`, `authorization`, `api_key`, and
  common variants in diagnostic JSON/text.
- Do not put secrets in logs or `ARCHITECTURE.md` samples.
