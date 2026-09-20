# Install (no sudo from this tree)

Build a release binary, then copy it into a user prefix:

```bash
cargo build -p scs-ui --release
mkdir -p "$HOME/.local/bin" \
  "$HOME/.local/share/applications" \
  "$HOME/.local/share/icons/hicolor/scalable/apps"
cp target/release/shadow-creator-studio "$HOME/.local/bin/"
cp packaging/com.shadowfetch.creatorstudio.desktop "$HOME/.local/share/applications/"
cp packaging/com.shadowfetch.creatorstudio.svg \
  "$HOME/.local/share/icons/hicolor/scalable/apps/"
update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
```

Put `$HOME/.local/bin` on `PATH`. The desktop file expects `shadow-creator-studio` on `PATH`.

## Optional `.deb` recipe

This project does not run `sudo`. If you already have `dpkg-deb`:

```bash
./packaging/make-deb.sh
```

The script writes `dist/shadow-creator-studio_0.1.0_amd64.deb` using a user-owned
staging directory. Install the package yourself if you want it system-wide.
