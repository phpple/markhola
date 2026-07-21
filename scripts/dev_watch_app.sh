#!/bin/zsh

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
WATCH_INTERVAL_SECONDS="${WATCH_INTERVAL_SECONDS:-1}"

snapshot() {
  (
    cd "$ROOT_DIR"
    find src themes assets -type f \
      ! -path "*/target/*" \
      ! -path "*/dist/*" \
      -print0 \
      | xargs -0 stat -f "%N %m" 2>/dev/null \
      | sort
  ) | shasum | awk '{print $1}'
}

echo "==> MarkHola dev watch: rebuilding dist/MarkHola.app on changes"
echo "==> Interval: ${WATCH_INTERVAL_SECONDS}s  (set WATCH_INTERVAL_SECONDS to change)"

last="$(snapshot || true)"

while true; do
  sleep "$WATCH_INTERVAL_SECONDS"
  next="$(snapshot || true)"
  if [[ "$next" != "$last" ]]; then
    last="$next"
    echo "==> Change detected at $(date '+%H:%M:%S')"
    "$ROOT_DIR/scripts/build_app.sh" || true
  fi
done

