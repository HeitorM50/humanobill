#!/usr/bin/env bash
# Gera fixtures/bills/<id>.pdf e <id>.png a partir de fixtures/templates/*.html.
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHROME="${CHROMIUM:-chromium}"
mkdir -p "$DIR/bills"
for tpl in "$DIR"/templates/*.html; do
  id="$(basename "$tpl" .html)"
  "$CHROME" --headless --no-sandbox --disable-gpu --no-pdf-header-footer \
    --print-to-pdf="$DIR/bills/$id.pdf" "file://$tpl" 2>/dev/null
  "$CHROME" --headless --no-sandbox --disable-gpu --hide-scrollbars \
    --screenshot="$DIR/bills/$id.png" --window-size=900,1300 "file://$tpl" 2>/dev/null
  echo "ok: $id"
done
