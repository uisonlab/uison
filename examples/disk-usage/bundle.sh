#!/bin/sh
# Disk Usage with its icon, as a program people install.
#   examples/disk-usage/bundle.sh macos          → out/disk-usage/Disk Usage.app (.icns from assets/icon.png)
#   examples/disk-usage/bundle.sh linux [prefix] → the program, a .desktop file and the icon in prefix (~/.local)
# Windows needs nothing: build.rs puts the icon into disk-usage.exe. BIN=path — an already built program.
set -eu

here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/Cargo.toml" | head -1)"
# The Wayland app_id of the window is dev.uison.<program>: the .desktop file and the icon carry that name.
id=dev.uison.disk-usage

build() {
    if [ -z "${BIN:-}" ]; then
        (cd "$root" && cargo build --release -p disk-usage)
        BIN="$root/target/release/disk-usage"
    fi
}

case "${1:-}" in
macos)
    build
    out="$root/out/disk-usage"
    app="$out/Disk Usage.app"
    rm -rf "$app"
    mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
    cp "$BIN" "$app/Contents/MacOS/disk-usage"
    # An icon set from the 1024 px picture: every size at 1× and 2×.
    iconset="$(mktemp -d)/AppIcon.iconset"
    mkdir -p "$iconset"
    for size in 16 32 128 256 512; do
        sips -z $size $size "$here/assets/icon.png" --out "$iconset/icon_${size}x${size}.png" >/dev/null
        sips -z $((size * 2)) $((size * 2)) "$here/assets/icon.png" --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
    done
    iconutil -c icns "$iconset" -o "$app/Contents/Resources/AppIcon.icns"
    rm -rf "$(dirname "$iconset")"
    cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>Disk Usage</string>
    <key>CFBundleDisplayName</key><string>Disk Usage</string>
    <key>CFBundleIdentifier</key><string>$id</string>
    <key>CFBundleExecutable</key><string>disk-usage</string>
    <key>CFBundleIconFile</key><string>AppIcon</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleVersion</key><string>$version</string>
    <key>CFBundleShortVersionString</key><string>$version</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST
    # An ad-hoc signature: Apple Silicon runs it after copying (not notarized: right-click → Open the first time).
    codesign --force --sign - "$app" >/dev/null 2>&1 || true
    echo "$app"
    ;;
linux)
    build
    prefix="${2:-$HOME/.local}"
    install -Dm755 "$BIN" "$prefix/bin/disk-usage"
    install -Dm644 "$here/assets/icon-256.png" "$prefix/share/icons/hicolor/256x256/apps/$id.png"
    mkdir -p "$prefix/share/applications"
    # Exec with the full path: ~/.local/bin is not always in PATH. StartupWMClass is WM_CLASS on X11.
    cat > "$prefix/share/applications/$id.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Disk Usage
Comment=See where your disk space goes
Comment[ru]=Куда уходит место на диске
Exec=$prefix/bin/disk-usage %f
Icon=$id
Terminal=false
Categories=Utility;System;Filesystem;
Keywords=disk;space;usage;storage;sunburst;
StartupWMClass=disk-usage
DESKTOP
    command -v update-desktop-database >/dev/null && update-desktop-database -q "$prefix/share/applications" || true
    echo "$prefix/share/applications/$id.desktop"
    ;;
*)
    echo "usage: $0 macos | linux [prefix]" >&2
    exit 2
    ;;
esac
