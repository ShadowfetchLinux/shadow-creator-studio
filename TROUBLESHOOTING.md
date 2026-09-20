# Troubleshooting (Milestone 1)

Practical notes for Pop!_OS 24.04 / COSMIC or GNOME / NVIDIA NVENC setups.
Later milestones should append here instead of inventing a second FAQ.

## `Package 'gtk4' was not found` / `libadwaita-1` not found

The GTK4 and libadwaita **runtimes** can be installed while the **dev** packages
(and `.pc` files) are missing.

```bash
pkg-config --modversion gtk4
pkg-config --modversion libadwaita-1
```

If those fail:

```bash
sudo apt install libgtk-4-dev libadwaita-1-dev
```

Foundation tests do not need those packages:

```bash
cargo test --workspace --exclude scs-ui --lib
```

## GUI will not start (no window)

1. Confirm `DISPLAY` or `WAYLAND_DISPLAY` is set.
2. Confirm `scs-ui` actually linked (`ls target/debug/shadow-creator-studio`).
3. On COSMIC, GTK4 apps should open without extra env vars. If decorations look
   wrong, that is cosmetic — do not force `GDK_BACKEND=x11` unless Wayland
   actually fails.
4. This project does not install or patch desktop portals.

## Settings file looks empty or old

Path: `$XDG_CONFIG_HOME/com.shadowfetch.creatorstudio/settings.json`

- The file is versioned (`version: 1`). Unknown future versions refuse to
  silently downgrade; see `scs-core` migration.
- **Restore Recommended Settings** in the Settings page rewrites defaults and
  saves. It does not touch PipeWire or EasyEffects.
- Delete the JSON to get a clean first-run wizard on the next launch (your
  recordings folder is not deleted).

## Diagnostics say Unavailable

That is intentional. M1 only reports probes that exist on the machine that
**runs** the app. Those results are runtime-only — do not commit them.

| Row | How it is gathered |
| --- | --- |
| OS | `/etc/os-release` |
| Rust | `rustc --version` (once) |
| FFmpeg | `ffmpeg -version` (once) |
| PipeWire | `pipewire --version` and the runtime socket |
| GPU | NVML first; never a dashboard `nvidia-smi` loop |
| OBS | binary on `PATH`; websocket **not** connected |
| EasyEffects | `PATH` only |

If a detector is not implemented, the UI shows **Unavailable** — not a green
checkmark.

## Copy Diagnostic Report included something sensitive

It should not. Redaction strips common secret keys and bearer-like tokens.
If you still see a secret:

1. Treat the clipboard as compromised and rotate that credential.
2. File a bug with a **redacted** example (never paste the real key).
3. Do not put stream keys into Settings in M1 — the field is disabled.

## NVIDIA / NVENC rows look idle

Idle is correct in M1. “NVENC advertised” means FFmpeg listed `h264_nvenc`
(or HEVC/AV1). It is **not** a test encode. A full encode path is M3.

Dashboard GPU numbers come from NVML. If NVML init fails, values are `—` and
labeled unavailable. Do not add a `nvidia-smi` poll to the 2-second timer.

## PipeWire vs PulseAudio

`pactl info` showing `PulseAudio (on PipeWire …)` is normal. The server is
PipeWire. M2 will prefer native PipeWire; Pulse remains a fallback.

This app must not rewrite default sources/sinks in M1.

## OBS is installed but the app says recording is unavailable

Correct for M1. Recording starts in M3. Prefer a single OBS install and enable
obs-websocket when that milestone lands.

## EasyEffects is missing

Expected. Audio processing in-app is M4. Installing EasyEffects later is
optional and user-controlled.

## Disk / time remaining looks pessimistic

Estimates use the selected quality preset’s **assumed** bitrate plus a 1 GiB
reserve. They are planning numbers, not a promise. Actual NVENC bitrate is
not measured until recording exists.

## `cargo test` fails in rustdoc / libLLVM

Some Pop!_OS Rust packages ship `rustc` without a matching `rustdoc` LLVM
library. Library unit tests still pass:

```bash
cargo test --workspace --exclude scs-ui --lib
```

## Build warnings from gtk-rs

After installing `-dev` packages, a first `cargo build -p scs-ui` downloads
a large gtk-rs graph. That is expected. Warnings inside generated bindings
are upstream; fix only warnings in this repository’s sources.

## Never do these

- Do not run the app as root.
- Do not paste stream keys into bug reports or diagnostic files you will share.
- Do not delete an MKV because an MP4 appeared — verification is M6.
- Do not “fix” mic quality by writing WirePlumber/EasyEffects config from this app.
- Do not commit Diagnostics output, recordings, or `.env` files.
