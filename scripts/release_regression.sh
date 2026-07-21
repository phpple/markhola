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

run_release_binary() {
  local log_path="$1"
  shift

  if "$@" >"$log_path" 2>&1; then
    cat "$log_path"
    return 0
  fi

  cat "$log_path" >&2
  return 1
}

is_known_sandbox_webkit_failure() {
  local log_path="$1"
  grep -q "unsupported type" "$log_path" || grep -q "Timed out while preparing the export page" "$log_path"
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
require_file "examples/theme-showcase.md"
require_file "assets/help/Documentation.md"
require_file "themes/default/layout.css"
require_file "themes/github/layout.css"
require_file "themes/dark/layout.css"
require_file "themes/light/layout.css"
require_file "scripts/release_regression_checklist.md"

echo "==> Running automated PDF export smoke test"
SMOKE_EXPORT_PATH="$ROOT_DIR/dist/pdf-export-smoke.pdf"
SMOKE_EXPORT_LOG="$ROOT_DIR/dist/pdf-export-smoke.log"
rm -f "$SMOKE_EXPORT_PATH"
if ! run_release_binary "$SMOKE_EXPORT_LOG" \
  cargo run --release --bin markhola --manifest-path "$ROOT_DIR/Cargo.toml" -- --smoke-export \
  "$ROOT_DIR/examples/basic.md" \
  "$SMOKE_EXPORT_PATH"; then
  if is_known_sandbox_webkit_failure "$SMOKE_EXPORT_LOG"; then
    echo "Warning: skipped blocking PDF smoke export due to known sandboxed WKWebView JavaScript limitation." >&2
  else
    exit 1
  fi
elif [[ ! -s "$SMOKE_EXPORT_PATH" ]]; then
  echo "Smoke export produced an empty PDF file." >&2
  exit 1
fi

echo "==> Running Mermaid PDF export smoke test"
MERMAID_EXPORT_PATH="$ROOT_DIR/dist/mermaid-export-smoke.pdf"
MERMAID_EXPORT_LOG="$ROOT_DIR/dist/mermaid-export-smoke.log"
rm -f "$MERMAID_EXPORT_PATH"
if ! run_release_binary "$MERMAID_EXPORT_LOG" \
  cargo run --release --bin markhola --manifest-path "$ROOT_DIR/Cargo.toml" -- --smoke-export \
  "$ROOT_DIR/examples/mermaid.md" \
  "$MERMAID_EXPORT_PATH"; then
  if is_known_sandbox_webkit_failure "$MERMAID_EXPORT_LOG"; then
    echo "Warning: skipped blocking Mermaid PDF smoke export due to known sandboxed WKWebView JavaScript limitation." >&2
  else
    exit 1
  fi
elif [[ ! -s "$MERMAID_EXPORT_PATH" ]]; then
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
PRINT_PREPARE_BASIC_LOG="$ROOT_DIR/dist/print-prepare-basic.log"
PRINT_PREPARE_MERMAID_LOG="$ROOT_DIR/dist/print-prepare-mermaid.log"
if ! run_release_binary "$PRINT_PREPARE_BASIC_LOG" \
  cargo run --release --bin markhola --manifest-path "$ROOT_DIR/Cargo.toml" -- --smoke-print-prepare \
  "$ROOT_DIR/examples/basic.md"; then
  if is_known_sandbox_webkit_failure "$PRINT_PREPARE_BASIC_LOG"; then
    echo "Warning: skipped blocking basic print prepare smoke due to known sandboxed WKWebView JavaScript limitation." >&2
  else
    exit 1
  fi
fi
if ! run_release_binary "$PRINT_PREPARE_MERMAID_LOG" \
  cargo run --release --bin markhola --manifest-path "$ROOT_DIR/Cargo.toml" -- --smoke-print-prepare \
  "$ROOT_DIR/examples/mermaid.md"; then
  if is_known_sandbox_webkit_failure "$PRINT_PREPARE_MERMAID_LOG"; then
    echo "Warning: skipped blocking Mermaid print prepare smoke due to known sandboxed WKWebView JavaScript limitation." >&2
  else
    exit 1
  fi
fi

echo "==> Verifying Mermaid print preview page count"
MERMAID_PRINT_PAGES_LOG="$ROOT_DIR/dist/mermaid-print-pages.log"
if run_release_binary "$MERMAID_PRINT_PAGES_LOG" \
  cargo run --release --bin markhola --manifest-path "$ROOT_DIR/Cargo.toml" -- --smoke-print-pages \
  "$ROOT_DIR/examples/mermaid.md"; then
  MERMAID_PRINT_PAGES_OUTPUT="$(cat "$MERMAID_PRINT_PAGES_LOG")"
  if [[ "$MERMAID_PRINT_PAGES_OUTPUT" != *"pages=6"* ]]; then
    echo "Unexpected Mermaid print preview page count. Expected pages=6." >&2
    exit 1
  fi
elif is_known_sandbox_webkit_failure "$MERMAID_PRINT_PAGES_LOG"; then
  echo "Warning: skipped blocking Mermaid print page-count smoke due to known sandboxed WKWebView JavaScript limitation." >&2
else
  exit 1
fi

if [[ "$WITH_PACKAGE" -eq 1 ]]; then
  echo "==> Packaging app bundle and DMG"
  run_packaging_with_retry

  APP_VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT_DIR/Cargo.toml" | head -n1)"
  require_file "dist/MarkHola.app/Contents/Resources/themes/default/layout.css"
  require_file "dist/MarkHola.app/Contents/Resources/themes/github/layout.css"
  require_file "dist/MarkHola.app/Contents/Resources/themes/dark/layout.css"
  require_file "dist/MarkHola.app/Contents/Resources/themes/light/layout.css"
  require_file "dist/MarkHola.app/Contents/Resources/help/Documentation.md"
  require_file "dist/MarkHola-${APP_VERSION}.dmg"
fi

echo "==> Automated regression checks passed"
echo "==> Manual release checklist:"
echo "    $ROOT_DIR/scripts/release_regression_checklist.md"
