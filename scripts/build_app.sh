#!/bin/zsh

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
APP_NAME="MarkHola"
DIST_DIR="$ROOT_DIR/dist"
APP_DIR="$DIST_DIR/$APP_NAME.app"
CONTENTS_DIR="$APP_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"
ICONSET_DIR="$DIST_DIR/$APP_NAME.icon-build"
ICNS_PATH="$RESOURCES_DIR/$APP_NAME.icns"

mkdir -p "$DIST_DIR"

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "Missing required command: $command_name" >&2
    exit 1
  fi
}

render_icon() {
  local size="$1"
  local output="$2"
  sips -z "$size" "$size" "$ROOT_DIR/assets/app-icon.png" --out "$output" >/dev/null
}

echo "==> Building Rust binary"
cargo build --release --manifest-path "$ROOT_DIR/Cargo.toml"

require_command sips

echo "==> Rendering macOS iconset"
rm -rf "$ICONSET_DIR"
mkdir -p "$ICONSET_DIR"

render_icon 16 "$ICONSET_DIR/icon_16x16.png"
render_icon 32 "$ICONSET_DIR/icon_32x32.png"
render_icon 48 "$ICONSET_DIR/icon_48x48.png"
render_icon 128 "$ICONSET_DIR/icon_128x128.png"
render_icon 256 "$ICONSET_DIR/icon_256x256.png"
render_icon 512 "$ICONSET_DIR/icon_512x512.png"
render_icon 1024 "$ICONSET_DIR/icon_1024x1024.png"

echo "==> Creating icns"
rm -rf "$APP_DIR"
mkdir -p "$MACOS_DIR" "$RESOURCES_DIR"
cargo run --manifest-path "$ROOT_DIR/Cargo.toml" --bin make_icns -- "$ICONSET_DIR" "$ICNS_PATH"

echo "==> Assembling app bundle"
cp "$ROOT_DIR/target/release/markhola" "$MACOS_DIR/$APP_NAME"
chmod +x "$MACOS_DIR/$APP_NAME"
ditto "$ROOT_DIR/themes" "$RESOURCES_DIR/themes"
ditto "$ROOT_DIR/assets/help" "$RESOURCES_DIR/help"
cp "$ROOT_DIR/assets/logo.png" "$RESOURCES_DIR/logo.png"

cat > "$CONTENTS_DIR/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
  <dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleDisplayName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleExecutable</key>
    <string>${APP_NAME}</string>
    <key>CFBundleIconFile</key>
    <string>${APP_NAME}</string>
    <key>CFBundleIdentifier</key>
    <string>com.markhola.app</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>${APP_NAME}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>NSPrincipalClass</key>
    <string>NSApplication</string>
    <key>UTImportedTypeDeclarations</key>
    <array>
      <dict>
        <key>UTTypeIdentifier</key>
        <string>net.daringfireball.markdown</string>
        <key>UTTypeDescription</key>
        <string>Markdown document</string>
        <key>UTTypeConformsTo</key>
        <array>
          <string>public.plain-text</string>
          <string>public.text</string>
          <string>public.data</string>
        </array>
        <key>UTTypeTagSpecification</key>
        <dict>
          <key>public.filename-extension</key>
          <array>
            <string>md</string>
            <string>markdown</string>
          </array>
          <key>public.mime-type</key>
          <array>
            <string>text/markdown</string>
            <string>text/x-markdown</string>
          </array>
        </dict>
      </dict>
    </array>
    <key>CFBundleDocumentTypes</key>
    <array>
      <dict>
        <key>CFBundleTypeName</key>
        <string>Markdown Document</string>
        <key>CFBundleTypeRole</key>
        <string>Editor</string>
        <key>LSHandlerRank</key>
        <string>Owner</string>
        <key>CFBundleTypeExtensions</key>
        <array>
          <string>md</string>
          <string>markdown</string>
        </array>
        <key>CFBundleTypeMIMETypes</key>
        <array>
          <string>text/markdown</string>
          <string>text/x-markdown</string>
        </array>
        <key>LSItemContentTypes</key>
        <array>
          <string>net.daringfireball.markdown</string>
        </array>
      </dict>
    </array>
    <key>NSDocumentsFolderUsageDescription</key>
    <string>MarkHola needs access to your Documents folder to open Markdown files and load referenced local assets (images, diagrams) located alongside your documents.</string>
    <key>NSDesktopFolderUsageDescription</key>
    <string>MarkHola needs access to your Desktop folder to open Markdown files and load referenced local assets (images, diagrams) located alongside your documents.</string>
    <key>NSDownloadsFolderUsageDescription</key>
    <string>MarkHola needs access to your Downloads folder to open Markdown files and load referenced local assets (images, diagrams) located alongside your documents.</string>
    <key>CFBundleShortVersionString</key>
    <string>$(sed -n 's/^version = \"\\(.*\\)\"/\\1/p' "$ROOT_DIR/Cargo.toml" | head -n1)</string>
    <key>CFBundleVersion</key>
    <string>$(sed -n 's/^version = \"\\(.*\\)\"/\\1/p' "$ROOT_DIR/Cargo.toml" | head -n1)</string>
  </dict>
</plist>
PLIST

echo "==> Done: $APP_DIR"
