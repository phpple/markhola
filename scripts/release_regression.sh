#!/bin/zsh

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
WITH_PACKAGE=0

for argument in "$@"; do
  case "$argument" in
    --with-package)
      WITH_PACKAGE=1
      ;;
    *)
      echo "Unknown argument: $argument" >&2
      exit 1
      ;;
  esac
done

require_file() {
  local path="$1"
  if [[ ! -f "$ROOT_DIR/$path" ]]; then
    echo "Missing required file: $path" >&2
    exit 1
  fi
}

run_packaging_with_retry() {
  local attempt
  for attempt in 1 2 3; do
    if "$ROOT_DIR/scripts/package_dmg.sh"; then
      return 0
    fi

    if [[ "$attempt" -eq 3 ]]; then
      echo "Release packaging failed after ${attempt} attempts." >&2
      exit 1
    fi

    echo "Retrying full packaging flow after transient failure (attempt ${attempt}/3)..." >&2
    sleep 2
  done
}

echo "==> Running automated regression tests"
cargo test --manifest-path "$ROOT_DIR/Cargo.toml"

echo "==> Building release binary"
cargo build --release --manifest-path "$ROOT_DIR/Cargo.toml"

echo "==> Verifying required regression fixtures"
require_file "examples/basic.md"
require_file "examples/languages.md"
require_file "examples/mermaid.md"
require_file "examples/math.md"
require_file "examples/multi-document.md"
require_file "examples/pdf-export.md"
require_file "assets/help/Documentation.md"
require_file "themes/default/layout.css"
require_file "scripts/release_regression_checklist.md"

echo "==> Running automated PDF export smoke test"
SMOKE_EXPORT_PATH="$ROOT_DIR/dist/pdf-export-smoke.pdf"
rm -f "$SMOKE_EXPORT_PATH"
cargo run --release --bin markhola --manifest-path "$ROOT_DIR/Cargo.toml" -- --smoke-export \
  "$ROOT_DIR/examples/basic.md" \
  "$SMOKE_EXPORT_PATH"
require_file "dist/pdf-export-smoke.pdf"
if [[ ! -s "$SMOKE_EXPORT_PATH" ]]; then
  echo "Smoke export produced an empty PDF file." >&2
  exit 1
fi

echo "==> Running Mermaid PDF export smoke test"
MERMAID_EXPORT_PATH="$ROOT_DIR/dist/mermaid-export-smoke.pdf"
rm -f "$MERMAID_EXPORT_PATH"
cargo run --release --bin markhola --manifest-path "$ROOT_DIR/Cargo.toml" -- --smoke-export \
  "$ROOT_DIR/examples/mermaid.md" \
  "$MERMAID_EXPORT_PATH"
require_file "dist/mermaid-export-smoke.pdf"
if [[ ! -s "$MERMAID_EXPORT_PATH" ]]; then
  echo "Mermaid smoke export produced an empty PDF file." >&2
  exit 1
fi

echo "==> Running HTML export smoke test"
HTML_EXPORT_PATH="$ROOT_DIR/dist/html-export-smoke.html"
rm -f "$HTML_EXPORT_PATH"
cargo run --release --bin markhola --manifest-path "$ROOT_DIR/Cargo.toml" -- --smoke-export-html \
  "$ROOT_DIR/examples/basic.md" \
  "$HTML_EXPORT_PATH"
require_file "dist/html-export-smoke.html"
if [[ ! -s "$HTML_EXPORT_PATH" ]]; then
  echo "HTML smoke export produced an empty file." >&2
  exit 1
fi

echo "==> Running print preparation smoke test"
cargo run --release --bin markhola --manifest-path "$ROOT_DIR/Cargo.toml" -- --smoke-print-prepare \
  "$ROOT_DIR/examples/basic.md"
cargo run --release --bin markhola --manifest-path "$ROOT_DIR/Cargo.toml" -- --smoke-print-prepare \
  "$ROOT_DIR/examples/mermaid.md"

echo "==> Verifying Mermaid print preview page count"
MERMAID_PRINT_PAGES_OUTPUT="$(cargo run --release --bin markhola --manifest-path "$ROOT_DIR/Cargo.toml" -- --smoke-print-pages \
  "$ROOT_DIR/examples/mermaid.md")"
echo "$MERMAID_PRINT_PAGES_OUTPUT"
if [[ "$MERMAID_PRINT_PAGES_OUTPUT" != *"pages=6"* ]]; then
  echo "Unexpected Mermaid print preview page count. Expected pages=6." >&2
  exit 1
fi

if [[ "$WITH_PACKAGE" -eq 1 ]]; then
  echo "==> Packaging app bundle and DMG"
  run_packaging_with_retry

  APP_VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT_DIR/Cargo.toml" | head -n1)"
  require_file "dist/MarkHola.app/Contents/Resources/themes/default/layout.css"
  require_file "dist/MarkHola.app/Contents/Resources/help/Documentation.md"
  require_file "dist/MarkHola-${APP_VERSION}.dmg"
fi

echo "==> Automated regression checks passed"
echo "==> Manual release checklist:"
echo "    $ROOT_DIR/scripts/release_regression_checklist.md"
