# Building Shadow Creator Studio (Pop!_OS 24.04)

These commands are documentation. This repository **does not run `sudo`** and
does not change system configuration for you.

Expected host class: Pop!_OS 24.04 LTS (or Ubuntu 24.04), GNOME or COSMIC,
PipeWire, optional NVIDIA GPU with NVENC. Use the in-app Diagnostics page for
live versions on *your* machine — do not commit those reports.

## 1. Core toolchain

```bash
rustc --version    # 1.80 or newer
cargo --version
pkg-config --version
gcc --version
```

If those fail on a clean Pop!_OS:

```bash
sudo apt update
sudo apt install build-essential pkg-config rustc cargo
```

Optional quality tools:

```bash
sudo apt install rustfmt rust-clippy
```

`rustup` is **not** required. A rustup install also works if you prefer it;
do not mix it blindly with the apt `rustc` on `PATH`.

## 2. Packages required to compile the GUI (`scs-ui`)

Without these, `cargo build -p scs-ui` fails at `pkg-config` (`gtk4` /
`libadwaita-1`). Runtime GTK libraries can be present while the **dev** packages
are missing.

```bash
sudo apt install \
  libgtk-4-dev \
  libadwaita-1-dev \
  libpango1.0-dev \
  libcairo2-dev \
  libgraphene-1.0-dev \
  libgdk-pixbuf-2.0-dev
```

`libglib2.0-dev` is often already installed. Apt skips packages you already have.

Typical runtimes that go with those headers on Pop!_OS 24.04:

- `libgtk-4-1` 4.14.x
- `libadwaita-1-0` 1.5.x

## 3. Packages required for later milestones (not needed to test M1 libraries)

```bash
# M2+ PipeWire headers
sudo apt install libpipewire-0.3-dev

# Keyring backend (when stream keys are stored)
sudo apt install libsecret-1-dev

# Optional: EasyEffects
sudo apt install easyeffects

# Optional: OBS (primary record engine from M3)
sudo apt install obs-studio
```

GStreamer **dev** packages are only needed if a future milestone links
GStreamer directly:

```bash
sudo apt install libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev
```

## 4. Multimedia runtimes to verify on your machine

```bash
ffmpeg -version
ffmpeg -hide_banner -encoders | grep -E 'nvenc|libx264'
pipewire --version
pw-cli info 0
obs --version
```

On NVIDIA systems you can also run `nvidia-smi` once by hand. The app dashboard
uses NVML instead of polling that command.

Look for FFmpeg 6.x and, when applicable, `h264_nvenc` / `hevc_nvenc` /
`av1_nvenc`.

## 5. Build

From the repository root:

```bash
# Foundation crates + tests (no GTK headers required)
cargo test --workspace --exclude scs-ui --lib

# GUI (requires section 2)
cargo build -p scs-ui
cargo run -p scs-ui
```

Release:

```bash
cargo build --workspace --exclude scs-ui --release
cargo build -p scs-ui --release
```

Outputs land in `target/debug/` or `target/release/`. The GUI binary name is
`shadow-creator-studio`.

## 6. XDG paths (created on first successful save)

| Kind | Path |
| --- | --- |
| Settings | `$XDG_CONFIG_HOME/com.shadowfetch.creatorstudio/settings.json` |
| Default recordings | `$XDG_VIDEOS_DIR/Shadow Creator Studio` |

The app never writes PipeWire, EasyEffects, or system microphone configuration.

## 7. Display

COSMIC and GNOME on Wayland are supported. Headless CI should run
`cargo test --workspace --exclude scs-ui --lib` only.

## 8. Minimum apt set for the M1 window

```bash
sudo apt install libgtk-4-dev libadwaita-1-dev
```

If `pkg-config` still cannot find a GTK dependency after that, install the
full GUI set from section 2.
